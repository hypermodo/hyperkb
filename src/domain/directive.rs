use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Directive {
    pub id: String,
    pub collection_id: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub author: String,
    pub scope: Vec<String>,
    pub enforcement: String,
    pub supersedes: Option<String>,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DirectiveFrontmatter {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub author: String,
    pub scope: Vec<String>,
    pub enforcement: String,
    pub supersedes: Option<String>,
    pub created_at: String,
}

impl Directive {
    pub fn new(
        id: impl Into<String>,
        collection_id: impl Into<String>,
        title: impl Into<String>,
        category: impl Into<String>,
        author: impl Into<String>,
        scope: Vec<String>,
        enforcement: impl Into<String>,
        supersedes: Option<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            collection_id: collection_id.into(),
            title: title.into(),
            category: category.into().to_lowercase(),
            status: "active".to_string(),
            author: author.into(),
            scope,
            enforcement: enforcement.into(),
            supersedes,
            content: content.into(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn is_global(&self) -> bool {
        self.scope.is_empty() || self.scope.iter().any(|s| s == "*" || s == "**" || s == "**/*")
    }

    pub fn applies_to_path(&self, path: &str) -> bool {
        if self.is_global() {
            return true;
        }

        let normalized = path.replace('\\', "/");
        for pattern in &self.scope {
            let pat_norm = pattern.replace('\\', "/");
            if pat_norm == normalized {
                return true;
            }
            if pat_norm.ends_with("/**") {
                let prefix = &pat_norm[..pat_norm.len() - 3];
                if normalized.starts_with(prefix) {
                    return true;
                }
            } else if pat_norm.ends_with("/*") {
                let prefix = &pat_norm[..pat_norm.len() - 2];
                if normalized.starts_with(prefix) && !normalized[prefix.len() + 1..].contains('/') {
                    return true;
                }
            } else if pat_norm.starts_with("*.") {
                let ext = &pat_norm[1..];
                if normalized.ends_with(ext) {
                    return true;
                }
            }
        }

        false
    }

    /// Selects the most relevant directives for given target paths, enforcing the "Rule of 5"
    /// to strictly guard against prompt bloat.
    pub fn filter_relevant(
        directives: &[Directive],
        target_paths: &[String],
        max_limit: usize,
    ) -> Vec<Directive> {
        let mut active: Vec<&Directive> = directives
            .iter()
            .filter(|d| d.status == "active")
            .collect();

        if target_paths.is_empty() {
            active.sort_by(|a, b| {
                // Globals first, then newer
                b.is_global()
                    .cmp(&a.is_global())
                    .then_with(|| b.created_at.cmp(&a.created_at))
            });
            return active.into_iter().take(max_limit).cloned().collect();
        }

        let mut matched_specific = Vec::new();
        let mut globals = Vec::new();

        for d in active {
            let matches_specific = !d.is_global()
                && target_paths.iter().any(|p| d.applies_to_path(p));

            if matches_specific {
                matched_specific.push(d);
            } else if d.is_global() {
                globals.push(d);
            }
        }

        // Specific path matches take precedence, followed by top global directives
        let mut combined = Vec::new();
        for d in matched_specific {
            if combined.len() < max_limit {
                combined.push(d.clone());
            }
        }
        for d in globals {
            if combined.len() < max_limit {
                combined.push(d.clone());
            }
        }

        combined
    }

    pub fn to_markdown(&self) -> String {
        let frontmatter = DirectiveFrontmatter {
            id: self.id.clone(),
            kind: "directive".to_string(),
            title: self.title.clone(),
            category: self.category.clone(),
            status: self.status.clone(),
            author: self.author.clone(),
            scope: self.scope.clone(),
            enforcement: self.enforcement.clone(),
            supersedes: self.supersedes.clone(),
            created_at: self.created_at.clone(),
        };

        let yaml = serde_json::to_string_pretty(&frontmatter).unwrap_or_default();
        format!("---hyperkb\n{}\n---\n{}", yaml, self.content.trim_start())
    }

    pub fn parse_markdown(raw: &str, collection_id: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        if !trimmed.starts_with("---hyperkb") {
            return Err("Missing '---hyperkb' metadata block".to_string());
        }

        let rest = &trimmed["---hyperkb".len()..];
        let end_idx = rest.find("---").ok_or_else(|| "Unclosed frontmatter block".to_string())?;
        let frontmatter_str = rest[..end_idx].trim();
        let body = rest[end_idx + 3..].trim();

        let meta: DirectiveFrontmatter = serde_json::from_str(frontmatter_str)
            .map_err(|e| format!("Invalid directive frontmatter JSON: {}", e))?;

        Ok(Self {
            id: meta.id,
            collection_id: collection_id.to_string(),
            title: meta.title,
            category: meta.category.to_lowercase(),
            status: meta.status,
            author: meta.author,
            scope: meta.scope,
            enforcement: meta.enforcement,
            supersedes: meta.supersedes,
            content: body.to_string(),
            created_at: meta.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directive_roundtrip_markdown() {
        let dir = Directive::new(
            "DIR-001",
            "coll_1",
            "Zero Code Comments",
            "behavior",
            "wiqar",
            vec!["src/**".to_string()],
            "check_work",
            None,
            "# Zero Code Comments\nCode must be self-documenting.\n",
        );

        let md = dir.to_markdown();
        assert!(md.contains("---hyperkb"));
        assert!(md.contains("DIR-001"));
        assert!(md.contains("Zero Code Comments"));

        let parsed = Directive::parse_markdown(&md, "coll_1").expect("parsed");
        assert_eq!(parsed.id, "DIR-001");
        assert_eq!(parsed.category, "behavior");
        assert_eq!(parsed.scope, vec!["src/**".to_string()]);
        assert!(parsed.content.contains("Code must be self-documenting."));
    }

    #[test]
    fn test_applies_to_path() {
        let global = Directive::new(
            "DIR-G",
            "coll",
            "Prime Directive",
            "architecture",
            "wiqar",
            vec!["*".to_string()],
            "check_work",
            None,
            "rule",
        );
        assert!(global.is_global());
        assert!(global.applies_to_path("src/storage/db.rs"));
        assert!(global.applies_to_path("ui/index.html"));

        let scoped = Directive::new(
            "DIR-S",
            "coll",
            "Storage Busy Timeout",
            "architecture",
            "wiqar",
            vec!["src/storage/**".to_string()],
            "check_work",
            None,
            "rule",
        );
        assert!(!scoped.is_global());
        assert!(scoped.applies_to_path("src/storage/db.rs"));
        assert!(!scoped.applies_to_path("src/ui/app.rs"));
    }

    #[test]
    fn test_rule_of_five_filtering() {
        let mut directives = Vec::new();
        // 3 global directives
        for i in 1..=3 {
            directives.push(Directive::new(
                format!("DIR-G-{}", i),
                "coll",
                format!("Global {}", i),
                "behavior",
                "wiqar",
                vec!["*".to_string()],
                "check_work",
                None,
                "body",
            ));
        }

        // 4 scoped directives for storage
        for i in 1..=4 {
            directives.push(Directive::new(
                format!("DIR-STORE-{}", i),
                "coll",
                format!("Storage Rule {}", i),
                "architecture",
                "wiqar",
                vec!["src/storage/**".to_string()],
                "check_work",
                None,
                "body",
            ));
        }

        // Filter for storage path with limit 5
        let relevant = Directive::filter_relevant(&directives, &["src/storage/db.rs".to_string()], 5);
        assert_eq!(relevant.len(), 5);

        // All 4 storage rules must be included
        let storage_count = relevant.iter().filter(|d| d.id.starts_with("DIR-STORE")).count();
        assert_eq!(storage_count, 4);

        // Exactly 1 global rule fills the remaining capacity
        let global_count = relevant.iter().filter(|d| d.id.starts_with("DIR-G")).count();
        assert_eq!(global_count, 1);
    }
}
