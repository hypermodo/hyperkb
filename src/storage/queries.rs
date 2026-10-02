use crate::domain::{
    AgentSession, BrowseOptions, Directive, Document, DocumentKind, DocumentStatus, Hit, Memory,
    RecordMeta, SessionEventRecord,
};
use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub struct Queries;

impl Queries {
    pub fn tokenize_query(query: &str) -> Vec<String> {
        query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(|w| w.to_lowercase())
            .collect()
    }

    pub fn upsert_document(
        conn: &Connection,
        source_id: &str,
        collection_id: &str,
        path: &str,
        topic: &str,
        title: &str,
        content: &str,
        search_text: &str,
        meta: &RecordMeta,
        checksum: &str,
        is_tombstone: bool,
    ) -> Result<()> {
        let risk_paths = serde_json::to_string(&meta.paths).unwrap_or_else(|_| "[]".into());
        let risk_versions = serde_json::to_string(&meta.versions).unwrap_or_else(|_| "[]".into());
        let risk_environments =
            serde_json::to_string(&meta.environments).unwrap_or_else(|_| "[]".into());
        let supersedes = meta.supersedes.as_deref().unwrap_or("");
        let issue = meta.issue.as_deref().unwrap_or("");
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR IGNORE INTO collections (id) VALUES (?1);",
            params![collection_id],
        )?;

        conn.execute(
            "DELETE FROM documents WHERE collection_id = ?1 AND path = ?2 AND source_id != ?3;",
            params![collection_id, path, source_id],
        )?;

        conn.execute(
            "INSERT INTO documents (
                source_id, collection_id, path, topic, title, content, search_text,
                status, kind, owner, issue, risk_paths, risk_versions, risk_environments,
                supersedes, checksum, indexed_at, is_tombstone
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
             ON CONFLICT(source_id) DO UPDATE SET
                collection_id=excluded.collection_id,
                path=excluded.path,
                topic=excluded.topic,
                title=excluded.title,
                content=excluded.content,
                search_text=excluded.search_text,
                status=excluded.status,
                kind=excluded.kind,
                owner=excluded.owner,
                issue=excluded.issue,
                risk_paths=excluded.risk_paths,
                risk_versions=excluded.risk_versions,
                risk_environments=excluded.risk_environments,
                supersedes=excluded.supersedes,
                checksum=excluded.checksum,
                indexed_at=excluded.indexed_at,
                is_tombstone=excluded.is_tombstone;",
            params![
                source_id,
                collection_id,
                path,
                topic,
                title,
                content,
                search_text,
                meta.status,
                meta.kind,
                meta.owner,
                issue,
                risk_paths,
                risk_versions,
                risk_environments,
                supersedes,
                checksum,
                now,
                if is_tombstone { 1 } else { 0 }
            ],
        )?;
        Ok(())
    }

    pub fn get_document(conn: &Connection, source_id: &str) -> Result<Option<Document>> {
        let mut stmt = conn.prepare(
            "SELECT d.source_id, d.collection_id, d.path, d.title, d.topic,
                    e.effective_status, d.kind, d.owner, d.issue,
                    e.replacement_id, d.supersedes, d.content, d.checksum,
                    d.is_tombstone
             FROM effective_documents e
             JOIN documents d ON d.id = e.id
             WHERE d.source_id = ?1 OR d.path = ?1;",
        )?;

        let mut rows = stmt.query([source_id])?;
        if let Some(row) = rows.next()? {
            let status_str: String = row.get(5)?;
            let kind_str: String = row.get(6)?;
            let replacement: String = row.get(9)?;
            let supersedes_val: String = row.get(10)?;
            let is_tombstone_num: i64 = row.get(13).unwrap_or(0);

            Ok(Some(Document {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                path: row.get(2)?,
                title: row.get(3)?,
                topic: row.get(4)?,
                status: DocumentStatus::from_str(&status_str),
                kind: DocumentKind::from_str(&kind_str),
                owner: row.get(7)?,
                issue: row.get(8)?,
                replacement_id: if replacement.is_empty() {
                    None
                } else {
                    Some(replacement)
                },
                supersedes: if supersedes_val.is_empty() {
                    None
                } else {
                    Some(supersedes_val)
                },
                content: row.get(11)?,
                source: "repo document".into(),
                available: true,
                stale: false,
                declared_status: Some(status_str),
                checksum: row.get(12)?,
                worktree_state: None,
                is_tombstone: is_tombstone_num > 0,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn search(
        conn: &Connection,
        collection_ids: &[String],
        profile_id: &str,
        query: &str,
        limit: usize,
        include_private: bool,
        include_archived: bool,
    ) -> Result<Vec<Hit>> {
        let (and_expr, or_expr) = crate::core::QueryExpander::expand(query);
        if and_expr.is_empty() || collection_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut hits = Self::execute_fts_match(
            conn,
            collection_ids,
            profile_id,
            &and_expr,
            limit,
            include_private,
            include_archived,
            false,
        )?;

        if hits.is_empty() {
            if let Some(ref or_str) = or_expr {
                hits = Self::execute_fts_match(
                    conn,
                    collection_ids,
                    profile_id,
                    or_str,
                    limit,
                    include_private,
                    include_archived,
                    true,
                )?;
            }
        }

        Ok(hits)
    }

    fn execute_fts_match(
        conn: &Connection,
        collection_ids: &[String],
        profile_id: &str,
        match_expr: &str,
        limit: usize,
        include_private: bool,
        include_archived: bool,
        broadened: bool,
    ) -> Result<Vec<Hit>> {
        let placeholders = collection_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let tombstone_filter = if include_archived {
            ""
        } else {
            "AND d.is_tombstone = 0"
        };

        let sql = format!(
            "SELECT d.source_id, d.collection_id, d.path, d.title, d.topic,
                    e.effective_status, d.kind, e.replacement_id, d.supersedes,
                    d.checksum, snippet(documents_fts, 2, '[', ']', '…', 24),
                    bm25(documents_fts), d.is_tombstone
             FROM documents_fts
             JOIN documents d ON d.id = documents_fts.rowid
             JOIN effective_documents e ON e.id = d.id
             WHERE d.collection_id IN ({}) AND documents_fts MATCH ? {}
             ORDER BY bm25(documents_fts)
             LIMIT ?;",
            placeholders, tombstone_filter
        );

        let mut params: Vec<rusqlite::types::Value> = collection_ids
            .iter()
            .map(|id| rusqlite::types::Value::Text(id.clone()))
            .collect();
        params.push(rusqlite::types::Value::Text(match_expr.to_string()));
        params.push(rusqlite::types::Value::Integer(limit as i64));

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let status_str: String = row.get(5)?;
            let kind_str: String = row.get(6)?;
            let replacement: String = row.get(7)?;
            let supersedes_val: String = row.get(8)?;
            let is_tombstone_val: i64 = row.get(12).unwrap_or(0);

            Ok(Hit {
                id: row.get(0)?,
                collection_id: Some(row.get(1)?),
                source: "repo document".into(),
                path: Some(row.get(2)?),
                title: row.get(3)?,
                topic: Some(row.get(4)?),
                status: DocumentStatus::from_str(&status_str),
                kind: DocumentKind::from_str(&kind_str),
                replacement_id: if replacement.is_empty() {
                    None
                } else {
                    Some(replacement)
                },
                supersedes: if supersedes_val.is_empty() {
                    None
                } else {
                    Some(supersedes_val)
                },
                snippet: row.get(10)?,
                score: row.get(11)?,
                available: true,
                stale: false,
                declared_status: Some(status_str),
                worktree_state: None,
                broadened,
                is_tombstone: is_tombstone_val > 0,
            })
        })?;

        let mut hits = Vec::new();
        for hit in rows {
            hits.push(hit?);
        }

        if include_private {
            let mut mem_stmt = conn.prepare(
                "SELECT m.source_id, m.title, m.kind, m.origin,
                        snippet(memories_fts, 1, '[', ']', '…', 24),
                        bm25(memories_fts)
                 FROM memories_fts
                 JOIN private_memories m ON m.id = memories_fts.rowid
                 WHERE m.profile_id = ?1 AND memories_fts MATCH ?2
                 ORDER BY bm25(memories_fts)
                 LIMIT ?3;",
            )?;

            let mem_rows = mem_stmt.query_map(
                params![profile_id, match_expr, limit as i64],
                |row| {
                    let origin: String = row.get(3)?;
                    let source = if origin == "local" {
                        "private memory".to_string()
                    } else {
                        "imported private memory".to_string()
                    };

                    Ok(Hit {
                        id: row.get(0)?,
                        collection_id: None,
                        source,
                        path: None,
                        title: row.get(1)?,
                        topic: None,
                        status: DocumentStatus::Unknown,
                        kind: DocumentKind::from_str(&row.get::<_, String>(2)?),
                        replacement_id: None,
                        supersedes: None,
                        snippet: row.get(4)?,
                        score: row.get(5)?,
                        available: true,
                        stale: false,
                        declared_status: None,
                        worktree_state: None,
                        broadened,
                        is_tombstone: false,
                    })
                },
            )?;

            for hit in mem_rows {
                hits.push(hit?);
            }
        }

        hits.sort_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if hits.len() > limit {
            hits.truncate(limit);
        }

        Ok(hits)
    }

    pub fn browse(
        conn: &Connection,
        collection_ids: &[String],
        opts: &BrowseOptions,
    ) -> Result<(Vec<Document>, usize)> {
        if collection_ids.is_empty() {
            return Ok((Vec::new(), 0));
        }

        let placeholders = collection_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let mut where_clauses = vec![format!("d.collection_id IN ({})", placeholders)];
        let mut params: Vec<rusqlite::types::Value> = collection_ids
            .iter()
            .map(|id| rusqlite::types::Value::Text(id.clone()))
            .collect();

        if let Some(ref coll_id) = opts.collection_id {
            where_clauses.push("d.collection_id = ?".into());
            params.push(rusqlite::types::Value::Text(coll_id.clone()));
        }

        match opts.category.as_str() {
            "tasks" => where_clauses.push("d.kind = 'task'".into()),
            "decisions" => where_clauses.push("d.kind = 'decision'".into()),
            "risks" => where_clauses.push("d.kind = 'risk' AND e.effective_status = 'open'".into()),
            "proposals" => where_clauses.push("e.effective_status = 'proposed'".into()),
            "superseded" => where_clauses.push("e.effective_status = 'superseded'".into()),
            "specs" => where_clauses.push("d.path LIKE '%/specs/%' AND d.kind != 'directive'".into()),
            "plans" => where_clauses.push("d.path LIKE '%/plans/%' AND d.kind != 'directive'".into()),
            _ => where_clauses.push("d.kind != 'directive'".into()),
        }

        if let Some(ref kind) = opts.kind {
            where_clauses.push("d.kind = ?".into());
            params.push(rusqlite::types::Value::Text(kind.clone()));
        }

        if let Some(ref status) = opts.status {
            where_clauses.push("e.effective_status = ?".into());
            params.push(rusqlite::types::Value::Text(status.clone()));
        }

        if let Some(ref project) = opts.project {
            where_clauses.push("d.path LIKE ? ESCAPE '\\'".into());
            let escaped = project
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            params.push(rusqlite::types::Value::Text(format!("projects/{}/%", escaped)));
        }

        if let Some(ref topic) = opts.topic {
            where_clauses.push("d.topic = ?".into());
            params.push(rusqlite::types::Value::Text(topic.clone()));
        }

        if !opts.include_archived {
            where_clauses.push("d.is_tombstone = 0".into());
        }

        let where_sql = where_clauses.join(" AND ");

        let count_sql = format!(
            "SELECT count(*) FROM documents d
             JOIN effective_documents e ON e.id = d.id
             WHERE {};",
            where_sql
        );
        let total: usize = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(params.clone()),
            |r| r.get(0),
        )?;

        let order = if opts.recent {
            "d.indexed_at DESC, d.path ASC"
        } else {
            "d.path ASC"
        };

        let fetch_sql = format!(
            "SELECT d.source_id, d.collection_id, d.path, d.title, d.topic,
                    e.effective_status, d.kind, d.owner, d.issue,
                    e.replacement_id, d.supersedes, d.content, d.checksum,
                    d.is_tombstone
             FROM documents d
             JOIN effective_documents e ON e.id = d.id
             WHERE {}
             ORDER BY {}
             LIMIT ? OFFSET ?;",
            where_sql, order
        );

        let mut fetch_params = params;
        fetch_params.push(rusqlite::types::Value::Integer(opts.limit as i64));
        fetch_params.push(rusqlite::types::Value::Integer(opts.offset as i64));

        let mut stmt = conn.prepare(&fetch_sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(fetch_params), |row| {
            let status_str: String = row.get(5)?;
            let kind_str: String = row.get(6)?;
            let replacement: String = row.get(9)?;
            let supersedes_val: String = row.get(10)?;
            let is_tombstone_num: i64 = row.get(13).unwrap_or(0);

            Ok(Document {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                path: row.get(2)?,
                title: row.get(3)?,
                topic: row.get(4)?,
                status: DocumentStatus::from_str(&status_str),
                kind: DocumentKind::from_str(&kind_str),
                owner: row.get(7)?,
                issue: row.get(8)?,
                replacement_id: if replacement.is_empty() {
                    None
                } else {
                    Some(replacement)
                },
                supersedes: if supersedes_val.is_empty() {
                    None
                } else {
                    Some(supersedes_val)
                },
                content: row.get(11)?,
                source: "repo document".into(),
                available: true,
                stale: false,
                declared_status: Some(status_str),
                checksum: row.get(12)?,
                worktree_state: None,
                is_tombstone: is_tombstone_num > 0,
            })
        })?;

        let mut docs = Vec::new();
        for doc in rows {
            docs.push(doc?);
        }

        Ok((docs, total))
    }

    pub fn list_projects(
        conn: &Connection,
        collection_id: &str,
        include_archived: bool,
    ) -> Result<Vec<crate::domain::ProjectSummary>> {
        let filter = if include_archived {
            ""
        } else {
            "AND is_tombstone = 0 AND substr(path, 10, instr(substr(path, 10), '/') - 1) NOT IN ('_archive', '_draft', '_archived')"
        };

        let sql = format!(
            "SELECT
                substr(path, 10, instr(substr(path, 10), '/') - 1) AS proj_name,
                count(*) AS total_docs,
                count(CASE WHEN kind = 'task' AND status IN ('pending', 'todo', 'open') THEN 1 END) AS pending_tasks,
                count(CASE WHEN kind = 'task' AND status IN ('in_progress', 'active') THEN 1 END) AS in_progress_tasks,
                count(CASE WHEN kind = 'task' AND status IN ('completed', 'done', 'resolved') THEN 1 END) AS completed_tasks,
                count(CASE WHEN kind = 'task' AND status = 'blocked' THEN 1 END) AS blocked_tasks,
                count(CASE WHEN kind = 'risk' AND status = 'open' THEN 1 END) AS open_risks,
                count(CASE WHEN kind = 'decision' THEN 1 END) AS decisions_count,
                max(CASE WHEN lower(path) = 'projects/' || lower(substr(path, 10, instr(substr(path, 10), '/') - 1)) || '/status.md' THEN 1 ELSE 0 END) AS has_status
             FROM documents
             WHERE collection_id = ?1 AND path LIKE 'projects/%/%' {}
             GROUP BY proj_name
             HAVING length(proj_name) > 0
             ORDER BY proj_name ASC;",
            filter
        );

        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([collection_id])?;
        let mut summaries = Vec::new();

        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let total_docs: usize = row.get(1)?;
            let pending: usize = row.get(2)?;
            let in_progress: usize = row.get(3)?;
            let completed: usize = row.get(4)?;
            let blocked: usize = row.get(5)?;
            let open_risks: usize = row.get(6)?;
            let decisions: usize = row.get(7)?;
            let has_status_num: i64 = row.get(8)?;

            summaries.push(crate::domain::ProjectSummary {
                name: name.clone(),
                path: format!("projects/{}", name),
                total_documents: total_docs,
                tasks_pending: pending,
                tasks_in_progress: in_progress,
                tasks_completed: completed,
                tasks_blocked: blocked,
                open_risks,
                decisions_count: decisions,
                has_status_doc: has_status_num > 0,
                health: if blocked > 0 { "blocked".to_string() } else { "healthy".to_string() },
                active_task: None,
                exit_criteria: None,
                exit_verified: false,
                churn_warning: false,
            });
        }

        Ok(summaries)
    }

    pub fn remember(
        conn: &Connection,
        source_id: &str,
        profile_id: &str,
        title: &str,
        content: &str,
        kind: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO private_memories (source_id, profile_id, title, content, kind, origin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'local', ?6, ?6);",
            params![source_id, profile_id, title, content, kind, now],
        )?;
        Ok(())
    }

    pub fn memories_page(
        conn: &Connection,
        profile_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Memory>, usize)> {
        let total: usize = conn.query_row(
            "SELECT count(*) FROM private_memories WHERE profile_id = ?1;",
            [profile_id],
            |r| r.get(0),
        )?;

        let mut stmt = conn.prepare(
            "SELECT source_id, title, content, kind, origin, created_at
             FROM private_memories
             WHERE profile_id = ?1
             ORDER BY created_at DESC, id DESC
             LIMIT ?2 OFFSET ?3;",
        )?;

        let rows = stmt.query_map(params![profile_id, limit as i64, offset as i64], |row| {
            Ok(Memory {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                kind: row.get(3)?,
                origin: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut memories = Vec::new();
        for mem in rows {
            memories.push(mem?);
        }

        Ok((memories, total))
    }

    pub fn get_open_risks(
        conn: &Connection,
        collection_id: &str,
    ) -> Result<Vec<(Document, Vec<String>, Vec<String>, Vec<String>)>> {
        let mut stmt = conn.prepare(
            "SELECT d.source_id, d.collection_id, d.path, d.title, d.topic,
                    e.effective_status, d.kind, d.owner, d.issue,
                    e.replacement_id, d.supersedes, d.content, d.checksum,
                    d.risk_paths, d.risk_versions, d.risk_environments
             FROM effective_documents e
             JOIN documents d ON d.id = e.id
             WHERE d.collection_id = ?1 AND d.kind = 'risk' AND e.effective_status NOT IN ('superseded', 'resolved') AND d.is_tombstone = 0;",
        )?;

        let mut rows = stmt.query([collection_id])?;
        let mut results = Vec::new();

        while let Some(row) = rows.next()? {
            let status_str: String = row.get(5)?;
            let kind_str: String = row.get(6)?;
            let replacement: String = row.get(9)?;
            let supersedes_val: String = row.get(10)?;
            let paths_json: String = row.get(13)?;
            let versions_json: String = row.get(14)?;
            let envs_json: String = row.get(15)?;

            let doc = Document {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                path: row.get(2)?,
                title: row.get(3)?,
                topic: row.get(4)?,
                status: DocumentStatus::from_str(&status_str),
                kind: DocumentKind::from_str(&kind_str),
                owner: row.get(7)?,
                issue: row.get(8)?,
                replacement_id: if replacement.is_empty() { None } else { Some(replacement) },
                supersedes: if supersedes_val.is_empty() { None } else { Some(supersedes_val) },
                content: row.get(11)?,
                source: "repo document".into(),
                available: true,
                stale: false,
                declared_status: Some(status_str),
                checksum: row.get(12)?,
                worktree_state: None,
                is_tombstone: false,
            };

            let paths: Vec<String> = serde_json::from_str(&paths_json).unwrap_or_default();
            let versions: Vec<String> = serde_json::from_str(&versions_json).unwrap_or_default();
            let envs: Vec<String> = serde_json::from_str(&envs_json).unwrap_or_default();

            results.push((doc, paths, versions, envs));
        }

        Ok(results)
    }

    pub fn check_work(
        conn: &Connection,
        collection_id: &str,
        target_paths: &[String],
        target_version: Option<&str>,
        target_env: Option<&str>,
    ) -> Result<crate::domain::RiskCheck> {
        let open_risks = Self::get_open_risks(conn, collection_id)?;
        let mut check = crate::core::RiskEngine::check_paths(
            &open_risks,
            target_paths,
            target_version,
            target_env,
        );
        let applicable_dirs = Self::get_active_directives_for_paths(conn, collection_id, target_paths, 5)?;
        check.applicable_directives = applicable_dirs;
        Ok(check)
    }

    pub fn create_session(conn: &Connection, session: &AgentSession) -> Result<()> {
        conn.execute(
            "INSERT INTO agent_sessions (
                id, collection_id, profile_id, agent_id, grant_id, project, started_at,
                ended_at, total_tool_calls, total_edits, total_diff_lines,
                risks_cited, risks_prevented, review_loops, first_pass_clean, status
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16);",
            params![
                session.id,
                session.collection_id,
                session.profile_id,
                session.agent_id,
                session.grant_id,
                session.project,
                session.started_at,
                session.ended_at,
                session.total_tool_calls,
                session.total_edits,
                session.total_diff_lines,
                session.risks_cited,
                session.risks_prevented,
                session.review_loops,
                if session.first_pass_clean { 1 } else { 0 },
                session.status,
            ],
        )?;
        Ok(())
    }

    pub fn update_session(conn: &Connection, session: &AgentSession) -> Result<()> {
        conn.execute(
            "UPDATE agent_sessions SET
                ended_at = ?2,
                total_tool_calls = ?3,
                total_edits = ?4,
                total_diff_lines = ?5,
                risks_cited = ?6,
                risks_prevented = ?7,
                review_loops = ?8,
                first_pass_clean = ?9,
                status = ?10,
                project = ?11
             WHERE id = ?1;",
            params![
                session.id,
                session.ended_at,
                session.total_tool_calls,
                session.total_edits,
                session.total_diff_lines,
                session.risks_cited,
                session.risks_prevented,
                session.review_loops,
                if session.first_pass_clean { 1 } else { 0 },
                session.status,
                session.project,
            ],
        )?;
        Ok(())
    }

    pub fn record_session_event(
        conn: &Connection,
        session_id: &str,
        event_kind: &str,
        target_path: &str,
        query_or_tool: &str,
        detail_json: &str,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO session_events (session_id, timestamp, event_kind, target_path, query_or_tool, detail_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![session_id, now, event_kind, target_path, query_or_tool, detail_json],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_session(conn: &Connection, session_id: &str) -> Result<Option<AgentSession>> {
        let mut stmt = conn.prepare(
            "SELECT id, collection_id, profile_id, agent_id, grant_id, started_at, ended_at,
                    total_tool_calls, total_edits, total_diff_lines, risks_cited, risks_prevented,
                    review_loops, first_pass_clean, status, project
             FROM agent_sessions WHERE id = ?1;",
        )?;
        let mut rows = stmt.query([session_id])?;
        if let Some(row) = rows.next()? {
            let first_pass_clean_int: i32 = row.get(13)?;
            Ok(Some(AgentSession {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                profile_id: row.get(2)?,
                agent_id: row.get(3)?,
                grant_id: row.get(4)?,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
                total_tool_calls: row.get(7)?,
                total_edits: row.get(8)?,
                total_diff_lines: row.get(9)?,
                risks_cited: row.get(10)?,
                risks_prevented: row.get(11)?,
                review_loops: row.get(12)?,
                first_pass_clean: first_pass_clean_int == 1,
                status: row.get(14)?,
                project: row.get(15)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_session_agent_id(conn: &Connection, session_id: &str, agent_id: &str) -> Result<()> {
        conn.execute(
            "UPDATE agent_sessions SET agent_id = ?1 WHERE id = ?2;",
            params![agent_id, session_id],
        )?;
        Ok(())
    }

    pub fn update_session_project(conn: &Connection, session_id: &str, project: &str) -> Result<()> {
        conn.execute(
            "UPDATE agent_sessions SET project = ?1 WHERE id = ?2;",
            params![project, session_id],
        )?;
        Ok(())
    }

    pub fn retire_stale_sessions(conn: &Connection, collection_id: &str, stale_seconds: i64) -> Result<usize> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(stale_seconds)).to_rfc3339();
        let changed = conn.execute(
            "UPDATE agent_sessions 
             SET status = 'idle', ended_at = coalesce(ended_at, started_at) 
             WHERE collection_id = ?1 AND status = 'active' AND started_at < ?2;",
            params![collection_id, cutoff],
        )?;
        Ok(changed)
    }

    pub fn prune_stale_sessions(conn: &Connection, collection_id: &str) -> Result<usize> {
        let now_str = chrono::Utc::now().to_rfc3339();
        let changed = conn.execute(
            "UPDATE agent_sessions 
             SET status = 'completed', ended_at = coalesce(ended_at, ?2) 
             WHERE collection_id = ?1 AND status != 'completed';",
            params![collection_id, now_str],
        )?;
        Ok(changed)
    }

    pub fn list_sessions(
        conn: &Connection,
        collection_id: &str,
        limit: usize,
    ) -> Result<Vec<AgentSession>> {
        let _ = Self::retire_stale_sessions(conn, collection_id, 900);
        let mut stmt = conn.prepare(
            "SELECT id, collection_id, profile_id, agent_id, grant_id, started_at, ended_at,
                    total_tool_calls, total_edits, total_diff_lines, risks_cited, risks_prevented,
                    review_loops, first_pass_clean, status, project
             FROM agent_sessions
             WHERE collection_id = ?1
             ORDER BY started_at DESC
             LIMIT ?2;",
        )?;
        let rows = stmt.query_map(params![collection_id, limit as i64], |row| {
            let first_pass_clean_int: i32 = row.get(13)?;
            Ok(AgentSession {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                profile_id: row.get(2)?,
                agent_id: row.get(3)?,
                grant_id: row.get(4)?,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
                total_tool_calls: row.get(7)?,
                total_edits: row.get(8)?,
                total_diff_lines: row.get(9)?,
                risks_cited: row.get(10)?,
                risks_prevented: row.get(11)?,
                review_loops: row.get(12)?,
                first_pass_clean: first_pass_clean_int == 1,
                status: row.get(14)?,
                project: row.get(15)?,
            })
        })?;

        let mut out = Vec::new();
        for s in rows {
            out.push(s?);
        }
        Ok(out)
    }

    pub fn get_session_events(
        conn: &Connection,
        session_id: &str,
    ) -> Result<Vec<SessionEventRecord>> {
        let mut stmt = conn.prepare(
            "SELECT id, session_id, timestamp, event_kind, target_path, query_or_tool, detail_json
             FROM session_events
             WHERE session_id = ?1
             ORDER BY id ASC;",
        )?;
        let rows = stmt.query_map([session_id], |row| {
            Ok(SessionEventRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                timestamp: row.get(2)?,
                event_kind: row.get(3)?,
                target_path: row.get(4)?,
                query_or_tool: row.get(5)?,
                detail_json: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_recent_hotspots(
        conn: &Connection,
        collection_id: &str,
        limit: usize,
    ) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT se.target_path, count(*) as cnt
             FROM session_events se
             JOIN agent_sessions s ON s.id = se.session_id
             WHERE s.collection_id = ?1 AND se.target_path != ''
             GROUP BY se.target_path
             ORDER BY cnt DESC
             LIMIT ?2;",
        )?;
        let rows = stmt.query_map(params![collection_id, limit as i64], |row| {
            row.get::<_, String>(0)
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_recent_zero_hit_queries(
        conn: &Connection,
        collection_id: &str,
        limit: usize,
    ) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT se.query_or_tool
             FROM session_events se
             JOIN agent_sessions s ON s.id = se.session_id
             WHERE s.collection_id = ?1 AND se.event_kind = 'zero_hit_query' AND se.query_or_tool != ''
             ORDER BY se.id DESC
             LIMIT ?2;",
        )?;
        let rows = stmt.query_map(params![collection_id, limit as i64], |row| {
            row.get::<_, String>(0)
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_recent_friction_warnings(
        conn: &Connection,
        collection_id: &str,
    ) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT se.target_path, count(*) as cnt
             FROM session_events se
             JOIN agent_sessions s ON s.id = se.session_id
             WHERE s.collection_id = ?1 AND se.target_path != '' AND se.event_kind IN ('file_edit', 'review_oscillation', 'risk_cited')
             GROUP BY se.target_path
             HAVING cnt >= 3
             ORDER BY cnt DESC
             LIMIT 5;",
        )?;
        let rows = stmt.query_map([collection_id], |row| {
            let path: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok(format!("`{}` experienced high churn ({} edits/risk citations); verify invariants before modifying", path, count))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_active_invariants(
        conn: &Connection,
        collection_id: &str,
        limit: usize,
    ) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT d.title, d.topic FROM documents d
             JOIN effective_documents e ON e.id = d.id
             WHERE d.collection_id = ?1 AND d.kind = 'decision' AND e.effective_status = 'accepted' AND d.is_tombstone = 0
             ORDER BY d.id DESC LIMIT ?2;",
        )?;
        let rows = stmt.query_map(params![collection_id, limit as i64], |row| {
            let title: String = row.get(0)?;
            let topic: String = row.get(1)?;
            if topic.is_empty() {
                Ok(title)
            } else {
                Ok(format!("[{}] {}", topic, title))
            }
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn upsert_directive(conn: &Connection, directive: &Directive) -> Result<()> {
        let scope_json =
            serde_json::to_string(&directive.scope).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO directives (
                id, collection_id, title, category, status, author, scope_json, enforcement, supersedes, content, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                collection_id=excluded.collection_id,
                title=excluded.title,
                category=excluded.category,
                status=excluded.status,
                author=excluded.author,
                scope_json=excluded.scope_json,
                enforcement=excluded.enforcement,
                supersedes=excluded.supersedes,
                content=excluded.content,
                created_at=excluded.created_at;",
            params![
                directive.id,
                directive.collection_id,
                directive.title,
                directive.category,
                directive.status,
                directive.author,
                scope_json,
                directive.enforcement,
                directive.supersedes,
                directive.content,
                directive.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_directive(conn: &Connection, id: &str) -> Result<Option<Directive>> {
        let mut stmt = conn.prepare(
            "SELECT id, collection_id, title, category, status, author, scope_json, enforcement, supersedes, content, created_at
             FROM directives WHERE id = ?1;",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            let scope_raw: String = row.get(6)?;
            let scope: Vec<String> = serde_json::from_str(&scope_raw).unwrap_or_default();
            Ok(Some(Directive {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                title: row.get(2)?,
                category: row.get(3)?,
                status: row.get(4)?,
                author: row.get(5)?,
                scope,
                enforcement: row.get(7)?,
                supersedes: row.get(8)?,
                content: row.get(9)?,
                created_at: row.get(10)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_directives(
        conn: &Connection,
        collection_id: &str,
        category: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<Directive>> {
        let mut sql = "SELECT id, collection_id, title, category, status, author, scope_json, enforcement, supersedes, content, created_at
             FROM directives WHERE collection_id = ?1".to_string();
        let mut params_vec: Vec<rusqlite::types::Value> =
            vec![rusqlite::types::Value::Text(collection_id.to_string())];

        if let Some(cat) = category {
            sql.push_str(" AND LOWER(category) = LOWER(?)");
            params_vec.push(rusqlite::types::Value::Text(cat.to_string()));
        }
        if let Some(st) = status {
            sql.push_str(" AND status = ?");
            params_vec.push(rusqlite::types::Value::Text(st.to_string()));
        }

        sql.push_str(" ORDER BY created_at DESC;");

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec), |row| {
            let scope_raw: String = row.get(6)?;
            let scope: Vec<String> = serde_json::from_str(&scope_raw).unwrap_or_default();
            Ok(Directive {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                title: row.get(2)?,
                category: row.get(3)?,
                status: row.get(4)?,
                author: row.get(5)?,
                scope,
                enforcement: row.get(7)?,
                supersedes: row.get(8)?,
                content: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;

        let mut out = Vec::new();
        for d in rows {
            out.push(d?);
        }
        Ok(out)
    }

    pub fn retire_directive(conn: &Connection, id: &str) -> Result<bool> {
        let affected = conn.execute("UPDATE directives SET status = 'retired' WHERE id = ?1;", [id])?;
        Ok(affected > 0)
    }

    pub fn activate_directive(conn: &Connection, id: &str) -> Result<bool> {
        let affected = conn.execute("UPDATE directives SET status = 'active' WHERE id = ?1;", [id])?;
        Ok(affected > 0)
    }

    pub fn supersede_directive(conn: &Connection, old_id: &str, new_id: &str) -> Result<()> {
        conn.execute(
            "UPDATE directives SET status = 'superseded', supersedes = ?2 WHERE id = ?1;",
            params![old_id, new_id],
        )?;
        Ok(())
    }

    pub fn get_active_directives_for_paths(
        conn: &Connection,
        collection_id: &str,
        paths: &[String],
        limit: usize,
    ) -> Result<Vec<Directive>> {
        let active = Self::list_directives(conn, collection_id, None, Some("active"))?;
        Ok(Directive::filter_relevant(&active, paths, limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;

    #[test]
    fn test_upsert_and_fts_search() -> Result<()> {
        let db = Database::open_in_memory("coll_1", "prof_1")?;
        let meta = RecordMeta {
            id: "doc_test_1".into(),
            kind: "decision".into(),
            status: "accepted".into(),
            owner: "wiqar".into(),
            issue: None,
            paths: vec!["src/main.rs".into()],
            versions: vec!["1.0.0".into()],
            environments: vec!["prod".into()],
            supersedes: None,
            delegation: None,
        };

        Queries::upsert_document(
            db.conn(),
            "doc_test_1",
            "coll_1",
            "docs/decisions/0001-arch.md",
            "Architecture",
            "Zero-Cost Rust Architecture",
            "This decision replaces Go with high-performance Rust.",
            "Zero-Cost Rust Architecture This decision replaces Go with high-performance Rust.",
            &meta,
            "abc123hash",
            false,
        )?;

        let hits = Queries::search(
            db.conn(),
            &["coll_1".into()],
            "prof_1",
            "architecture",
            10,
            false,
            false,
        )?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "doc_test_1");
        assert_eq!(hits[0].title, "Zero-Cost Rust Architecture");
        assert!(hits[0].snippet.contains("[Architecture]"));

        let hits_broadened = Queries::search(
            db.conn(),
            &["coll_1".into()],
            "prof_1",
            "architecture missingword",
            10,
            false,
            false,
        )?;
        assert_eq!(hits_broadened.len(), 1);
        assert!(hits_broadened[0].broadened);

        Ok(())
    }

    #[test]
    fn test_private_memory_search() -> Result<()> {
        let db = Database::open_in_memory("coll_1", "prof_1")?;
        Queries::remember(
            db.conn(),
            "mem_001",
            "prof_1",
            "Private Note on Auth",
            "Remember to double check token expiration logic.",
            "note",
        )?;

        let hits = Queries::search(
            db.conn(),
            &["coll_1".into()],
            "prof_1",
            "token expiration",
            10,
            true,
            false,
        )?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "mem_001");
        assert_eq!(hits[0].source, "private memory");

        let hits_agent = Queries::search(
            db.conn(),
            &["coll_1".into()],
            "prof_1",
            "token expiration",
            10,
            false,
            false,
        )?;
        assert_eq!(hits_agent.len(), 0);

        Ok(())
    }

    #[test]
    fn test_check_work_against_indexed_risk() -> Result<()> {
        let db = Database::open_in_memory("coll_1", "prof_1")?;
        let meta = RecordMeta {
            id: "risk_auth_01".into(),
            kind: "risk".into(),
            status: "accepted".into(),
            owner: "wiqar".into(),
            issue: Some("AUTH-401".into()),
            paths: vec!["src/auth/**".into(), "tokens/jwt.go".into()],
            versions: vec!["v2.0".into()],
            environments: vec!["production".into()],
            supersedes: None,
            delegation: None,
        };

        Queries::upsert_document(
            db.conn(),
            "risk_auth_01",
            "coll_1",
            "risks/2026-auth-vulnerability.md",
            "Security",
            "JWT Session Invalidation Flaw",
            "Editing auth tokens without revoking sessions causes CVE regression.",
            "JWT Session Invalidation Flaw Editing auth tokens without revoking sessions causes CVE regression.",
            &meta,
            "hash999",
            false,
        )?;

        let check = Queries::check_work(
            db.conn(),
            "coll_1",
            &["src/auth/session.rs".into()],
            Some("v2.0"),
            Some("production"),
        )?;
        assert_eq!(check.matches.len(), 1);
        assert_eq!(check.matches[0].document.id, "risk_auth_01");
        assert_eq!(check.matches[0].applicability, crate::domain::RiskApplicability::Applies);

        let clean_check = Queries::check_work(
            db.conn(),
            "coll_1",
            &["src/ui/layout.rs".into()],
            Some("v2.0"),
            Some("production"),
        )?;
        assert_eq!(clean_check.matches.len(), 0);

        Ok(())
    }

    #[test]
    fn test_synonym_expansion_search() -> Result<()> {
        let db = Database::open_in_memory("coll_syn", "prof_syn")?;

        let meta1 = RecordMeta {
            id: "doc_db".into(),
            kind: "decision".into(),
            status: "accepted".into(),
            owner: "architect".into(),
            issue: None,
            paths: vec!["src/storage/mod.rs".into()],
            versions: vec!["1.0.0".into()],
            environments: vec!["production".into()],
            supersedes: None,
            delegation: None,
        };

        Queries::upsert_document(
            db.conn(),
            "doc_db",
            "coll_syn",
            "docs/decisions/0010-storage.md",
            "Storage",
            "Relational Storage Engine",
            "We are standardizing our database layer with WAL journal mode and busy timeouts.",
            "Relational Storage Engine We are standardizing our database layer with WAL journal mode and busy timeouts.",
            &meta1,
            "hash_db_1",
            false,
        )?;

        let meta2 = RecordMeta {
            id: "doc_auth".into(),
            kind: "decision".into(),
            status: "accepted".into(),
            owner: "security".into(),
            issue: None,
            paths: vec!["src/security/mod.rs".into()],
            versions: vec!["1.0.0".into()],
            environments: vec!["production".into()],
            supersedes: None,
            delegation: None,
        };

        Queries::upsert_document(
            db.conn(),
            "doc_auth",
            "coll_syn",
            "docs/decisions/0011-tokens.md",
            "Security",
            "Session Invalidation",
            "All user credentials and tokens must be verified before granting access.",
            "Session Invalidation All user credentials and tokens must be verified before granting access.",
            &meta2,
            "hash_auth_1",
            false,
        )?;

        // Search for shorthand "db" should find the document that only mentions "database"
        let db_hits = Queries::search(
            db.conn(),
            &["coll_syn".into()],
            "prof_syn",
            "db",
            10,
            false,
            false,
        )?;
        assert_eq!(db_hits.len(), 1);
        assert_eq!(db_hits[0].id, "doc_db");

        // Search for shorthand "auth" should find the document that only mentions "credentials" and "tokens"
        let auth_hits = Queries::search(
            db.conn(),
            &["coll_syn".into()],
            "prof_syn",
            "auth",
            10,
            false,
            false,
        )?;
        assert_eq!(auth_hits.len(), 1);
        assert_eq!(auth_hits[0].id, "doc_auth");

        // Search with exact phrase
        let phrase_hits = Queries::search(
            db.conn(),
            &["coll_syn".into()],
            "prof_syn",
            r#""database layer""#,
            10,
            false,
            false,
        )?;
        assert_eq!(phrase_hits.len(), 1);
        assert_eq!(phrase_hits[0].id, "doc_db");

        Ok(())
    }

    #[test]
    fn test_session_storage_and_queries() -> Result<()> {
        let db = Database::open_in_memory("coll_sess", "prof_sess")?;
        let session = AgentSession::new(
            "sess_001",
            "coll_sess",
            "prof_sess",
            "agent_rust",
            Some("grant_123".into()),
        );

        Queries::create_session(db.conn(), &session)?;

        let retrieved = Queries::get_session(db.conn(), "sess_001")?.expect("session found");
        assert_eq!(retrieved.id, "sess_001");
        assert_eq!(retrieved.agent_id, "agent_rust");
        assert_eq!(retrieved.grant_id.as_deref(), Some("grant_123"));

        Queries::record_session_event(
            db.conn(),
            "sess_001",
            "file_edit",
            "src/core/git.rs",
            "edit",
            "{}",
        )?;
        Queries::record_session_event(
            db.conn(),
            "sess_001",
            "zero_hit_query",
            "",
            "tokio reactor",
            "{}",
        )?;

        let events = Queries::get_session_events(db.conn(), "sess_001")?;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_kind, "file_edit");
        assert_eq!(events[1].event_kind, "zero_hit_query");

        let hotspots = Queries::get_recent_hotspots(db.conn(), "coll_sess", 5)?;
        assert_eq!(hotspots, vec!["src/core/git.rs".to_string()]);

        let zero_queries = Queries::get_recent_zero_hit_queries(db.conn(), "coll_sess", 5)?;
        assert_eq!(zero_queries, vec!["tokio reactor".to_string()]);

        let sessions = Queries::list_sessions(db.conn(), "coll_sess", 10)?;
        assert_eq!(sessions.len(), 1);

        Ok(())
    }

    #[test]
    fn test_directive_storage_and_queries() -> Result<()> {
        let db = Database::open_in_memory("coll_dir", "prof_dir")?;

        let dir1 = Directive::new(
            "DIR-001",
            "coll_dir",
            "Prime Directive",
            "architecture",
            "wiqar",
            vec!["*".to_string()],
            "check_work",
            None,
            "Never break existing input.",
        );

        let dir2 = Directive::new(
            "DIR-002",
            "coll_dir",
            "Zero Code Comments",
            "behavior",
            "wiqar",
            vec!["src/**".to_string()],
            "check_work",
            None,
            "Code must be self-documenting.",
        );

        Queries::upsert_directive(db.conn(), &dir1)?;
        Queries::upsert_directive(db.conn(), &dir2)?;

        let retrieved = Queries::get_directive(db.conn(), "DIR-001")?.expect("found");
        assert_eq!(retrieved.title, "Prime Directive");
        assert_eq!(retrieved.category, "architecture");

        let all = Queries::list_directives(db.conn(), "coll_dir", None, None)?;
        assert_eq!(all.len(), 2);

        let behavior = Queries::list_directives(db.conn(), "coll_dir", Some("behavior"), None)?;
        assert_eq!(behavior.len(), 1);
        assert_eq!(behavior[0].id, "DIR-002");

        let relevant = Queries::get_active_directives_for_paths(
            db.conn(),
            "coll_dir",
            &["src/main.rs".to_string()],
            5,
        )?;
        assert_eq!(relevant.len(), 2);

        Queries::retire_directive(db.conn(), "DIR-002")?;
        let active_only = Queries::list_directives(db.conn(), "coll_dir", None, Some("active"))?;
        assert_eq!(active_only.len(), 1);
        assert_eq!(active_only[0].id, "DIR-001");

        Ok(())
    }

    #[test]
    fn test_list_projects_aggregation() -> Result<()> {
        let db = Database::open_in_memory("coll_proj", "prof_proj")?;

        let meta_task = RecordMeta {
            paths: vec![],
            versions: vec![],
            environments: vec![],
            supersedes: None,
            issue: None,
            ..Default::default()
        };

        Queries::upsert_document(
            db.conn(),
            "doc-1",
            "coll_proj",
            "projects/alpha/tasks/task1.md",
            "task",
            "Alpha Task 1",
            "Pending task",
            "Pending task",
            &meta_task,
            "chk1",
            false,
        )?;
        db.conn().execute(
            "UPDATE documents SET kind = 'task', status = 'pending' WHERE source_id = 'doc-1';",
            [],
        )?;

        Queries::upsert_document(
            db.conn(),
            "doc-2",
            "coll_proj",
            "projects/alpha/tasks/task2.md",
            "task",
            "Alpha Task 2",
            "In progress task",
            "In progress task",
            &meta_task,
            "chk2",
            false,
        )?;
        db.conn().execute(
            "UPDATE documents SET kind = 'task', status = 'in_progress' WHERE source_id = 'doc-2';",
            [],
        )?;

        Queries::upsert_document(
            db.conn(),
            "doc-3",
            "coll_proj",
            "projects/alpha/risks/risk1.md",
            "risk",
            "Alpha Risk 1",
            "Open risk",
            "Open risk",
            &meta_task,
            "chk3",
            false,
        )?;
        db.conn().execute(
            "UPDATE documents SET kind = 'risk', status = 'open' WHERE source_id = 'doc-3';",
            [],
        )?;

        Queries::upsert_document(
            db.conn(),
            "doc-4",
            "coll_proj",
            "projects/alpha/status.md",
            "status",
            "Alpha Project Status",
            "Current status",
            "Current status",
            &meta_task,
            "chk4",
            false,
        )?;

        Queries::upsert_document(
            db.conn(),
            "doc-5",
            "coll_proj",
            "projects/beta/tasks/task1.md",
            "task",
            "Beta Task 1",
            "Completed task",
            "Completed task",
            &meta_task,
            "chk5",
            false,
        )?;
        db.conn().execute(
            "UPDATE documents SET kind = 'task', status = 'completed' WHERE source_id = 'doc-5';",
            [],
        )?;

        let projects = Queries::list_projects(db.conn(), "coll_proj", false)?;
        assert_eq!(projects.len(), 2);

        let alpha = &projects[0];
        assert_eq!(alpha.name, "alpha");
        assert_eq!(alpha.path, "projects/alpha");
        assert_eq!(alpha.total_documents, 4);
        assert_eq!(alpha.tasks_pending, 1);
        assert_eq!(alpha.tasks_in_progress, 1);
        assert_eq!(alpha.tasks_completed, 0);
        assert_eq!(alpha.open_risks, 1);
        assert!(alpha.has_status_doc);

        let beta = &projects[1];
        assert_eq!(beta.name, "beta");
        assert_eq!(beta.path, "projects/beta");
        assert_eq!(beta.total_documents, 1);
        assert_eq!(beta.tasks_completed, 1);
        assert!(!beta.has_status_doc);

        Ok(())
    }

    #[test]
    fn test_tombstone_filtering() -> Result<()> {
        let db = Database::open_in_memory("coll_tomb", "prof_tomb")?;
        let meta = RecordMeta::default();

        // 1. Active document
        Queries::upsert_document(
            db.conn(),
            "doc_active",
            "coll_tomb",
            "projects/api/specs/auth.md",
            "specs",
            "Auth Specification",
            "Active auth specification details",
            "Auth Specification Active auth specification details",
            &meta,
            "chk_act",
            false,
        )?;

        // 2. Tombstoned document (e.g. archived plan)
        Queries::upsert_document(
            db.conn(),
            "doc_archived",
            "coll_tomb",
            "projects/api/plans/old_plan.md",
            "plans",
            "Old Auth Plan",
            "Obsolete implementation plan for auth",
            "Old Auth Plan Obsolete implementation plan for auth",
            &meta,
            "chk_arc",
            true,
        )?;

        // Default search: tombstones excluded
        let search_clean = Queries::search(
            db.conn(),
            &["coll_tomb".into()],
            "prof_tomb",
            "auth",
            10,
            false,
            false,
        )?;
        assert_eq!(search_clean.len(), 1);
        assert_eq!(search_clean[0].id, "doc_active");

        // Search with include_archived: both returned
        let search_all = Queries::search(
            db.conn(),
            &["coll_tomb".into()],
            "prof_tomb",
            "auth",
            10,
            false,
            true,
        )?;
        assert_eq!(search_all.len(), 2);

        // Default browse: tombstones excluded
        let opts_default = BrowseOptions {
            collection_id: Some("coll_tomb".into()),
            include_archived: false,
            ..Default::default()
        };
        let (browse_clean, total_clean) = Queries::browse(db.conn(), &["coll_tomb".into()], &opts_default)?;
        assert_eq!(total_clean, 1);
        assert_eq!(browse_clean.len(), 1);
        assert_eq!(browse_clean[0].id, "doc_active");

        // Browse with include_archived: both returned
        let opts_all = BrowseOptions {
            collection_id: Some("coll_tomb".into()),
            include_archived: true,
            ..Default::default()
        };
        let (browse_all, total_all) = Queries::browse(db.conn(), &["coll_tomb".into()], &opts_all)?;
        assert_eq!(total_all, 2);
        assert_eq!(browse_all.len(), 2);

        Ok(())
    }
}


