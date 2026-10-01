use crate::domain::{
    BrowseOptions, Document, DocumentKind, DocumentStatus, Hit, Memory, RecordMeta,
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
    ) -> Result<()> {
        let risk_paths = serde_json::to_string(&meta.paths).unwrap_or_else(|_| "[]".into());
        let risk_versions = serde_json::to_string(&meta.versions).unwrap_or_else(|_| "[]".into());
        let risk_environments =
            serde_json::to_string(&meta.environments).unwrap_or_else(|_| "[]".into());
        let supersedes = meta.supersedes.as_deref().unwrap_or("");
        let issue = meta.issue.as_deref().unwrap_or("");
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "DELETE FROM documents WHERE collection_id = ?1 AND path = ?2 AND source_id != ?3;",
            params![collection_id, path, source_id],
        )?;

        conn.execute(
            "INSERT INTO documents (
                source_id, collection_id, path, topic, title, content, search_text,
                status, kind, owner, issue, risk_paths, risk_versions, risk_environments,
                supersedes, checksum, indexed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
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
                indexed_at=excluded.indexed_at;",
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
                now
            ],
        )?;
        Ok(())
    }

    pub fn get_document(conn: &Connection, source_id: &str) -> Result<Option<Document>> {
        let mut stmt = conn.prepare(
            "SELECT d.source_id, d.collection_id, d.path, d.title, d.topic,
                    e.effective_status, d.kind, d.owner, d.issue,
                    e.replacement_id, d.supersedes, d.content, d.checksum
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
        broadened: bool,
    ) -> Result<Vec<Hit>> {
        let placeholders = collection_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let sql = format!(
            "SELECT d.source_id, d.collection_id, d.path, d.title, d.topic,
                    e.effective_status, d.kind, e.replacement_id, d.supersedes,
                    d.checksum, snippet(documents_fts, 2, '[', ']', '…', 24),
                    bm25(documents_fts)
             FROM documents_fts
             JOIN documents d ON d.id = documents_fts.rowid
             JOIN effective_documents e ON e.id = d.id
             WHERE d.collection_id IN ({}) AND documents_fts MATCH ?
             ORDER BY bm25(documents_fts)
             LIMIT ?;",
            placeholders
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
            "decisions" => where_clauses.push("d.kind = 'decision'".into()),
            "risks" => where_clauses.push("d.kind = 'risk' AND e.effective_status = 'open'".into()),
            "proposals" => where_clauses.push("e.effective_status = 'proposed'".into()),
            "superseded" => where_clauses.push("e.effective_status = 'superseded'".into()),
            "specs" => where_clauses.push("d.path LIKE '%/specs/%'".into()),
            "plans" => where_clauses.push("d.path LIKE '%/plans/%'".into()),
            _ => {}
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
                    e.replacement_id, d.supersedes, d.content, d.checksum
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
            })
        })?;

        let mut docs = Vec::new();
        for doc in rows {
            docs.push(doc?);
        }

        Ok((docs, total))
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
             WHERE d.collection_id = ?1 AND d.kind = 'risk' AND e.effective_status NOT IN ('superseded', 'resolved');",
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
        Ok(crate::core::RiskEngine::check_paths(
            &open_risks,
            target_paths,
            target_version,
            target_env,
        ))
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
        )?;

        let hits = Queries::search(
            db.conn(),
            &["coll_1".into()],
            "prof_1",
            "architecture",
            10,
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
        )?;

        // Search for shorthand "db" should find the document that only mentions "database"
        let db_hits = Queries::search(
            db.conn(),
            &["coll_syn".into()],
            "prof_syn",
            "db",
            10,
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
        )?;
        assert_eq!(phrase_hits.len(), 1);
        assert_eq!(phrase_hits[0].id, "doc_db");

        Ok(())
    }
}


