"""Transactional project state and audit events, separate from runtime budgets."""

import json
import sqlite3
from contextlib import contextmanager
from datetime import datetime, timezone
from pathlib import Path

from .contracts import encoded


APPLICATION_ID = 0x464E4631


class ProjectStore:
    def __init__(self, database, workspace, skills_root):
        self.workspace = Path(workspace).resolve()
        self.skills_root = Path(skills_root).resolve()
        self.database = Path(database).resolve()
        if not self.workspace.is_dir():
            raise ValueError("workspace must be an existing directory")
        if self.database.is_relative_to(self.workspace):
            raise ValueError("authoritative state must be outside worker workspace")
        existed = self.database.exists()
        self.database.parent.mkdir(parents=True, exist_ok=True)
        self.connection = sqlite3.connect(self.database, timeout=10, isolation_level=None)
        self.connection.row_factory = sqlite3.Row
        try:
            self.connection.execute("PRAGMA foreign_keys=ON")
            with self.write():
                version = self.connection.execute("PRAGMA user_version").fetchone()[0]
                app = self.connection.execute("PRAGMA application_id").fetchone()[0]
                if version == 0 and not existed:
                    statements = [
                        "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
                        "CREATE TABLE tasks (id TEXT PRIMARY KEY, body TEXT NOT NULL, state TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0)",
                        "CREATE TABLE attempts (id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), actor TEXT NOT NULL, mode TEXT NOT NULL, brief TEXT NOT NULL, state TEXT NOT NULL, activation TEXT, report TEXT, evidence TEXT)",
                        "CREATE TABLE events (id INTEGER PRIMARY KEY AUTOINCREMENT, at TEXT NOT NULL, task_id TEXT, kind TEXT NOT NULL, payload TEXT NOT NULL)",
                        "CREATE TABLE memories (id TEXT PRIMARY KEY, attempt_id TEXT NOT NULL REFERENCES attempts(id), candidate_index INTEGER NOT NULL, kind TEXT NOT NULL, body TEXT NOT NULL, evidence TEXT NOT NULL, actor TEXT NOT NULL, rationale TEXT NOT NULL, supersedes TEXT REFERENCES memories(id), active INTEGER NOT NULL, UNIQUE(attempt_id,candidate_index))",
                    ]
                    for statement in statements:
                        self.connection.execute(statement)
                    self.connection.execute(f"PRAGMA application_id={APPLICATION_ID}")
                    self.connection.execute("PRAGMA user_version=1")
                    for key, value in (("workspace", str(self.workspace)), ("skills_root", str(self.skills_root))):
                        self.connection.execute("INSERT INTO meta VALUES (?,?)", (key, encoded(value)))
                    version, app = 1, APPLICATION_ID
                elif version not in (1, 2) or app != APPLICATION_ID:
                    raise ValueError("unknown database identity/schema; refusing initialization")
                for key, expected in (("workspace", str(self.workspace)), ("skills_root", str(self.skills_root))):
                    if self.meta(key) != expected:
                        raise ValueError(f"stored {key} differs; explicit migration required")
                if self.connection.execute("PRAGMA quick_check").fetchone()[0] != "ok":
                    raise ValueError("database integrity check failed")
                if version == 1:
                    self.connection.execute("CREATE TABLE verifications (id TEXT PRIMARY KEY, attempt_id TEXT NOT NULL UNIQUE REFERENCES attempts(id), state TEXT NOT NULL, request TEXT NOT NULL, result TEXT)")
                    self.connection.execute("PRAGMA user_version=2")
        except BaseException:
            self.connection.close()
            raise

    @contextmanager
    def write(self):
        self.connection.execute("BEGIN IMMEDIATE")
        try:
            yield self.connection
        except BaseException:
            self.connection.rollback()
            raise
        else:
            self.connection.commit()

    def meta(self, key):
        row = self.connection.execute("SELECT value FROM meta WHERE key=?", (key,)).fetchone()
        return json.loads(row[0]) if row else None

    @contextmanager
    def read(self):
        self.connection.execute("BEGIN")
        try:
            yield self.connection
        except BaseException:
            self.connection.rollback()
            raise
        else:
            self.connection.commit()

    def event(self, task_id, kind, payload):
        self.connection.execute("INSERT INTO events(at,task_id,kind,payload) VALUES (?,?,?,?)", (datetime.now(timezone.utc).isoformat(), task_id, kind, encoded(payload)))

    def close(self):
        self.connection.close()
