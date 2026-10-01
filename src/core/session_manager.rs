use crate::domain::{AgentSession, SessionBriefing, SessionScorecard};
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
    ) -> Result<AgentSession, String> {
        let id = format!("sess_{}", Uuid::now_v7());
        let session = AgentSession::new(&id, collection_id, profile_id, agent_id, grant_id.clone());
        Queries::create_session(conn, &session)
            .map_err(|e| format!("Failed to create session: {}", e))?;

        let detail = serde_json::json!({
            "agent_id": agent_id,
            "grant_id": grant_id
        })
        .to_string();

        Queries::record_session_event(conn, &id, "session_start", "", agent_id, &detail)
            .map_err(|e| format!("Failed to record session start event: {}", e))?;

        Ok(session)
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
            Queries::get_active_directives_for_paths(conn, collection_id, &recent_hotspots, 5)
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

        let formatted_markdown = SessionBriefing::render_markdown(
            collection_id,
            &active_directives,
            &active_invariants,
            &active_risks,
            &recent_hotspots,
            &friction_warnings,
            &knowledge_debt,
            grant_scope,
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
            formatted_markdown,
        })
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
        )
        .expect("session starts");

        assert!(session.id.starts_with("sess_"));
        assert_eq!(session.agent_id, "agent_007");

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
