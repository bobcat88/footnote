---
name: manage-databases
description: Use when designing a schema, writing or tuning queries, adding indexes, writing migrations, setting up backups, or choosing a store, for SQLite, Postgres or analytics files (DuckDB, Parquet, Polars): EXPLAIN plans, N+1 and pagination, WAL and locking, connection pooling, row-level security, safe migrations and tested restores. Runs sqlite-health for a read-only SQLite checkup.
license: MIT
---

# Manage databases

Work on a copy first. Never `VACUUM`, `REINDEX`, change journal mode or run migrations on a live database to "diagnose".

## Run
- `python <skills>/manage-databases/scripts/sqlite_health.py <db> [--query "SELECT …"]… [--json]` → read-only: version / size / WAL / free pages / `quick_check` / foreign keys / rows + indexes per table / query plans (flags full scans). Exit 1 on integrity or FK failure. Test: `python -m unittest test_sqlite_health` (in `scripts/`).
- Postgres: `EXPLAIN (ANALYZE, BUFFERS)` (it executes: never on writes outside a rolled-back transaction) / `pg_stat_statements` sorted by total time / `pg_stat_activity` for connections.

## Pick the store
- App state, single machine, embedded → SQLite (WAL)
- Many writers, network clients, roles, RLS → Postgres
- Analytics over files → DuckDB SQL on Parquet / Polars lazy frames in code; `to_pandas()` only at a legacy boundary

## Schema
- Primary key on every table / `NOT NULL` + `CHECK` + FKs in the database, not only in the app
- Index the referencing side of every FK / lowercase snake_case identifiers / right types (timestamps with time zone, numeric for money)
- Tenant isolation in the database: Postgres RLS `enable` + `force` row level security; filtering in the app isn't isolation

## Queries
- `EXPLAIN` before indexing / index WHERE + JOIN + ORDER BY columns / composite order = equality first, then range
- A scan is fine on small tables or broad reads / unused-index reports are candidates: check the observation window before dropping
- N+1 → one join or `IN` batch / keyset pagination (`WHERE id > ?`) over deep `OFFSET` / select only needed columns / batch inserts in one transaction / upsert with `ON CONFLICT`
- Parameters always, never string-built SQL

## Concurrency
- SQLite: WAL = readers during one writer / short write transactions / `busy_timeout` / not on network filesystems / FTS5 `bm25()` lower = better
- Postgres: short transactions / consistent lock order / `SKIP LOCKED` for queues / pooler (PgBouncer) for many clients; transaction mode breaks prepared statements, `SET` and temp tables → migrations and `pg_dump` use a direct connection

## Migrations
- One tool owns the schema / versioned files in the repo / no hand edits on prod
- Test on a copy with production-like data / time the lock it takes
- Postgres: `CREATE INDEX CONCURRENTLY` / add nullable column → backfill in batches → add constraint `NOT VALID` → `VALIDATE` / expand → migrate → contract for renames
- SQLite: `ALTER TABLE` is limited → create-copy-swap in one transaction with `foreign_keys` off, then `foreign_key_check`

## Backups
- SQLite: `VACUUM INTO 'backup.db'` or the online backup API, never a raw file copy of a live WAL database
- Postgres: `pg_dump -Fc` for logical / base backup + WAL archive for point-in-time
- 3 copies / 2 media / 1 off-site / a restore actually tested and timed, with the date recorded
