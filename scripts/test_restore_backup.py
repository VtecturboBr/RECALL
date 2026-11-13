"""Tests for SQLite-safe offline RECALL restore, no extra dependencies."""
import sqlite3
import tempfile
from contextlib import closing
import unittest
from pathlib import Path

from restore_backup import RestoreError, recover_interrupted_restore, restore_database, verify_database


def make_database(path: Path, text: str) -> None:
    with closing(sqlite3.connect(path)) as db:
        with db:
            db.execute("CREATE TABLE notes(
                id TEXT PRIMARY KEY, title TEXT NOT NULL, body TEXT NOT NULL DEFAULT '',
                kind TEXT NOT NULL DEFAULT 'note',created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,deleted_at INTEGER
            )")
            db.execute("PRAGMA user_version=1")
            db.execute("INSERT INTO notes(id,title) VALUES('n1',?)", (text,))


def title(path: Path) -> str:
    with closing(sqlite3.connect(path)) as db:
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

    def test_recovers_orphaned_original_without_creating_an_empty_db(self):
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            target = folder / "memoria.sqlite"
            stranded = folder / ".recall-original-test.sqlite"
            make_database(stranded, "Preservado na queda")
            self.assertFalse(target.exists())
            self.assertTrue(recover_interrupted_restore(target))
            self.assertEqual(title(target), "Preservado na queda")
            self.assertFalse(stranded.exists())

    def test_recovery_rejects_multiple_stranded_candidates(self):
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            target = folder / "memoria.sqlite"
            make_database(folder / ".recall-original-one.sqlite", "Um")
            make_database(folder / ".recall-original-two.sqlite", "Dois")
            with self.assertRaises(RestoreError):
                recover_interrupted_restore(target)
            self.assertFalse(target.exists())

    def test_rejects_incompatible_future_schema_before_touching_target(self):
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            target = folder / "memoria.sqlite"
            backup = folder / "recall-bad.sqlite"
            make_database(target, "Atual")
            make_database(backup, "Futuro")
            with closing(sqlite3.connect(backup)) as db:
                db.execute("PRAGMA user_version=99")
            with self.assertRaises(RestoreError):
                restore_database(backup, target)
            self.assertEqual(title(target), "Atual")

    def test_rejects_backup_with_missing_required_note_columns(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "bad.sqlite"
            with closing(sqlite3.connect(path)) as db:
                db.execute("CREATE TABLE notes(id TEXT PRIMARY KEY,title TEXT)")
            with self.assertRaises(RestoreError):
                verify_database(path)

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
