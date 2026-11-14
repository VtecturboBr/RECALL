#!/usr/bin/env python3
"""Offline RECALL SQLite restore, with verification and a rollback snapshot.

Requires only Python 3.10+ (standard library). RECALL must be fully closed
from the system tray. Do not replace a SQLite database while it is in use.
"""
from __future__ import annotations

import argparse
import csv
from contextlib import closing
import os
import sqlite3
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from uuid import uuid4


class RestoreError(RuntimeError):
    pass


def ensure_not_running() -> None:
    if sys.platform != "win32":
        return
    try:
        result = subprocess.run(
            ["tasklist", "/FO", "CSV", "/NH"],
            capture_output=True, text=True, errors="replace", check=True
        )
        running = {
            row[0].lower() for row in csv.reader(result.stdout.splitlines()) if row
        }
    except (OSError, subprocess.CalledProcessError) as exc:
        raise RestoreError(
            f"Não foi possível verificar se o RECALL está fechado: {exc}"
        ) from exc
    if "recall.exe" in running or "memoria_externa.exe" in running:
        raise RestoreError(
            "Feche o RECALL usando Sair do aplicativo no ícone da bandeja antes de restaurar."
        )


def verify_database(path: Path) -> dict[str, int]:
    if not path.is_file():
        raise RestoreError(f"Arquivo SQLite não encontrado: {path}")
    try:
        with closing(sqlite3.connect(path.resolve().as_uri() + "?mode=ro", uri=True)) as connection:
            result = connection.execute("PRAGMA quick_check").fetchone()
            if result is None or result[0] != "ok":
                raise RestoreError(f"Falha na integridade do SQLite: {result}")
            version = int(connection.execute("PRAGMA user_version").fetchone()[0])
            if version > 7 or version < 0:
                raise RestoreError(f"Versão SQLite do RECALL incompatível: v{version}.")
            fk_errors = connection.execute("PRAGMA foreign_key_check").fetchone()
            if fk_errors:
                raise RestoreError(f"Integridade referencial comprometida: {fk_errors}")
            tables = {row[0] for row in connection.execute(
                "SELECT name FROM sqlite_master WHERE type='table'"
            )}
            if "notes" not in tables:
                raise RestoreError("Este arquivo não contém a tabela notes do RECALL.")
            columns = {row[1] for row in connection.execute("PRAGMA table_info(notes)")}
            minimum = {"id","title","body","kind","pinned","remind_at",
                       "notified_at","created_at","updated_at","deleted_at"}
            if not minimum.issubset(columns):
                raise RestoreError("Tabela notes incompleta ou de outra aplicação.")
            if version >= 5 and not {"tasks","inbox_items","content_history","inbox_dispositions"}.issubset(tables):
                raise RestoreError("Banco v5+ sem tabelas essenciais de histórico.")
            if version >= 6 and "focus_sessions" not in tables:
                raise RestoreError("Banco v6+ sem sessões de foco.")
            if version >= 7 and "item_links" not in tables:
                raise RestoreError("Banco v7 sem relacionamentos.")
            counts: dict[str, int] = {}
            for name in ("notes", "tasks", "inbox_items"):
                if name in tables:
                    counts[name] = int(connection.execute(
                        f"SELECT COUNT(*) FROM {name}"
                    ).fetchone()[0])
            return counts
    except sqlite3.DatabaseError as exc:
        raise RestoreError(f"Banco inválido: {exc}") from exc


def snapshot(source: Path, destination: Path) -> None:
    """SQLite's online backup API incorporates committed WAL frames."""
    if destination.exists():
        raise RestoreError(f"Arquivo de destino já existe: {destination}")
    try:
        with closing(sqlite3.connect(source.resolve().as_uri() + "?mode=ro", uri=True)) as source_db:
            with closing(sqlite3.connect(str(destination))) as dest_db:
                source_db.backup(dest_db)
        verify_database(destination)
    except Exception:
        destination.unlink(missing_ok=True)
        raise


def restore_database(backup: Path, target: Path) -> Path:
    """Restore a consistent snapshot only while the application is stopped.

    Returns the immutable pre-restore backup. Never deletes user backup files.
    """
    backup = backup.resolve()
    target = target.resolve()
    if backup == target:
        raise RestoreError("O backup e o banco ativo não podem ser o mesmo arquivo.")
    ensure_not_running()
    verify_database(backup)
    verify_database(target)

    # Never discard potentially committed data in live or crash-left WAL files.
    if Path(str(target) + "-wal").exists() or Path(str(target) + "-shm").exists():
        raise RestoreError(
            "Arquivos SQLite WAL/SHM ainda existem. Feche totalmente o RECALL e "
            "confirme que não há outro processo usando o banco; não apague esses arquivos manualmente."
        )

    folder = target.parent / "backups"
    folder.mkdir(parents=True, exist_ok=True)
    suffix = datetime.now().strftime("%Y%m%d-%H%M%S") + "-" + uuid4().hex[:10]
    fallback = folder / f"recall-pre-restore-{suffix}.sqlite"
    staging = target.parent / f".recall-restore-{suffix}.partial"
    snapshot(target, fallback)
    try:
        snapshot(backup, staging)
        # A single replace avoids the dangerous interval with no memoria.sqlite.
        # The previous database has already been safely captured in fallback.
        with staging.open("r+b") as staged_file:
            os.fsync(staged_file.fileno())
        os.replace(staging, target)
        verify_database(target)
        return fallback
    finally:
        staging.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Restaure um backup do RECALL sem sobrescrever dados inesperadamente."
    )
    parser.add_argument("backup", type=Path, help="Caminho do arquivo recall-*.sqlite")
    parser.add_argument(
        "--db", type=Path, default=None,
        help="Caminho do memoria.sqlite ativo (padrão: pasta acima de backups)"
    )
    parser.add_argument("--confirm", action="store_true", help="Executar restauração após prévia")
    args = parser.parse_args()
    backup = args.backup.expanduser().resolve()
    target = (args.db or (backup.parent.parent / "memoria.sqlite")).expanduser().resolve()
    try:
        original = verify_database(target)
        future = verify_database(backup)
        print(f"Banco atual: {target}\nContagens: {original}")
        print(f"Backup escolhido: {backup}\nContagens: {future}")
        if not args.confirm:
            print("Prévia apenas. Feche o RECALL pela bandeja e acrescente --confirm para restaurar.")
            return 0
        fallback = restore_database(backup, target)
        print("Restauração concluída e verificada.")
        print(f"Cópia de segurança do estado anterior: {fallback}")
        return 0
    except (OSError, sqlite3.DatabaseError, RestoreError) as exc:
        print(f"Restauração cancelada com segurança: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
