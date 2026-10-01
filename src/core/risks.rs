use crate::core::decisions::DecisionWorkflow;
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
pub struct RiskDraft {
    pub path: String,
    pub id: String,
    pub status: String,
    pub paths: Vec<String>,
}

pub struct RiskWorkflow;

impl RiskWorkflow {
    /// Drafts an open risk record citing affected file paths for owner review and pre-edit interception.
    pub fn draft_risk<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        title: &str,
        rationale: &str,
        owner: &str,
        affected_paths: Vec<String>,
        versions: Vec<String>,
        environments: Vec<String>,
    ) -> Result<RiskDraft, String> {
        let title = title.trim();
        let rationale = rationale.trim();
        let owner = owner.trim();

        if title.is_empty() || rationale.is_empty() || owner.is_empty() {
            return Err("title, rationale, and owner declaration are required".to_string());
        }

        if affected_paths.is_empty() {
            return Err("risk requires at least one affected file path pattern".to_string());
        }

        let slug = DecisionWorkflow::slugify(title);
        if slug.is_empty() {
            return Err("risk title must contain alphanumeric characters".to_string());
        }

        let id = Uuid::now_v7().to_string();
        let date_str = Utc::now().format("%Y-%m-%d").to_string();
        let file_name = format!("{}-{}-{}.md", date_str, slug, &id[..8]);
        let rel_path = format!("docs/risks/{}", file_name);

        let meta = RecordMeta {
            id: id.clone(),
            kind: "risk".to_string(),
            status: "open".to_string(),
            owner: owner.to_string(),
            issue: None,
            paths: affected_paths.clone(),
            versions,
            environments,
            supersedes: None,
            delegation: None,
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
            .map_err(|e| format!("failed to write draft risk: {}", e))?;

        // Re-index so the new risk is active immediately
        let _ = Scanner::index_directory(conn, root_path, collection_id);

        Ok(RiskDraft {
            path: rel_path,
            id,
            status: "open".to_string(),
            paths: affected_paths,
        })
    }

    /// Drafts an open risk record with explicit Actor accountability and delegation metadata.
    pub fn draft_risk_with_actor<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        title: &str,
        rationale: &str,
        actor: &Actor,
        affected_paths: Vec<String>,
        versions: Vec<String>,
        environments: Vec<String>,
    ) -> Result<RiskDraft, String> {
        let draft = Self::draft_risk(
            &root,
            conn,
            collection_id,
            title,
            rationale,
            actor.responsible_owner(),
            affected_paths,
            versions,
            environments,
        )?;

        if let Some(del) = actor.delegation_meta() {
            let full_path = root.as_ref().join(&draft.path);
            if let Ok(content) = fs::read_to_string(&full_path) {
                if let Ok(parsed) = MetadataParser::parse(&content) {
                    if let Some(mut meta) = parsed.meta {
                        meta.delegation = Some(del);
                        if let Ok(meta_json) = serde_json::to_string_pretty(&meta) {
                            let updated = format!("---hyperkb\n{}\n---\n{}", meta_json, parsed.body);
                            let _ = fs::write(&full_path, updated.as_bytes());
                            let _ = Scanner::index_directory(conn, root.as_ref(), collection_id);
                        }
                    }
                }
            }
        }

        Ok(draft)
    }

    /// Acknowledges an existing risk with recorded justification, transitioning its status to 'acknowledged'.
    /// An acknowledged risk remains documented but will no longer block pre-commit / check_work interception.
    pub fn acknowledge_risk<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        risk_id: &str,
        actor: &Actor,
        rationale: &str,
    ) -> Result<RiskDraft, String> {
        let rationale = rationale.trim();
        if rationale.is_empty() {
            return Err("rationale is required to acknowledge a risk".to_string());
        }

        let doc_opt = Queries::get_document(conn, risk_id)
            .map_err(|e| format!("database query failed: {}", e))?;
        let doc = doc_opt.ok_or_else(|| format!("target risk '{}' not found", risk_id))?;

        if doc.kind != DocumentKind::Risk {
            return Err(format!(
                "document '{}' is a '{}', not a risk",
                risk_id,
                doc.kind.as_str()
            ));
        }

        if !actor.can_perform(&ActionKind::AcknowledgeRisk, &doc.path) {
            return Err(format!(
                "actor '{}' lacks authority to acknowledge risk at '{}'",
                actor.name(),
                doc.path
            ));
        }

        let root_path = root.as_ref();
        let full_path = root_path.join(&doc.path);
        if !full_path.exists() {
            return Err(format!("risk file not found on disk: {}", doc.path));
        }

        let content = fs::read_to_string(&full_path)
            .map_err(|e| format!("failed to read risk file: {}", e))?;

        let parsed = MetadataParser::parse(&content)?;
        let mut meta = parsed
            .meta
            .ok_or_else(|| "file lacks valid HyperKB metadata block".to_string())?;

        meta.status = "acknowledged".to_string();
        meta.owner = actor.responsible_owner().to_string(); // Always the responsible human principal
        meta.delegation = actor.delegation_meta(); // Non-repudiable delegation audit trail

        let ack_section = format!(
            "\n\n## Risk Acknowledgment\n- **Acknowledged by**: {}\n- **Date**: {}\n- **Rationale**: {}\n",
            actor.responsible_owner(),
            Utc::now().to_rfc3339(),
            rationale
        );

        let updated_body = format!("{}{}", parsed.body.trim_end(), ack_section);
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("failed to serialize metadata: {}", e))?;
        let updated_file = format!("---hyperkb\n{}\n---\n{}", meta_json, updated_body);

        let temp_file_name = format!(".hyperkb-risk-{}.part", Uuid::now_v7());
        let parent = full_path
            .parent()
            .ok_or_else(|| "invalid destination parent".to_string())?;
        let temp_path = parent.join(temp_file_name);

        fs::write(&temp_path, updated_file.as_bytes())
            .map_err(|e| format!("failed to write temporary risk file: {}", e))?;

        if let Err(e) = fs::rename(&temp_path, &full_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!("failed to atomically replace risk file: {}", e));
        }

        let _ = Scanner::index_directory(conn, root_path, collection_id);

        Ok(RiskDraft {
            path: doc.path,
            id: doc.id,
            status: "acknowledged".to_string(),
            paths: meta.paths,
        })
    }

    /// Resolves an open or acknowledged risk citing an accepted mitigating decision.
    pub fn resolve_risk<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        risk_id: &str,
        actor: &Actor,
        resolving_decision_id: &str,
    ) -> Result<RiskDraft, String> {
        let resolving_decision_id = resolving_decision_id.trim();
        if resolving_decision_id.is_empty() {
            return Err("resolving decision ID is required".to_string());
        }

        let dec_opt = Queries::get_document(conn, resolving_decision_id)
            .map_err(|e| format!("database query failed: {}", e))?;
        let dec = dec_opt
            .ok_or_else(|| format!("resolving decision '{}' not found", resolving_decision_id))?;
        if dec.kind != DocumentKind::Decision || dec.status != DocumentStatus::Accepted {
            return Err(format!(
                "resolving document must be an accepted decision (found {:?} {:?})",
                dec.kind, dec.status
            ));
        }

        let doc_opt = Queries::get_document(conn, risk_id)
            .map_err(|e| format!("database query failed: {}", e))?;
        let doc = doc_opt.ok_or_else(|| format!("target risk '{}' not found", risk_id))?;

        if doc.kind != DocumentKind::Risk {
            return Err(format!(
                "document '{}' is a '{}', not a risk",
                risk_id,
                doc.kind.as_str()
            ));
        }

        if !actor.can_perform(&ActionKind::AcknowledgeRisk, &doc.path) {
            return Err(format!(
                "actor '{}' lacks authority to resolve risk at '{}'",
                actor.name(),
                doc.path
            ));
        }

        let root_path = root.as_ref();
        let full_path = root_path.join(&doc.path);
        let content = fs::read_to_string(&full_path)
            .map_err(|e| format!("failed to read risk file: {}", e))?;

        let parsed = MetadataParser::parse(&content)?;
        let mut meta = parsed
            .meta
            .ok_or_else(|| "file lacks valid HyperKB metadata block".to_string())?;

        meta.status = "resolved".to_string();
        meta.owner = actor.responsible_owner().to_string();
        meta.delegation = actor.delegation_meta();
        meta.supersedes = Some(resolving_decision_id.to_string());

        let res_section = format!(
            "\n\n## Risk Resolution\n- **Resolved by**: {}\n- **Date**: {}\n- **Mitigating Decision**: {} ({})\n",
            actor.responsible_owner(),
            Utc::now().to_rfc3339(),
            dec.title,
            resolving_decision_id
        );

        let updated_body = format!("{}{}", parsed.body.trim_end(), res_section);
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("failed to serialize metadata: {}", e))?;
        let updated_file = format!("---hyperkb\n{}\n---\n{}", meta_json, updated_body);

        let temp_file_name = format!(".hyperkb-risk-{}.part", Uuid::now_v7());
        let parent = full_path
            .parent()
            .ok_or_else(|| "invalid destination parent".to_string())?;
        let temp_path = parent.join(temp_file_name);

        fs::write(&temp_path, updated_file.as_bytes())
            .map_err(|e| format!("failed to write temporary risk file: {}", e))?;

        if let Err(e) = fs::rename(&temp_path, &full_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!("failed to atomically replace risk file: {}", e));
        }

        let _ = Scanner::index_directory(conn, root_path, collection_id);

        Ok(RiskDraft {
            path: doc.path,
            id: doc.id,
            status: "resolved".to_string(),
            paths: meta.paths,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::decisions::DecisionWorkflow;
    use crate::storage::Database;
    use crate::storage::Queries;

    #[test]
    fn test_draft_risk_flow() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-risk-test-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);
        let root = &temp_dir;

        let collection_id = "test-collection";
        let db = Database::open_in_memory(collection_id, "test-profile").unwrap();

        let draft = RiskWorkflow::draft_risk(
            root,
            db.conn(),
            collection_id,
            "SQLite Busy Lock Contention",
            "Concurrent writers may starve readers unless WAL timeout is set to at least 5000ms.",
            "Wiqar (Architect)",
            vec!["src/storage/**".to_string()],
            vec!["v2.0".to_string()],
            vec!["production".to_string()],
        )
        .unwrap();

        assert_eq!(draft.status, "open");
        assert!(draft.path.starts_with("docs/risks/"));
        assert!(root.join(&draft.path).exists());

        // Verify risk interception works for this path
        let check = Queries::check_work(
            db.conn(),
            collection_id,
            &["src/storage/db.rs".to_string()],
            Some("v2.0"),
            Some("production"),
        )
        .unwrap();

        assert_eq!(check.matches.len(), 1);
        assert_eq!(check.matches[0].document.title, "SQLite Busy Lock Contention");
        assert!(!check.matches[0].acknowledged);
        assert!(check.has_open_risks());

        // Now test acknowledge_risk with human actor
        let human = Actor::Human {
            username: "wiqar".into(),
        };
        let ack_draft = RiskWorkflow::acknowledge_risk(
            root,
            db.conn(),
            collection_id,
            &draft.id,
            &human,
            "Set busy_timeout to 10000ms in DB connection pool.",
        )
        .unwrap();

        assert_eq!(ack_draft.status, "acknowledged");

        // Verify check_work reflects acknowledged status and no longer blocks has_open_risks()
        let check_after = Queries::check_work(
            db.conn(),
            collection_id,
            &["src/storage/db.rs".to_string()],
            Some("v2.0"),
            Some("production"),
        )
        .unwrap();

        assert_eq!(check_after.matches.len(), 1);
        assert!(check_after.matches[0].acknowledged);
        assert!(!check_after.has_open_risks());

        // Now test resolving risk citing an accepted decision
        let dec_draft = DecisionWorkflow::draft_replacement(
            root,
            db.conn(),
            collection_id,
            "Fix SQLite Busy Lock with Pool",
            "Added robust pool with WAL mode.",
            &human,
            None,
        )
        .unwrap();
        let dec_review = DecisionWorkflow::review_acceptance(
            root,
            db.conn(),
            &dec_draft.path,
            &human,
            None,
        )
        .unwrap();
        DecisionWorkflow::accept_decision(root, db.conn(), collection_id, &dec_review, Some(&human)).unwrap();

        let resolved = RiskWorkflow::resolve_risk(
            root,
            db.conn(),
            collection_id,
            &draft.id,
            &human,
            &dec_draft.id,
        )
        .unwrap();

        assert_eq!(resolved.status, "resolved");

        // Resolved risk should not be retrieved by get_open_risks at all
        let check_resolved = Queries::check_work(
            db.conn(),
            collection_id,
            &["src/storage/db.rs".to_string()],
            Some("v2.0"),
            Some("production"),
        )
        .unwrap();

        assert_eq!(check_resolved.matches.len(), 0);
    }
}
