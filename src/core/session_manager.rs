use crate::domain::{AgentSession, CriticalPathLock, SessionBriefing, SessionScorecard};
use crate::storage::Queries;
use chrono::Utc;
use rusqlite::Connection;
use uuid::Uuid;

pub struct SessionManager;

impl SessionManager {
    pub fn start_session(
        conn: &Connection,
        collection_id: &str,
        profile_id: &str,
        agent_id: &str,
        grant_id: Option<String>,
        project: Option<String>,
    ) -> Result<AgentSession, String> {
        let id = format!("sess_{}", Uuid::now_v7());
        let mut session = AgentSession::new(&id, collection_id, profile_id, agent_id, grant_id.clone());
        session.project = project.clone();
        Queries::create_session(conn, &session)
            .map_err(|e| format!("Failed to create session: {}", e))?;

        let detail = serde_json::json!({
            "agent_id": agent_id,
            "grant_id": grant_id,
            "project": project
        })
        .to_string();

        Queries::record_session_event(conn, &id, "session_start", "", agent_id, &detail)
            .map_err(|e| format!("Failed to record session start event: {}", e))?;

        Ok(session)
    }

    pub fn set_agent_id(
        conn: &Connection,
        session_id: &str,
        agent_id: &str,
    ) -> Result<(), String> {
        Queries::update_session_agent_id(conn, session_id, agent_id)
            .map_err(|e| format!("Failed to update agent_id: {}", e))?;
        let detail = serde_json::json!({ "agent_id": agent_id }).to_string();
        let _ = Queries::record_session_event(conn, session_id, "agent_identified", "", agent_id, &detail);
        Ok(())
    }

    pub fn set_project(
        conn: &Connection,
        session_id: &str,
        project: &str,
    ) -> Result<(), String> {
        Queries::update_session_project(conn, session_id, project)
            .map_err(|e| format!("Failed to update project: {}", e))?;
        let detail = serde_json::json!({ "project": project }).to_string();
        let _ = Queries::record_session_event(conn, session_id, "project_scoped", "", project, &detail);
        Ok(())
    }

    pub fn record_tool_call(
        conn: &Connection,
        session_id: &str,
        tool_name: &str,
        target_path: &str,
        detail_json: &str,
    ) -> Result<(), String> {
        let mut session = Queries::get_session(conn, session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        session.total_tool_calls += 1;
        Queries::update_session(conn, &session).map_err(|e| e.to_string())?;

        Queries::record_session_event(
            conn,
            session_id,
            "tool_call",
            target_path,
            tool_name,
            detail_json,
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn record_search(
        conn: &Connection,
        session_id: &str,
        query: &str,
        hit_count: usize,
    ) -> Result<(), String> {
        let event_kind = if hit_count == 0 {
            "zero_hit_query"
        } else {
            "search"
        };
        let detail = serde_json::json!({ "hits": hit_count }).to_string();

        Queries::record_session_event(conn, session_id, event_kind, "", query, &detail)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn record_file_edit(
        conn: &Connection,
        session_id: &str,
        path: &str,
        diff_lines: u32,
    ) -> Result<bool, String> {
        let mut session = Queries::get_session(conn, session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        let events = Queries::get_session_events(conn, session_id).map_err(|e| e.to_string())?;

        let prior_edits = events
            .iter()
            .filter(|e| e.event_kind == "file_edit" && e.target_path == path)
            .count();

        let is_loop = prior_edits >= 2;
        if is_loop {
            session.review_loops += 1;
            session.first_pass_clean = false;
        }

        session.total_edits += 1;
        session.total_diff_lines += diff_lines;
        Queries::update_session(conn, &session).map_err(|e| e.to_string())?;

        let detail = serde_json::json!({
            "diff_lines": diff_lines,
            "prior_edits": prior_edits
        })
        .to_string();

        Queries::record_session_event(conn, session_id, "file_edit", path, "edit", &detail)
            .map_err(|e| e.to_string())?;

        if is_loop {
            Queries::record_session_event(
                conn,
                session_id,
                "review_oscillation",
                path,
                "loop_detected",
                &detail,
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(is_loop)
    }

    pub fn record_risk_cited(
        conn: &Connection,
        session_id: &str,
        target_path: &str,
        risk_title: &str,
    ) -> Result<(), String> {
        let mut session = Queries::get_session(conn, session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        session.risks_cited += 1;
        session.first_pass_clean = false;
        Queries::update_session(conn, &session).map_err(|e| e.to_string())?;

        Queries::record_session_event(
            conn,
            session_id,
            "risk_cited",
            target_path,
            risk_title,
            "{}",
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn record_risk_prevented(
        conn: &Connection,
        session_id: &str,
        target_path: &str,
        risk_title: &str,
    ) -> Result<(), String> {
        let mut session = Queries::get_session(conn, session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        session.risks_prevented += 1;
        Queries::update_session(conn, &session).map_err(|e| e.to_string())?;

        Queries::record_session_event(
            conn,
            session_id,
            "risk_prevented",
            target_path,
            risk_title,
            "{}",
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn end_session(
        conn: &Connection,
        session_id: &str,
        status: &str,
    ) -> Result<SessionScorecard, String> {
        let mut session = Queries::get_session(conn, session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        session.ended_at = Some(Utc::now().to_rfc3339());
        session.status = status.to_string();
        Queries::update_session(conn, &session).map_err(|e| e.to_string())?;

        Queries::record_session_event(conn, session_id, "session_end", "", status, "{}")
            .map_err(|e| e.to_string())?;

        Self::compute_scorecard(conn, session_id)
    }

    pub fn compute_scorecard(
        conn: &Connection,
        session_id: &str,
    ) -> Result<SessionScorecard, String> {
        let session = Queries::get_session(conn, session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        let events = Queries::get_session_events(conn, session_id).map_err(|e| e.to_string())?;

        Ok(SessionScorecard::compute(&session, &events))
    }

    pub fn generate_briefing(
        conn: &Connection,
        collection_id: &str,
        grant_scope: Option<&str>,
    ) -> Result<SessionBriefing, String> {
        Self::generate_briefing_with_limit(conn, collection_id, grant_scope, 5)
    }

    pub fn generate_briefing_with_limit(
        conn: &Connection,
        collection_id: &str,
        grant_scope: Option<&str>,
        max_directives: usize,
    ) -> Result<SessionBriefing, String> {
        let active_invariants =
            Queries::get_active_invariants(conn, collection_id, 5).unwrap_or_default();

        let open_risks = Queries::get_open_risks(conn, collection_id).unwrap_or_default();
        let active_risks: Vec<String> = open_risks
            .iter()
            .take(5)
            .map(|(doc, _, _, _)| {
                if doc.topic.is_empty() {
                    doc.title.clone()
                } else {
                    format!("[{}] {}", doc.topic, doc.title)
                }
            })
            .collect();

        let recent_hotspots =
            Queries::get_recent_hotspots(conn, collection_id, 5).unwrap_or_default();

        let active_dirs =
            Queries::get_active_directives_for_paths(conn, collection_id, &recent_hotspots, max_directives)
                .unwrap_or_default();
        let active_directives: Vec<String> = active_dirs
            .iter()
            .map(|d| {
                let scope_tag = if d.is_global() {
                    "global".to_string()
                } else {
                    d.scope.join(",")
                };
                let summary = d
                    .content
                    .lines()
                    .map(|l| l.trim())
                    .find(|l| !l.is_empty() && !l.starts_with('#'))
                    .unwrap_or_else(|| {
                        d.content
                            .lines()
                            .map(|l| l.trim().trim_start_matches('#').trim())
                            .find(|l| !l.is_empty())
                            .unwrap_or(&d.enforcement)
                    });
                format!(
                    "[{}] **{}** ({}) - {}",
                    d.category.to_uppercase(),
                    d.title,
                    scope_tag,
                    summary
                )
            })
            .collect();

        let friction_warnings =
            Queries::get_recent_friction_warnings(conn, collection_id).unwrap_or_default();

        let knowledge_debt =
            Queries::get_recent_zero_hit_queries(conn, collection_id, 5).unwrap_or_default();

        let locked_critical_path = Self::resolve_locked_critical_path(conn, collection_id, grant_scope);

        let formatted_markdown = SessionBriefing::render_markdown(
            collection_id,
            &active_directives,
            &active_invariants,
            &active_risks,
            &recent_hotspots,
            &friction_warnings,
            &knowledge_debt,
            grant_scope,
            locked_critical_path.as_ref(),
        );

        Ok(SessionBriefing {
            collection_id: collection_id.to_string(),
            active_directives,
            active_invariants,
            active_risks,
            recent_hotspots,
            friction_warnings,
            knowledge_debt,
            grant_scope: grant_scope.map(|s| s.to_string()),
            locked_critical_path,
            formatted_markdown,
        })
    }

    pub fn resolve_locked_critical_path(
        conn: &Connection,
        collection_id: &str,
        grant_scope: Option<&str>,
    ) -> Option<CriticalPathLock> {
        let mut query = "SELECT d.path, d.title FROM documents d \
                         WHERE d.collection_id = ?1 AND d.kind = 'task' AND d.status = 'in_progress' AND d.is_tombstone = 0".to_string();
        if let Some(scope) = grant_scope {
            if scope.contains("projects/") {
                query.push_str(" AND d.path LIKE ?2");
            }
        }
        query.push_str(" ORDER BY d.id ASC LIMIT 1;");

        let lock: Option<(String, String)> = if let Some(scope) = grant_scope {
            if scope.contains("projects/") {
                let pattern = format!("%{}%", scope.trim_matches('*'));
                conn.query_row(&query, rusqlite::params![collection_id, pattern], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                }).ok()
            } else {
                conn.query_row(&query, rusqlite::params![collection_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                }).ok()
            }
        } else {
            conn.query_row(&query, rusqlite::params![collection_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }).ok()
        };

        if let Some((path, title)) = lock {
            let project = Self::extract_project_from_path(&path);
            let task_id = std::path::Path::new(&path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("task")
                .to_string();

            return Some(CriticalPathLock {
                project,
                task_id,
                task_title: title,
                constraint: "You are prohibited from refactoring other files or addressing adjacent bugs until this task passes verification.".to_string(),
            });
        }

        let status_query = "SELECT d.path, d.content FROM documents d \
                            WHERE d.collection_id = ?1 AND (d.path LIKE 'projects/%/status.md' OR d.path LIKE 'projects/%/STATUS.md') \
                              AND d.is_tombstone = 0 LIMIT 10;";
        if let Ok(mut stmt) = conn.prepare(status_query) {
            if let Ok(rows) = stmt.query_map([collection_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                for r in rows.flatten() {
                    let (path, content) = r;
                    let project = Self::extract_project_from_path(&path);
                    if let Ok((Some(status_doc), _)) = crate::domain::FrontmatterSplicer::parse::<crate::domain::StatusDocument>(&content) {
                        if let Some(active_task_id) = status_doc.active_task {
                            let task_title = conn.query_row(
                                "SELECT title FROM documents WHERE collection_id = ?1 AND (path LIKE ?2 OR source_id = ?3) LIMIT 1;",
                                rusqlite::params![collection_id, format!("%{}%", active_task_id), active_task_id],
                                |row| row.get::<_, String>(0),
                            ).unwrap_or_else(|_| format!("Task {}", active_task_id));

                            let is_blocked = status_doc.health == crate::domain::HealthState::Blocked;
                            let constraint = if is_blocked {
                                let blk_desc = status_doc.blockers.iter()
                                    .find(|b| !b.resolved)
                                    .map(|b| b.description.as_str())
                                    .unwrap_or("Active blocker reported");
                                format!("TASK IS CURRENTLY BLOCKED: {}. Do not thrash codebase; escalate or switch critical path to fix the blocker.", blk_desc)
                            } else {
                                "You are prohibited from refactoring other files or addressing adjacent bugs until this task passes verification.".to_string()
                            };

                            return Some(CriticalPathLock {
                                project,
                                task_id: active_task_id,
                                task_title,
                                constraint,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_project_from_path(path: &str) -> String {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 && parts[0] == "projects" {
            parts[1].to_string()
        } else {
            "default".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;

    #[test]
    fn test_session_manager_flow() {
        let db = Database::open_in_memory("coll_mgr", "prof_mgr").unwrap();
        let session = SessionManager::start_session(
            db.conn(),
            "coll_mgr",
            "prof_mgr",
            "agent_007",
            Some("grant_xyz".into()),
            Some("platform-shell".into()),
        )
        .expect("session starts");

        assert!(session.id.starts_with("sess_"));
        assert_eq!(session.agent_id, "agent_007");
        assert_eq!(session.project.as_deref(), Some("platform-shell"));

        SessionManager::record_tool_call(db.conn(), &session.id, "check_work", "src/auth.rs", "{}")
            .expect("record tool call");
        SessionManager::record_search(db.conn(), &session.id, "auth tokens", 3)
            .expect("record search");
        SessionManager::record_search(db.conn(), &session.id, "unknown concept", 0)
            .expect("record zero hit");

        let loop1 = SessionManager::record_file_edit(db.conn(), &session.id, "src/auth.rs", 25)
            .expect("record edit 1");
        assert!(!loop1);

        let loop2 = SessionManager::record_file_edit(db.conn(), &session.id, "src/auth.rs", 10)
            .expect("record edit 2");
        assert!(!loop2);

        // Third edit triggers oscillation loop
        let loop3 = SessionManager::record_file_edit(db.conn(), &session.id, "src/auth.rs", 5)
            .expect("record edit 3");
        assert!(loop3);

        SessionManager::record_risk_prevented(
            db.conn(),
            &session.id,
            "src/auth.rs",
            "Prevented plain text token leak",
        )
        .expect("record risk prevented");

        let scorecard =
            SessionManager::end_session(db.conn(), &session.id, "completed").expect("end session");

        assert_eq!(scorecard.session_id, session.id);
        assert_eq!(scorecard.review_loops_detected, 1);
        assert_eq!(scorecard.zero_hit_queries, vec!["unknown concept".to_string()]);
        assert_eq!(scorecard.risks_handled, 1);
        assert_eq!(scorecard.friction_hotspots, vec!["src/auth.rs".to_string()]);

        let briefing = SessionManager::generate_briefing(db.conn(), "coll_mgr", Some("src/**"))
            .expect("generate briefing");
        assert!(briefing.formatted_markdown.contains("src/auth.rs"));
    }
}
