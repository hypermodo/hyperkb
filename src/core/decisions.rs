use crate::core::metadata::MetadataParser;
use crate::core::scanner::Scanner;
use crate::domain::{DocumentKind, DocumentStatus, RecordMeta};
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
    /// It cannot self-ratify or supersede an accepted decision.
    pub fn draft_replacement<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        title: &str,
        rationale: &str,
        owner: &str,
        supersedes: Option<&str>,
    ) -> Result<DecisionDraft, String> {
        let title = title.trim();
        let rationale = rationale.trim();
        let owner = owner.trim();

        if title.is_empty() || rationale.is_empty() || owner.is_empty() {
            return Err("title, rationale, and owner declaration are required".to_string());
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

    /// Previews the acceptance diff for a proposed decision without committing the change to disk.
    pub fn review_acceptance<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        rel_path: &str,
        owner: &str,
        supersedes_override: Option<&str>,
    ) -> Result<DecisionReview, String> {
        let owner = owner.trim();
        if owner.is_empty() {
            return Err("owner declaration is required to accept a decision".to_string());
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
        meta.owner = owner.to_string();
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
    ) -> Result<(), String> {
        let root_path = root.as_ref();
        let full_path = root_path.join(&review.path);

        if !full_path.exists() {
            return Err(format!("decision file not found: {}", review.path));
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

        // 1. Draft a new decision
        let draft = DecisionWorkflow::draft_replacement(
            root,
            &conn,
            collection_id,
            "Adopt SQLite WAL Mode",
            "WAL mode enables concurrent non-blocking readers alongside one writer.",
            "Wiqar (Architect)",
            None,
        )
        .unwrap();

        assert_eq!(draft.status, "proposed");
        assert!(draft.path.starts_with("docs/decisions/"));
        assert!(root.join(&draft.path).exists());

        // 2. Review acceptance
        let review = DecisionWorkflow::review_acceptance(
            root,
            &conn,
            &draft.path,
            "Wiqar (Owner)",
            None,
        )
        .unwrap();

        assert!(review.before.contains("\"status\": \"proposed\""));
        assert!(review.after.contains("\"status\": \"accepted\""));
        assert!(review.after.contains("Wiqar (Owner)"));

        // 3. Accept decision
        DecisionWorkflow::accept_decision(root, &conn, collection_id, &review).unwrap();

        // 4. Verify on disk
        let saved = fs::read_to_string(root.join(&draft.path)).unwrap();
        assert!(saved.contains("\"status\": \"accepted\""));

        // 5. Query from database (indexed by Scanner)
        let doc = Queries::get_document(&conn, &draft.path).unwrap().unwrap();
        assert_eq!(doc.status, DocumentStatus::Accepted);
        assert_eq!(doc.kind, DocumentKind::Decision);
    }
}
