use crate::core::decisions::DecisionWorkflow;
use crate::core::scanner::Scanner;
use crate::domain::RecordMeta;
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
}

#[cfg(test)]
mod tests {
    use super::*;
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
    }
}
