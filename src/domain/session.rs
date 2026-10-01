use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub collection_id: String,
    pub profile_id: String,
    pub agent_id: String,
    pub grant_id: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub total_tool_calls: u32,
    pub total_edits: u32,
    pub total_diff_lines: u32,
    pub risks_cited: u32,
    pub risks_prevented: u32,
    pub review_loops: u32,
    pub first_pass_clean: bool,
    pub status: String,
}

impl AgentSession {
    pub fn new(
        id: impl Into<String>,
        collection_id: impl Into<String>,
        profile_id: impl Into<String>,
        agent_id: impl Into<String>,
        grant_id: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            collection_id: collection_id.into(),
            profile_id: profile_id.into(),
            agent_id: agent_id.into(),
            grant_id,
            started_at: Utc::now().to_rfc3339(),
            ended_at: None,
            total_tool_calls: 0,
            total_edits: 0,
            total_diff_lines: 0,
            risks_cited: 0,
            risks_prevented: 0,
            review_loops: 0,
            first_pass_clean: true,
            status: "active".to_string(),
        }
    }

    pub fn duration_seconds(&self) -> i64 {
        let start = DateTime::parse_from_rfc3339(&self.started_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let end = self
            .ended_at
            .as_deref()
            .and_then(|e| DateTime::parse_from_rfc3339(e).ok().map(|dt| dt.with_timezone(&Utc)))
            .unwrap_or_else(Utc::now);
        (end - start).num_seconds().max(0)
    }

    pub fn formatted_duration(&self) -> String {
        let start = DateTime::parse_from_rfc3339(&self.started_at)
            .map(|dt| dt.with_timezone(&Utc));
        let end = self
            .ended_at
            .as_deref()
            .and_then(|e| DateTime::parse_from_rfc3339(e).ok().map(|dt| dt.with_timezone(&Utc)));

        let is_running = self.ended_at.is_none() && self.status == "active";

        let (secs, millis) = match (start, end) {
            (Ok(s), Some(e)) => {
                let diff = e - s;
                (diff.num_seconds().max(0), diff.num_milliseconds().max(0))
            }
            (Ok(s), None) => {
                let diff = Utc::now() - s;
                (diff.num_seconds().max(0), diff.num_milliseconds().max(0))
            }
            _ => (0, 0),
        };

        let dur_str = if secs == 0 {
            if millis > 0 && millis < 1000 {
                format!("{}ms", millis)
            } else {
                "<1s".to_string()
            }
        } else if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            let mins = secs / 60;
            let rem_secs = secs % 60;
            format!("{}m {:02}s", mins, rem_secs)
        } else {
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            let rem_secs = secs % 60;
            format!("{}h {:02}m {:02}s", hours, mins, rem_secs)
        };

        if is_running {
            format!("{} (running)", dur_str)
        } else {
            dur_str
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventRecord {
    pub id: i64,
    pub session_id: String,
    pub timestamp: String,
    pub event_kind: String,
    pub target_path: String,
    pub query_or_tool: String,
    pub detail_json: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionScorecard {
    pub session_id: String,
    pub duration_seconds: u64,
    pub tool_to_edit_ratio: f32,
    pub coding_effectiveness: f32,
    pub knowledge_hit_rate: f32,
    pub zero_hit_queries: Vec<String>,
    pub review_loops_detected: u32,
    pub risks_handled: u32,
    pub friction_hotspots: Vec<String>,
}

impl SessionScorecard {
    pub fn compute(session: &AgentSession, events: &[SessionEventRecord]) -> Self {
        let start_time = DateTime::parse_from_rfc3339(&session.started_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let end_time = session
            .ended_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc)))
            .unwrap_or_else(Utc::now);

        let duration_seconds = (end_time - start_time).num_seconds().max(0) as u64;

        let tool_to_edit_ratio = if session.total_edits == 0 {
            session.total_tool_calls as f32
        } else {
            (session.total_tool_calls as f32 / session.total_edits as f32 * 10.0).round() / 10.0
        };

        // Path oscillation detection: paths edited or checked >= 3 times
        let mut path_counts: HashMap<String, usize> = HashMap::new();
        let mut searches_count: usize = 0;
        let mut zero_hit_queries = Vec::new();

        for ev in events {
            if !ev.target_path.is_empty() {
                *path_counts.entry(ev.target_path.clone()).or_insert(0) += 1;
            }
            if ev.event_kind == "search" || ev.event_kind == "zero_hit_query" {
                searches_count += 1;
                if ev.event_kind == "zero_hit_query" && !ev.query_or_tool.is_empty() {
                    zero_hit_queries.push(ev.query_or_tool.clone());
                }
            }
        }

        let mut friction_hotspots: Vec<String> = path_counts
            .into_iter()
            .filter(|(_, count)| *count >= 3)
            .map(|(path, _)| path)
            .collect();
        friction_hotspots.sort();

        let knowledge_hit_rate = if searches_count == 0 {
            1.0
        } else {
            let successful = searches_count.saturating_sub(zero_hit_queries.len());
            (successful as f32 / searches_count as f32 * 100.0).round() / 100.0
        };

        // Effectiveness Score calculation (0.0 to 1.0)
        let mut score: f32 = 1.0;
        if !session.first_pass_clean {
            score -= 0.15;
        }
        score -= (session.review_loops as f32) * 0.15;
        if session.total_tool_calls > 5 && tool_to_edit_ratio > 8.0 {
            score -= 0.20; // Tool thrashing penalty
        }
        if session.risks_prevented > 0 {
            score += 0.10; // Proactive risk compliance bonus
        }
        let coding_effectiveness = (score.clamp(0.05, 1.0) * 100.0).round() / 100.0;

        Self {
            session_id: session.id.clone(),
            duration_seconds,
            tool_to_edit_ratio,
            coding_effectiveness,
            knowledge_hit_rate,
            zero_hit_queries,
            review_loops_detected: session.review_loops,
            risks_handled: session.risks_prevented + session.risks_cited,
            friction_hotspots,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionBriefing {
    pub collection_id: String,
    #[serde(default)]
    pub active_directives: Vec<String>,
    pub active_invariants: Vec<String>,
    pub active_risks: Vec<String>,
    pub recent_hotspots: Vec<String>,
    pub friction_warnings: Vec<String>,
    pub knowledge_debt: Vec<String>,
    pub grant_scope: Option<String>,
    pub formatted_markdown: String,
}

impl SessionBriefing {
    pub fn render_markdown(
        collection_id: &str,
        active_directives: &[String],
        active_invariants: &[String],
        active_risks: &[String],
        recent_hotspots: &[String],
        friction_warnings: &[String],
        knowledge_debt: &[String],
        grant_scope: Option<&str>,
    ) -> String {
        let mut out = String::new();
        out.push_str(&format!("# HyperKB Session Briefing: `{}`\n", collection_id));

        if let Some(scope) = grant_scope {
            out.push_str(&format!("**Authorized Scope:** `{}`\n\n", scope));
        }

        if !active_directives.is_empty() {
            out.push_str("### Standing Directives (Rule of 5)\n");
            for d in active_directives {
                out.push_str(&format!("- {}\n", d));
            }
            out.push('\n');
        }

        if !friction_warnings.is_empty() {
            out.push_str("### Friction Hotspots & Review Warnings\n");
            for w in friction_warnings {
                out.push_str(&format!("- {}\n", w));
            }
            out.push('\n');
        }

        if !active_invariants.is_empty() {
            out.push_str("### Architectural Invariants (Accepted ADRs)\n");
            for inv in active_invariants {
                out.push_str(&format!("- {}\n", inv));
            }
            out.push('\n');
        }

        if !active_risks.is_empty() {
            out.push_str("### High-Priority Risks\n");
            for r in active_risks {
                out.push_str(&format!("- {}\n", r));
            }
            out.push('\n');
        }

        if !recent_hotspots.is_empty() {
            out.push_str("### Recent Touched Paths\n");
            for h in recent_hotspots {
                out.push_str(&format!("- `{}`\n", h));
            }
            out.push('\n');
        }

        if !knowledge_debt.is_empty() {
            out.push_str("### Knowledge Debt (Unanswered Queries)\n");
            for q in knowledge_debt {
                out.push_str(&format!("- \"{}\"\n", q));
            }
            out.push('\n');
        }

        out.push_str("> *HyperKB Zero-Ceremony Protocol: All context pre-warmed. Proceed directly to code.*\n");
        out
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_session_lifecycle_and_scorecard() {
        let mut session = AgentSession::new("sess_test1", "default", "default", "agent_antigravity", None);
        session.total_tool_calls = 6;
        session.total_edits = 2;
        session.total_diff_lines = 45;
        session.risks_prevented = 1;
        session.review_loops = 0;

        let events = vec![
            SessionEventRecord {
                id: 1,
                session_id: "sess_test1".to_string(),
                timestamp: "2026-10-01T00:00:00Z".to_string(),
                event_kind: "search".to_string(),
                target_path: "".to_string(),
                query_or_tool: "sqlite wal pragma".to_string(),
                detail_json: "{}".to_string(),
            },
            SessionEventRecord {
                id: 2,
                session_id: "sess_test1".to_string(),
                timestamp: "2026-10-01T00:01:00Z".to_string(),
                event_kind: "file_edit".to_string(),
                target_path: "src/storage/db.rs".to_string(),
                query_or_tool: "replace_file_content".to_string(),
                detail_json: "{}".to_string(),
            },
            SessionEventRecord {
                id: 3,
                session_id: "sess_test1".to_string(),
                timestamp: "2026-10-01T00:02:00Z".to_string(),
                event_kind: "file_edit".to_string(),
                target_path: "src/storage/db.rs".to_string(),
                query_or_tool: "replace_file_content".to_string(),
                detail_json: "{}".to_string(),
            },
            SessionEventRecord {
                id: 4,
                session_id: "sess_test1".to_string(),
                timestamp: "2026-10-01T00:03:00Z".to_string(),
                event_kind: "file_edit".to_string(),
                target_path: "src/storage/db.rs".to_string(),
                query_or_tool: "replace_file_content".to_string(),
                detail_json: "{}".to_string(),
            },
        ];

        let scorecard = SessionScorecard::compute(&session, &events);
        assert_eq!(scorecard.tool_to_edit_ratio, 3.0);
        assert!(scorecard.coding_effectiveness >= 0.9);
        assert_eq!(scorecard.knowledge_hit_rate, 1.0);
        assert_eq!(scorecard.friction_hotspots, vec!["src/storage/db.rs".to_string()]);
    }

    #[test]
    fn test_scorecard_penalizes_thrashing_and_loops() {
        let mut session = AgentSession::new("sess_thrash", "default", "default", "agent_slow", None);
        session.total_tool_calls = 25;
        session.total_edits = 1;
        session.review_loops = 3;
        session.first_pass_clean = false;

        let events = vec![SessionEventRecord {
            id: 1,
            session_id: "sess_thrash".to_string(),
            timestamp: "2026-10-01T00:00:00Z".to_string(),
            event_kind: "zero_hit_query".to_string(),
            target_path: "".to_string(),
            query_or_tool: "missing_concept".to_string(),
            detail_json: "{}".to_string(),
        }];

        let scorecard = SessionScorecard::compute(&session, &events);
        assert_eq!(scorecard.tool_to_edit_ratio, 25.0);
        // Thrashing penalty (-0.20), review loops (-0.45), first pass unclean (-0.15)
        assert!(scorecard.coding_effectiveness <= 0.3);
        assert_eq!(scorecard.zero_hit_queries, vec!["missing_concept".to_string()]);
        assert_eq!(scorecard.knowledge_hit_rate, 0.0);
    }

    #[test]
    fn test_render_session_briefing() {
        let md = SessionBriefing::render_markdown(
            "HyperModo",
            &["[BEHAVIOR] **Zero Code Comments**: check_work".to_string()],
            &["ADR-001: Zero Heavy Async (Stdlib Only)".to_string()],
            &["CRIT-01: Never run unsafe unverified sql".to_string()],
            &["src/core/git.rs".to_string()],
            &["src/core/git.rs was rewritten 3 times recently".to_string()],
            &["tokio runtime".to_string()],
            Some("src/core/**"),
        );

        assert!(md.contains("# HyperKB Session Briefing: `HyperModo`"));
        assert!(md.contains("**Authorized Scope:** `src/core/**`"));
        assert!(md.contains("Standing Directives (Rule of 5)"));
        assert!(md.contains("Zero Code Comments"));
        assert!(md.contains("Friction Hotspots & Review Warnings"));
        assert!(md.contains("ADR-001: Zero Heavy Async"));
        assert!(md.contains("CRIT-01: Never run unsafe unverified sql"));
        assert!(md.contains("tokio runtime"));
    }

    #[test]
    fn test_session_formatted_duration() {
        let mut session = AgentSession::new("s1", "col", "prof", "agent", None);
        session.started_at = "2026-10-01T12:00:00Z".to_string();
        session.ended_at = Some("2026-10-01T12:02:15Z".to_string());
        assert_eq!(session.formatted_duration(), "2m 15s");

        session.ended_at = Some("2026-10-01T12:00:45Z".to_string());
        assert_eq!(session.formatted_duration(), "45s");

        session.ended_at = Some("2026-10-01T13:05:08Z".to_string());
        assert_eq!(session.formatted_duration(), "1h 05m 08s");

        session.ended_at = Some("2026-10-01T12:00:00.250Z".to_string());
        assert_eq!(session.formatted_duration(), "250ms");
    }
}
