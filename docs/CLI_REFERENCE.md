# HyperKB CLI Reference

HyperKB provides a command-line interface for human developers, pre-commit hooks, and CI/CD pipelines.

---

## 💻 Commands

### Repository Lifecycle
* **`hyperkb init [--name <name>] [--collection <id>]`**  
  Initializes a new `hyperkb.json` repository manifest and standard governance directories (`docs/decisions`, `docs/directives`, `docs/risks`).
* **`hyperkb index [--dir <path>]`**  
  Scans and indexes Markdown documents and frontmatter metadata into the local SQLite FTS5 database.

### Knowledge & Search
* **`hyperkb search <query> [--limit <n>] [--include-private]`**  
  Executes BM25 full-text search across all indexed documents, specs, and decisions.
* **`hyperkb browse [--category <cat>] [--project <proj>] [--topic <topic>]`**  
  Lists indexed documents filtered by category (`decisions`, `risks`, `specs`, `plans`) or project taxonomy.

### Risk Management & Pre-Commit Interception
* **`hyperkb check-work [files...] [--staged] [--changed]`**  
  Validates specified or git-staged files against active risk cards and directives. Exits with non-zero status code if unmitigated hazard paths are detected.
* **`hyperkb install-hook`**  
  Installs a native pre-commit hook into `.git/hooks/pre-commit` to prevent committing changes to hazard paths without review.
* **`hyperkb draft-risk -t <title> -r <rationale> -p <paths...>`**  
  Drafts a new architectural risk citing affected file path globs.
* **`hyperkb ack <risk-id> -r <rationale>`**  
  Acknowledges an open risk with justification so it no longer blocks `check-work`.

### Standing Policy Directives (Rule of 5)
* **`hyperkb directive list`**  
  Lists all active and retired repository invariants.
* **`hyperkb directive draft -t <title> -c <category> -s <scopes...>`**  
  Drafts a new standing policy directive.
* **`hyperkb directive retire <directive-id>`**  
  Retires an invariant when system architecture evolves.

### Authority Grants & Delegation
* **`hyperkb grant list`**  
  Lists active and expired agent authority grants.
* **`hyperkb grant issue <grantee> --scopes <globs...> --actions <actions...>`**  
  Issues a scoped, time-bounded permission grant for autonomous agent changes.
* **`hyperkb grant revoke <grant-id>`**  
  Immediately invalidates an active authority grant.

### Telemetry & Auditing
* **`hyperkb brief [--scope <pattern>]`**  
  Outputs a warm-start context briefing containing active invariants, unmitigated risks, and git churn hotspots.
* **`hyperkb metrics [--session-id <id>] [--json]`**  
  Displays session quality scores, review loop penalties, and tool thrashing counts.
* **`hyperkb audit [--kb] [--directives]`**  
  Runs anti-bloat, taxonomy consistency, and documentation decay checks.

### Background Services & Maintenance
* **`hyperkb mcp`**  
  Runs the headless Model Context Protocol (MCP) server over stdin/stdout.
* **`hyperkb backup [--keep <n>]`**  
  Creates a verified point-in-time SQLite snapshot backup.
* **`hyperkb compact`**  
  Runs `VACUUM` and truncates SQLite WAL journals to reclaim disk space.
