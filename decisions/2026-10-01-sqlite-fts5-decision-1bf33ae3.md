---hyperkb
{
  "id": "DEC-20261001-0001",
  "kind": "decision",
  "title": "Use Statically Bundled SQLite and FTS5 for Knowledge Storage",
  "status": "accepted",
  "owner": "Developer",
  "author": "Architect",
  "supersedes": null,
  "created_at": "2026-10-01T12:00:00.000000+00:00"
}
---
# Use Statically Bundled SQLite and FTS5 for Knowledge Storage

## Context & Problem Statement
HyperKB requires fast (<5ms) full-text search, transactional record integrity, and sub-millisecond document retrieval without imposing external daemon requirements (like PostgreSQL, Redis, or Elasticsearch) or dynamic C library dependencies on developer machines.

## Decision
We statically compile SQLite 3 with the FTS5 full-text search extension directly into the native Rust binary via `rusqlite` bundled mode.

## Consequences
- **Positive**: Zero external runtime dependencies. Instant startup (<10ms).
- **Positive**: Full-text BM25 ranked search across markdown documents, specs, and decisions.
- **Negative**: Database write concurrency is managed via WAL mode, which is optimal for single-user developer workflows but not designed as a distributed multi-tenant service.
