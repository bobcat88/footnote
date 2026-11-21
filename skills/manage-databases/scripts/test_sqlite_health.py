import os
import sqlite3
import tempfile
import unittest

from sqlite_health import health


class SqliteHealthTest(unittest.TestCase):
    def setUp(self):
        fd, self.path = tempfile.mkstemp(suffix=".db")
        os.close(fd)
        con = sqlite3.connect(self.path)
        con.executescript("""
            PRAGMA journal_mode=wal;
            CREATE TABLE author(id INTEGER PRIMARY KEY, name TEXT);
            CREATE TABLE book(id INTEGER PRIMARY KEY, author_id INTEGER REFERENCES author(id), title TEXT);
            CREATE INDEX book_author ON book(author_id);
            INSERT INTO author VALUES (1, 'a');
            INSERT INTO book VALUES (1, 1, 't'), (2, 99, 'orphan');
        """)
        con.close()

    def tearDown(self):
        for suffix in ("", "-wal", "-shm"):
            if os.path.exists(self.path + suffix):
                os.remove(self.path + suffix)

    def test_reads_without_changing_the_file(self):
        before = os.path.getmtime(self.path)
        r = health(self.path)
        self.assertEqual(r["journal_mode"], "wal")
        self.assertEqual(r["tables"]["book"]["rows"], 2)
        self.assertEqual(os.path.getmtime(self.path), before)

    def test_foreign_key_violation_fails(self):
        r = health(self.path)
        self.assertTrue(any(f["severity"] == "FAIL" and "foreign-key" in f["msg"] for f in r["findings"]))

    def test_plans_flag_scans_but_not_index_use(self):
        r = health(self.path, ["SELECT * FROM book WHERE title = 't'", "SELECT * FROM book WHERE author_id = 1"])
        scans = [f for f in r["findings"] if "full scan" in f["msg"]]
        self.assertEqual(len(scans), 1)
        self.assertIn("title", scans[0]["msg"])

    def test_bad_query_is_reported(self):
        r = health(self.path, ["SELECT nope FROM missing"])
        self.assertTrue(any("failed to plan" in f["msg"] for f in r["findings"]))


if __name__ == "__main__":
    unittest.main()
