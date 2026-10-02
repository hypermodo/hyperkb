use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessInitReport {
    pub file_path: String,
    pub created: bool,
    pub already_existed: bool,
    pub message: String,
}

pub struct HarnessInit;

impl HarnessInit {
    pub const THIN_POINTER_CONTENT: &'static str = r#"# Agent Governance & Architectural Boundaries

This repository is governed by HyperKB (Deterministic Governance & Knowledge Hub).

Before planning or executing changes:
1. Always run `check_work` via MCP (or CLI `hyperkb check-work <files>`) with planned file paths.
2. Adhere strictly to any returned Policy Directives (Rule of 5) and mitigate Cited Risks.
3. Discover architecture, schemas, and tasks via `browse` and `search` instead of guessing.
4. Record significant architectural decisions with `draft_decision` and hazards with `draft_risk`.

All code edits are mechanically verified on pre-commit via local Git hooks.
"#;

    pub fn init_harness<P: AsRef<Path>>(
        root: P,
        target_harness: &str,
        force: bool,
    ) -> Result<Vec<HarnessInitReport>, String> {
        let root = root.as_ref();
        let targets = match target_harness.to_lowercase().as_str() {
            "all" => vec!["AGENTS.md", "CLAUDE.md", "GEMINI.md", ".cursorrules"],
            "agents" | "universal" => vec!["AGENTS.md"],
            "claude" => vec!["CLAUDE.md"],
            "gemini" | "antigravity" => vec!["GEMINI.md"],
            "cursor" => vec![".cursorrules"],
            "copilot" => vec![".github/copilot-instructions.md"],
            other => return Err(format!(
                "Unknown harness target '{}'. Supported: all, agents, claude, gemini, cursor, copilot",
                other
            )),
        };

        let mut reports = Vec::new();

        for rel in targets {
            let path = root.join(rel);
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
                }
            }

            if path.exists() && !force {
                reports.push(HarnessInitReport {
                    file_path: rel.to_string(),
                    created: false,
                    already_existed: true,
                    message: format!("File '{}' already exists (use --force to overwrite)", rel),
                });
            } else {
                fs::write(&path, Self::THIN_POINTER_CONTENT.as_bytes())
                    .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
                reports.push(HarnessInitReport {
                    file_path: rel.to_string(),
                    created: true,
                    already_existed: path.exists(),
                    message: format!("Generated thin pointer at '{}'", rel),
                });
            }
        }

        Ok(reports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_init_harness_generates_thin_pointers() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-harness-init-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);

        let reports = HarnessInit::init_harness(&temp_dir, "all", false).unwrap();
        assert_eq!(reports.len(), 4);
        assert!(temp_dir.join("AGENTS.md").exists());
        assert!(temp_dir.join("CLAUDE.md").exists());
        assert!(temp_dir.join("GEMINI.md").exists());
        assert!(temp_dir.join(".cursorrules").exists());

        let content = fs::read_to_string(temp_dir.join("AGENTS.md")).unwrap();
        assert!(content.contains("HyperKB"));
        assert!(content.contains("check_work"));

        // Second run without force should not overwrite
        let reports_again = HarnessInit::init_harness(&temp_dir, "claude", false).unwrap();
        assert_eq!(reports_again.len(), 1);
        assert!(!reports_again[0].created);
        assert!(reports_again[0].already_existed);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
