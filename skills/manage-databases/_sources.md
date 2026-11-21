---
skill_hash: ac79c7bc60f38a58
reviewed: 2026-11-21
---
# manage-databases: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| EXPLAIN QUERY PLAN output | [SQLite EQP](https://www.sqlite.org/eqp.html) |
| WAL: one writer, no network FS | [SQLite WAL](https://www.sqlite.org/wal.html) |
| bm25 lower is better | [SQLite FTS5](https://www.sqlite.org/fts5.html#the_bm25_function) |
| VACUUM INTO, backup API | [VACUUM](https://www.sqlite.org/lang_vacuum.html#vacuuminto) · [Online backup API](https://www.sqlite.org/backup.html) |
| ALTER TABLE limits, 12-step rebuild | [SQLite ALTER TABLE](https://www.sqlite.org/lang_altertable.html#otheralter) |
| EXPLAIN ANALYZE executes the statement | [Postgres EXPLAIN](https://www.postgresql.org/docs/current/sql-explain.html) |
| pg_stat_statements | [Postgres docs](https://www.postgresql.org/docs/current/pgstatstatements.html) |
| RLS enable / force | [Postgres row security](https://www.postgresql.org/docs/current/ddl-rowsecurity.html) |
| CREATE INDEX CONCURRENTLY | [Postgres CREATE INDEX](https://www.postgresql.org/docs/current/sql-createindex.html#SQL-CREATEINDEX-CONCURRENTLY) |
| NOT VALID + VALIDATE | [Postgres ALTER TABLE](https://www.postgresql.org/docs/current/sql-altertable.html) |
| SKIP LOCKED | [Postgres SELECT locking](https://www.postgresql.org/docs/current/sql-select.html#SQL-FOR-UPDATE-SHARE) |
| Pooler modes, what transaction mode breaks | [PgBouncer features](https://www.pgbouncer.org/features.html) |
| Backups, PITR | [Postgres backup and restore](https://www.postgresql.org/docs/current/backup.html) |
| Keyset pagination | [use-the-index-luke: paging](https://use-the-index-luke.com/no-offset) |
| DuckDB on Parquet | [DuckDB Parquet](https://duckdb.org/docs/stable/data/parquet/overview) |
| Polars lazy | [Polars lazy API](https://docs.pola.rs/user-guide/lazy/using/) |

## Distilled from
High-install public database skills (Postgres best practices, a serverless Postgres skill, an ORM setup skill), surveyed 2026-11-21; rules rewritten and cross-checked against the primary docs above. Licences MIT and Apache-2.0; credited in NOTICE.
