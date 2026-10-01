use crate::domain::{Directive, RepoManifest};
use crate::storage::Queries;
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectiveAuditReport {
    pub total_directives: usize,
    pub active_directives: usize,
    pub global_count: usize,
    pub stale_directives: Vec<String>,
    pub bloat_warnings: Vec<String>,
    pub dormant_directives: Vec<String>,
    pub taxonomies_used: Vec<String>,
}

pub struct DirectiveWorkflow;

impl DirectiveWorkflow {
    pub fn slugify(title: &str) -> String {
        let slug: String = title
            .chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect();
        slug.split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    pub fn draft_directive<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        title: &str,
        category: &str,
        author: &str,
        scope: Vec<String>,
        enforcement: &str,
        supersedes: Option<String>,
        content: &str,
    ) -> Result<Directive, String> {
        let root_path = root.as_ref();
        let manifest = RepoManifest::load_or_default(root_path);

        if !manifest.is_valid_category(category) {
            let valid_cats: Vec<String> = manifest
                .taxonomy
                .categories
                .iter()
                .map(|c| c.id.clone())
                .collect();
            return Err(format!(
                "Invalid category '{}'. Valid project taxonomy categories are: {}",
                category,
                valid_cats.join(", ")
            ));
        }

        let slug = Self::slugify(title);
        if slug.is_empty() {
            return Err("Directive title must contain alphanumeric characters".to_string());
        }

        let uuid = Uuid::now_v7();
        let hex = uuid.simple().to_string();
        let id = format!("DIR-{}", &hex[20..32].to_uppercase());
        let date_str = Utc::now().format("%Y-%m-%d").to_string();
        let file_name = format!("{}-{}-{}.md", date_str, slug, &hex[24..32]);
        let dir_path = root_path.join(&manifest.directives_path);

        let directive = Directive::new(
            &id,
            collection_id,
            title,
            category,
            author,
            scope,
            enforcement,
            supersedes.clone(),
            content,
        );

        let markdown = directive.to_markdown();
        fs::create_dir_all(&dir_path)
            .map_err(|e| format!("Failed to create directives directory: {}", e))?;

        let file_path = dir_path.join(file_name);
        fs::write(&file_path, markdown.as_bytes())
            .map_err(|e| format!("Failed to write directive markdown: {}", e))?;

        // If superseding an older directive, mark the old one in DB
        if let Some(ref old_id) = supersedes {
            let _ = Queries::supersede_directive(conn, old_id, &id);
        }

        Queries::upsert_directive(conn, &directive)
            .map_err(|e| format!("Failed to index directive into SQLite: {}", e))?;

        Ok(directive)
    }

    pub fn retire_directive<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        id: &str,
    ) -> Result<bool, String> {
        let found = Queries::retire_directive(conn, id)
            .map_err(|e| format!("Database error retiring directive: {}", e))?;

        if !found {
            return Ok(false);
        }

        // Also update file status on disk if present
        let root_path = root.as_ref();
        let manifest = RepoManifest::load_or_default(root_path);
        let dir_path = root_path.join(&manifest.directives_path);

        if dir_path.exists() {
            if let Ok(entries) = fs::read_dir(dir_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "md") {
                        if let Ok(raw) = fs::read_to_string(&path) {
                            if let Ok(mut d) = Directive::parse_markdown(&raw, &manifest.collection_id) {
                                if d.id == id {
                                    d.status = "retired".to_string();
                                    let _ = fs::write(&path, d.to_markdown().as_bytes());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(true)
    }

    pub fn audit_directives<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
    ) -> Result<DirectiveAuditReport, String> {
        let root_path = root.as_ref();
        let directives = Queries::list_directives(conn, collection_id, None, None)
            .map_err(|e| format!("Failed to list directives: {}", e))?;

        let total_directives = directives.len();
        let active: Vec<&Directive> = directives.iter().filter(|d| d.status == "active").collect();
        let active_directives = active.len();
        let global_count = active.iter().filter(|d| d.is_global()).count();

        let mut stale_directives = Vec::new();
        let mut bloat_warnings = Vec::new();
        let mut taxonomies_used = Vec::new();

        let manifest = crate::domain::RepoManifest::load_or_default(root_path);
        let max_rules = manifest.settings.max_briefing_directives;

        // 1. Check for Rule Bloat
        if global_count > max_rules {
            bloat_warnings.push(format!(
                "Found {} active global directives. Configured threshold is ≤ {} to prevent LLM prompt degradation. Consider scoping rules to specific path patterns.",
                global_count, max_rules
            ));
        }

        // 2. Check for Stale Scopes (scoped paths that do not exist on disk)
        for d in &active {
            if !taxonomies_used.contains(&d.category) {
                taxonomies_used.push(d.category.clone());
            }

            if !d.is_global() {
                let has_any_match = d.scope.iter().any(|pattern| {
                    let clean = pattern.trim_end_matches("/**").trim_end_matches("/*");
                    root_path.join(clean).exists()
                });

                if !has_any_match {
                    stale_directives.push(format!(
                        "{} ('{}') targets non-existent paths {:?}. Subsystem may have been moved or deleted.",
                        d.id, d.title, d.scope
                    ));
                }
            }
        }

        taxonomies_used.sort();

        Ok(DirectiveAuditReport {
            total_directives,
            active_directives,
            global_count,
            stale_directives,
            bloat_warnings,
            dormant_directives: Vec::new(),
            taxonomies_used,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[test]
    fn test_draft_and_retire_directive() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-dir-test-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);
        let db = Database::open_in_memory("coll_d", "prof_d").unwrap();

        let dir = DirectiveWorkflow::draft_directive(
            &temp_dir,
            db.conn(),
            "coll_d",
            "Zero Code Comments",
            "behavior",
            "wiqar",
            vec!["src/**".to_string()],
            "check_work",
            None,
            "# Zero Code Comments\nNo narration comments.\n",
        )
        .expect("draft directive");

        assert!(dir.id.starts_with("DIR-"));
        assert_eq!(dir.category, "behavior");

        let retrieved = Queries::get_directive(db.conn(), &dir.id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Zero Code Comments");

        let retired = DirectiveWorkflow::retire_directive(&temp_dir, db.conn(), &dir.id).unwrap();
        assert!(retired);

        let after = Queries::get_directive(db.conn(), &dir.id).unwrap().unwrap();
        assert_eq!(after.status, "retired");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_taxonomy_validation_rejects_bogus_category() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-dir-tax-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);
        let db = Database::open_in_memory("coll_d", "prof_d").unwrap();

        let err = DirectiveWorkflow::draft_directive(
            &temp_dir,
            db.conn(),
            "coll_d",
            "Bogus Rule",
            "nonexistent_category",
            "wiqar",
            vec!["*".to_string()],
            "check_work",
            None,
            "rule",
        );

        assert!(err.is_err());
        assert!(err.unwrap_err().contains("Invalid category 'nonexistent_category'"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_audit_bloat_and_stale_detection() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-dir-audit-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);
        let db = Database::open_in_memory("coll_audit", "prof_audit").unwrap();

        // Draft 6 global directives to trigger bloat warning (>5)
        for i in 1..=6 {
            DirectiveWorkflow::draft_directive(
                &temp_dir,
                db.conn(),
                "coll_audit",
                &format!("Global Rule {}", i),
                "behavior",
                "wiqar",
                vec!["*".to_string()],
                "check_work",
                None,
                "rule",
            )
            .unwrap();
        }

        // Draft 1 scoped directive to a non-existent path to trigger stale warning
        DirectiveWorkflow::draft_directive(
            &temp_dir,
            db.conn(),
            "coll_audit",
            "Legacy Subsystem Guard",
            "architecture",
            "wiqar",
            vec!["non_existent_folder/**".to_string()],
            "check_work",
            None,
            "rule",
        )
        .unwrap();

        let audit = DirectiveWorkflow::audit_directives(&temp_dir, db.conn(), "coll_audit").unwrap();
        assert_eq!(audit.total_directives, 7);
        assert_eq!(audit.global_count, 6);
        assert_eq!(audit.bloat_warnings.len(), 1);
        assert!(audit.bloat_warnings[0].contains("Found 6 active global directives"));
        assert_eq!(audit.stale_directives.len(), 1);
        assert!(audit.stale_directives[0].contains("Legacy Subsystem Guard"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
