use crate::core::metadata::MetadataParser;
use crate::core::scanner::Scanner;
use crate::domain::{ActionKind, Actor, DocumentKind, DocumentStatus, RecordMeta};
use crate::storage::Queries;
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionDraft {
    pub path: String,
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionReview {
    pub path: String,
    pub before: String,
    pub after: String,
}

pub struct DecisionWorkflow;

impl DecisionWorkflow {
    pub fn slugify(title: &str) -> String {
        let mut slug = String::new();
        let mut last_dash = false;

        for c in title.to_ascii_lowercase().chars() {
            if c.is_ascii_alphanumeric() {
                slug.push(c);
                last_dash = false;
            } else if !last_dash && !slug.is_empty() {
                slug.push('-');
                last_dash = true;
            }
            if slug.len() > 50 {
                break;
            }
        }

        slug.trim_matches('-').to_string()
    }

    /// Drafts an untracked, explicitly proposed repo document for human review.
    /// It cannot self-ratify or supersede an accepted decision without explicit authority.
    pub fn draft_replacement<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        title: &str,
        rationale: &str,
        actor: &Actor,
        supersedes: Option<&str>,
    ) -> Result<DecisionDraft, String> {
        let title = title.trim();
        let rationale = rationale.trim();
        let owner = actor.responsible_owner().trim();

        if title.is_empty() || rationale.is_empty() || owner.is_empty() {
            return Err("title, rationale, and actor/owner declaration are required".to_string());
        }

        if !actor.can_perform(&ActionKind::ProposeDecision, "docs/decisions") {
            return Err(format!(
                "actor '{}' lacks authority to propose decisions",
                actor.name()
            ));
        }

        if title.contains('\r') || title.contains('\n') {
            return Err("decision title cannot contain line breaks".to_string());
        }

        let supersedes_cleaned = supersedes.and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        });

        if let Some(ref sup_id) = supersedes_cleaned {
            let doc_opt = Queries::get_document(conn, sup_id)
                .map_err(|e| format!("database query failed: {}", e))?;
            match doc_opt {
                Some(doc) => {
                    if doc.kind != DocumentKind::Decision || doc.status != DocumentStatus::Accepted {
                        return Err(format!(
                            "replacement must refer to a currently accepted decision (found {:?} {:?})",
                            doc.kind, doc.status
                        ));
                    }
                }
                None => {
                    return Err(format!(
                        "target decision to supersede '{}' not found",
                        sup_id
                    ));
                }
            }
        }

        let slug = Self::slugify(title);
        if slug.is_empty() {
            return Err("decision title must contain alphanumeric characters".to_string());
        }

        let id = Uuid::now_v7().to_string();
        let date_str = Utc::now().format("%Y-%m-%d").to_string();
        let file_name = format!("{}-{}-{}.md", date_str, slug, &id[..8]);
        let rel_path = format!("docs/decisions/{}", file_name);

        let meta = RecordMeta {
            id: id.clone(),
            kind: "decision".to_string(),
            status: "proposed".to_string(),
            owner: owner.to_string(),
            issue: None,
            paths: Vec::new(),
            versions: Vec::new(),
            environments: Vec::new(),
            supersedes: supersedes_cleaned,
            delegation: actor.delegation_meta(),
        };

        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("failed to serialize metadata: {}", e))?;

        let content = format!(
            "---hyperkb\n{}\n---\n# {}\n\n{}\n",
            meta_json, title, rationale
        );

        let root_path = root.as_ref();
        let full_path = root_path.join(&rel_path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create directory: {}", e))?;
        }

        if full_path.exists() {
            return Err(format!("destination path already exists: {}", rel_path));
        }

        fs::write(&full_path, content.as_bytes())
            .map_err(|e| format!("failed to write draft decision: {}", e))?;

        // Immediately index the newly drafted document so it's discoverable
        let _ = Scanner::index_directory(conn, root_path, collection_id);

        Ok(DecisionDraft {
            path: rel_path,
            id,
            status: "proposed".to_string(),
        })
    }

    /// Previews the acceptance diff for a proposed decision, validating actor authority.
    pub fn review_acceptance<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        rel_path: &str,
        actor: &Actor,
        supersedes_override: Option<&str>,
    ) -> Result<DecisionReview, String> {
        let owner = actor.responsible_owner().trim();
        if owner.is_empty() {
            return Err("owner declaration is required to accept a decision".to_string());
        }

        if !actor.can_perform(&ActionKind::AcceptDecision, rel_path) {
            return Err(format!(
                "actor '{}' lacks authority to accept decision at '{}'",
                actor.name(),
                rel_path
            ));
        }

        let root_path = root.as_ref();
        let full_path = root_path.join(rel_path);
        if !full_path.exists() {
            return Err(format!("decision file not found: {}", rel_path));
        }

        let current_content = fs::read_to_string(&full_path)
            .map_err(|e| format!("failed to read decision file: {}", e))?;

        let parsed = MetadataParser::parse(&current_content)?;
        let mut meta = match parsed.meta {
            Some(m) => m,
            None => return Err("file lacks valid HyperKB metadata block".to_string()),
        };

        if meta.kind != "decision" {
            return Err(format!("document is a '{}', not a decision", meta.kind));
        }

        if meta.status != "proposed" {
            return Err(format!(
                "only proposed decisions can be accepted (current status: '{}')",
                meta.status
            ));
        }

        let effective_supersedes = match supersedes_override {
            Some(s) if !s.trim().is_empty() => {
                let override_str = s.trim().to_string();
                if let Some(ref existing) = meta.supersedes {
                    if existing != &override_str {
                        return Err(
                            "supersedes override conflicts with draft; edit proposal before reviewing"
                                .to_string(),
                        );
                    }
                }
                Some(override_str)
            }
            _ => meta.supersedes.clone(),
        };

        if let Some(ref sup_id) = effective_supersedes {
            if let Actor::Agent { grant, .. } = actor {
                if !grant.constraints.allow_supersede {
                    return Err(
                        "authority grant does not permit superseding existing accepted decisions"
                            .to_string(),
                    );
                }
            }

            let doc_opt = Queries::get_document(conn, sup_id)
                .map_err(|e| format!("database query failed: {}", e))?;
            match doc_opt {
                Some(doc) => {
                    if doc.kind != DocumentKind::Decision || doc.status != DocumentStatus::Accepted {
                        return Err(format!(
                            "superseded source must be a currently accepted decision (found {:?} {:?})",
                            doc.kind, doc.status
                        ));
                    }
                }
                None => {
                    return Err(format!("superseded target decision '{}' not found", sup_id));
                }
            }
        }

        meta.status = "accepted".to_string();
        meta.owner = owner.to_string(); // Always the responsible human principal
        meta.delegation = actor.delegation_meta(); // Non-repudiable delegated provenance
        meta.supersedes = effective_supersedes;

        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("failed to serialize metadata: {}", e))?;

        // Preserve body byte-for-byte
        let after_content = format!("---hyperkb\n{}\n---\n{}", meta_json, parsed.body);

        Ok(DecisionReview {
            path: rel_path.to_string(),
            before: current_content,
            after: after_content,
        })
    }

    /// Atomically writes an owner-confirmed decision review to disk and refreshes the index.
    pub fn accept_decision<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        review: &DecisionReview,
        actor: Option<&Actor>,
    ) -> Result<(), String> {
        let root_path = root.as_ref();
        let full_path = root_path.join(&review.path);

        if !full_path.exists() {
            return Err(format!("decision file not found: {}", review.path));
        }

        if let Some(Actor::Agent { grant, .. }) = actor {
            if let Some(max_diff) = grant.constraints.max_line_diff {
                let before_lines: Vec<&str> = review.before.lines().collect();
                let after_lines: Vec<&str> = review.after.lines().collect();
                let diff_count = before_lines.iter().zip(after_lines.iter()).filter(|(a, b)| a != b).count()
                    + (before_lines.len() as isize - after_lines.len() as isize).unsigned_abs();
                if diff_count > max_diff {
                    return Err(format!(
                        "decision modifications ({} lines) exceed grant limit of {} lines",
                        diff_count, max_diff
                    ));
                }
            }
        }

        let current_content = fs::read_to_string(&full_path)
            .map_err(|e| format!("failed to read decision file: {}", e))?;

        if current_content != review.before {
            return Err(
                "decision file changed on disk since review was generated; review again".to_string(),
            );
        }

        let parsed = MetadataParser::parse(&review.after)?;
        let meta = parsed
            .meta
            .ok_or_else(|| "review content lacks valid metadata".to_string())?;

        if meta.kind != "decision" || meta.status != "accepted" {
            return Err("review does not produce a valid accepted decision".to_string());
        }

        let parent = full_path
            .parent()
            .ok_or_else(|| "invalid destination parent".to_string())?;

        let temp_file_name = format!(".hyperkb-decision-{}.part", Uuid::now_v7());
        let temp_path = parent.join(temp_file_name);

        fs::write(&temp_path, review.after.as_bytes())
            .map_err(|e| format!("failed to write temporary decision file: {}", e))?;

        if let Err(e) = fs::rename(&temp_path, &full_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!("failed to atomically replace decision file: {}", e));
        }

        let _ = Scanner::index_directory(conn, root_path, collection_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[test]
    fn test_slugify() {
        assert_eq!(
            DecisionWorkflow::slugify("Reject Stale Publish Snapshots!"),
            "reject-stale-publish-snapshots"
        );
        assert_eq!(
            DecisionWorkflow::slugify("  Use SQLite WAL Mode 2026  "),
            "use-sqlite-wal-mode-2026"
        );
    }

    #[test]
    fn test_draft_review_accept_flow() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-dec-test-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);
        let root = &temp_dir;
        let collection_id = "test-collection";
        let db = Database::open_in_memory(collection_id, "test-profile").unwrap();
        let conn = db.conn();

        // 1. Draft a new decision by human
        let human_actor = Actor::Human {
            username: "Wiqar (Architect)".into(),
        };
        let draft = DecisionWorkflow::draft_replacement(
            root,
            &conn,
            collection_id,
            "Adopt SQLite WAL Mode",
            "WAL mode enables concurrent non-blocking readers alongside one writer.",
            &human_actor,
            None,
        )
        .unwrap();

        assert_eq!(draft.status, "proposed");
        assert!(draft.path.starts_with("docs/decisions/"));
        assert!(root.join(&draft.path).exists());

        // 2. Review acceptance by human
        let review = DecisionWorkflow::review_acceptance(
            root,
            &conn,
            &draft.path,
            &human_actor,
            None,
        )
        .unwrap();

        assert!(review.before.contains("\"status\": \"proposed\""));
        assert!(review.after.contains("\"status\": \"accepted\""));
        assert!(review.after.contains("Wiqar (Architect)"));

        // 3. Accept decision
        DecisionWorkflow::accept_decision(root, &conn, collection_id, &review, Some(&human_actor)).unwrap();

        // 4. Verify on disk
        let saved = fs::read_to_string(root.join(&draft.path)).unwrap();
        assert!(saved.contains("\"status\": \"accepted\""));

        // 5. Query from database (indexed by Scanner)
        let doc = Queries::get_document(&conn, &draft.path).unwrap().unwrap();
        assert_eq!(doc.status, DocumentStatus::Accepted);
        assert_eq!(doc.kind, DocumentKind::Decision);

        // 6. Test Delegated Agent acceptance under an AuthorityGrant
        let grant = crate::domain::AuthorityGrant {
            grant_id: Uuid::now_v7(),
            grantee: "claude-3-7-sonnet".into(),
            granted_by: "wiqar".into(),
            allowed_actions: vec![ActionKind::ProposeDecision, ActionKind::AcceptDecision],
            allowed_scope_patterns: vec!["docs/decisions/**".into()],
            constraints: crate::domain::GrantConstraints::default(),
            created_at: Utc::now(),
            signature: None,
        };
        let agent = Actor::Agent {
            agent_id: "claude-3-7-sonnet".into(),
            model: "anthropic".into(),
            grant,
        };

        // Agent drafts decision
        let agent_draft = DecisionWorkflow::draft_replacement(
            root,
            &conn,
            collection_id,
            "Agent Delegated Decision",
            "Rationale drafted by agent delegate.",
            &agent,
            None,
        )
        .unwrap();

        // Agent reviews and accepts under grant
        let agent_review = DecisionWorkflow::review_acceptance(
            root,
            &conn,
            &agent_draft.path,
            &agent,
            None,
        )
        .unwrap();

        // Check that owner is WIQAR (human principal) and delegation contains agent info
        assert!(agent_review.after.contains("\"owner\": \"wiqar\""));
        assert!(agent_review.after.contains("\"agent_id\": \"claude-3-7-sonnet\""));

        DecisionWorkflow::accept_decision(root, &conn, collection_id, &agent_review, Some(&agent)).unwrap();
    }
}
