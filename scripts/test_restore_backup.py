"""Tests for SQLite-safe offline RECALL restore, no extra dependencies."""
import sqlite3
import tempfile
import unittest
from pathlib import Path

from restore_backup import RestoreError, restore_database, verify_database


def make_database(path: Path, text: str) -> None:
    with sqlite3.connect(path) as db:
        db.execute("CREATE TABLE notes(id TEXT PRIMARY KEY, title TEXT NOT NULL)")
        db.execute("INSERT INTO notes(id,title) VALUES('n1',?)", (text,))


def title(path: Path) -> str:
    with sqlite3.connect(path) as db:
        return db.execute("SELECT title FROM notes WHERE id='n1'").fetchone()[0]


class OfflineRestoreTests(unittest.TestCase):
    def test_restores_and_retains_old_database_backup(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            db = base / "memoria.sqlite"
            archive = base / "backups"
            archive.mkdir()
            target_backup = archive / "recall-manual-test.sqlite"
            make_database(db, "Versão atual")
            make_database(target_backup, "Versão de backup")
            self.assertEqual(verify_database(db), {"notes": 1})
            safety_path = restore_database(target_backup, db)
            self.assertEqual(title(db), "Versão de backup")
            self.assertEqual(title(safety_path), "Versão atual")
            self.assertEqual(title(target_backup), "Versão de backup")

    def test_rejects_invalid_backup_without_touching_current_data(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            db = base / "memoria.sqlite"
            invalid = base / "bad.sqlite"
            make_database(db, "Manter")
            invalid.write_bytes(b"not a sqlite db")
            with self.assertRaises(RestoreError):
                restore_database(invalid, db)
            self.assertEqual(title(db), "Manter")

    def test_refuses_to_replace_if_wal_indicator_exists(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            db = base / "memoria.sqlite"
            backup = base / "backups" / "recall-test.sqlite"
            backup.parent.mkdir()
            make_database(db, "Atual")
            make_database(backup, "Antigo")
            Path(str(db) + "-wal").write_bytes(b"uncheckpointed state")
            with self.assertRaises(RestoreError):
                restore_database(backup, db)
            self.assertEqual(title(db), "Atual")


if __name__ == "__main__":
    unittest.main()
