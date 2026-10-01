use crate::core::metadata::MetadataParser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KbAuditReport {
    pub total_documents: usize,
    pub valid_documents: usize,
    pub bloat_warnings: Vec<String>,
    pub schema_errors: Vec<String>,
    pub depth_warnings: Vec<String>,
    pub stale_warnings: Vec<String>,
    pub topics: Vec<String>,
}

impl KbAuditReport {
    pub fn is_clean(&self) -> bool {
        self.bloat_warnings.is_empty()
            && self.schema_errors.is_empty()
            && self.depth_warnings.is_empty()
            && self.stale_warnings.is_empty()
    }
}

pub struct KbLinter;

impl KbLinter {
    pub const MAX_LINE_COUNT: usize = 250;
    pub const HARD_MAX_LINE_COUNT: usize = 500;
    pub const MAX_WORD_COUNT: usize = 2000;
    pub const MAX_FOLDER_DEPTH: usize = 3;

    pub fn audit_directory<P: AsRef<Path>>(docs_dir: P) -> Result<KbAuditReport, String> {
        Self::audit_directory_with_settings(docs_dir, Self::MAX_LINE_COUNT, Self::MAX_FOLDER_DEPTH, 90)
    }

    pub fn audit_directory_with_settings<P: AsRef<Path>>(
        docs_dir: P,
        max_lines: usize,
        max_depth: usize,
        stale_days: i64,
    ) -> Result<KbAuditReport, String> {
        let docs_path = docs_dir.as_ref();
        if !docs_path.exists() {
            return Ok(KbAuditReport {
                total_documents: 0,
                valid_documents: 0,
                bloat_warnings: Vec::new(),
                schema_errors: Vec::new(),
                depth_warnings: Vec::new(),
                stale_warnings: Vec::new(),
                topics: Vec::new(),
            });
        }

        let mut md_files = Vec::new();
        Self::collect_markdown_files(docs_path, &mut md_files)?;

        let total_documents = md_files.len();
        let mut valid_documents = 0;
        let mut bloat_warnings = Vec::new();
        let mut schema_errors = Vec::new();
        let mut depth_warnings = Vec::new();
        let mut stale_warnings = Vec::new();
        let mut topics = Vec::new();

        for file in &md_files {
            let rel_path = file.strip_prefix(docs_path).unwrap_or(file);
            let rel_str = rel_path.to_string_lossy().to_string();

            if let Some(parent) = rel_path.parent() {
                let topic = parent.to_string_lossy().to_string();
                if !topic.is_empty() && !topics.contains(&topic) {
                    topics.push(topic);
                }
            }

            // 1. Nesting Depth Check
            let depth = rel_path.components().count().saturating_sub(1); // exclude filename
            if depth > max_depth {
                depth_warnings.push(format!(
                    "{} is nested {} directories deep (threshold: ≤ {}). Avoid deep folder hierarchies.",
                    rel_str, depth, max_depth
                ));
            }

            let content = match fs::read_to_string(file) {
                Ok(c) => c,
                Err(e) => {
                    schema_errors.push(format!("{}: failed to read file: {}", rel_str, e));
                    continue;
                }
            };

            let mut has_issue = false;

            // 2. Length & Bloat Check
            let line_count = content.lines().count();
            let word_count = content.split_whitespace().count();

            if line_count > Self::HARD_MAX_LINE_COUNT {
                bloat_warnings.push(format!(
                    "{} exceeds hard length ceiling: {} lines (limit: {} lines). Critical LLM bloat detected.",
                    rel_str, line_count, Self::HARD_MAX_LINE_COUNT
                ));
                has_issue = true;
            } else if line_count > max_lines {
                bloat_warnings.push(format!(
                    "{} is {} lines long (configured target: ≤ {} lines). Consider breaking into focused documents.",
                    rel_str, line_count, max_lines
                ));
            }

            if word_count > Self::MAX_WORD_COUNT {
                bloat_warnings.push(format!(
                    "{} contains {} words (recommended target: ≤ {} words). Trim unnecessary narrative fluff.",
                    rel_str, word_count, Self::MAX_WORD_COUNT
                ));
            }

            // 3. Schema & Frontmatter Validation
            match MetadataParser::parse(&content) {
                Ok(parsed) => {
                    if let Some(ref meta) = parsed.meta {
                        // Check for stale proposed status
                        if meta.status == "proposed" || meta.status == "draft" {
                            if let Ok(metadata) = fs::metadata(file) {
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(elapsed) = modified.elapsed() {
                                        let days = (elapsed.as_secs() / 86400) as i64;
                                        if days > stale_days {
                                            stale_warnings.push(format!(
                                                "{} ('{}') has remained in '{}' status for {} days (configured threshold: {} days) without review or acceptance.",
                                                rel_str, parsed.title, meta.status, days, stale_days
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        schema_errors.push(format!(
                            "{}: missing '---hyperkb' metadata header block.",
                            rel_str
                        ));
                        has_issue = true;
                    }
                }
                Err(e) => {
                    schema_errors.push(format!("{}: invalid frontmatter schema: {}", rel_str, e));
                    has_issue = true;
                }
            }

            if !has_issue {
                valid_documents += 1;
            }
        }

        topics.sort();

        Ok(KbAuditReport {
            total_documents,
            valid_documents,
            bloat_warnings,
            schema_errors,
            depth_warnings,
            stale_warnings,
            topics,
        })
    }

    pub fn lint_single_file<P: AsRef<Path>, D: AsRef<Path>>(
        file_path: P,
        docs_dir: D,
    ) -> Result<Vec<String>, String> {
        let file = file_path.as_ref();
        let docs = docs_dir.as_ref();

        if !file.exists() {
            return Ok(Vec::new());
        }

        let rel_path = file.strip_prefix(docs).unwrap_or(file);
        let rel_str = rel_path.to_string_lossy().to_string();
        let mut warnings = Vec::new();

        // 1. Depth
        let depth = rel_path.components().count().saturating_sub(1);
        if depth > Self::MAX_FOLDER_DEPTH {
            warnings.push(format!(
                "Folder depth warning: {} is nested {} levels deep (limit: ≤ {}).",
                rel_str, depth, Self::MAX_FOLDER_DEPTH
            ));
        }

        let content = fs::read_to_string(file).map_err(|e| e.to_string())?;

        // 2. Length
        let line_count = content.lines().count();
        if line_count > Self::HARD_MAX_LINE_COUNT {
            warnings.push(format!(
                "Critical bloat: {} is {} lines long (hard ceiling: {} lines).",
                rel_str, line_count, Self::HARD_MAX_LINE_COUNT
            ));
        } else if line_count > Self::MAX_LINE_COUNT {
            warnings.push(format!(
                "Document length warning: {} is {} lines long (target: ≤ {} lines).",
                rel_str, line_count, Self::MAX_LINE_COUNT
            ));
        }

        // 3. Schema
        match MetadataParser::parse(&content) {
            Ok(parsed) => {
                if parsed.meta.is_none() {
                    warnings.push(format!("{}: missing '---hyperkb' metadata header.", rel_str));
                }
            }
            Err(e) => {
                warnings.push(format!("{}: invalid frontmatter: {}", rel_str, e));
            }
        }

        Ok(warnings)
    }

    fn collect_markdown_files(dir: &Path, acc: &mut Vec<PathBuf>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Ignore hidden directories like .git
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') {
                        continue;
                    }
                }
                Self::collect_markdown_files(&path, acc)?;
            } else if path.extension().is_some_and(|ext| ext == "md") {
                acc.push(path);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use uuid::Uuid;

    #[test]
    fn test_kb_linter_detects_bloat_and_schema_issues() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-linter-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);

        // 1. Clean document
        let clean_path = temp_dir.join("clean.md");
        let mut f1 = fs::File::create(&clean_path).unwrap();
        writeln!(
            f1,
            r#"---hyperkb
{{
  "id": "dec_2026_001",
  "kind": "decision",
  "status": "accepted",
  "owner": "Developer"
}}
---
# Architectural Decision 1
Short and concise decision rationale.
"#
        )
        .unwrap();

        // 2. Bloated document (> 250 lines)
        let bloat_path = temp_dir.join("bloat.md");
        let mut f2 = fs::File::create(&bloat_path).unwrap();
        writeln!(
            f2,
            r#"---hyperkb
{{
  "id": "dec_2026_002",
  "kind": "decision",
  "status": "accepted",
  "owner": "Developer"
}}
---
# Verbose Document
"#
        )
        .unwrap();
        for i in 0..260 {
            writeln!(f2, "Paragraph line {} of redundant explanation.", i).unwrap();
        }

        // 3. Document without frontmatter
        let no_header_path = temp_dir.join("no_header.md");
        let mut f3 = fs::File::create(&no_header_path).unwrap();
        writeln!(f3, "# Plain markdown with no metadata").unwrap();

        // 4. Overly deep folder
        let deep_dir = temp_dir.join("a").join("b").join("c").join("d");
        fs::create_dir_all(&deep_dir).unwrap();
        let deep_path = deep_dir.join("deep.md");
        let mut f4 = fs::File::create(&deep_path).unwrap();
        writeln!(
            f4,
            r#"---hyperkb
{{
  "id": "dec_2026_004",
  "kind": "decision",
  "status": "accepted",
  "owner": "Developer"
}}
---
# Deep Doc
"#
        )
        .unwrap();

        let report = KbLinter::audit_directory(&temp_dir).unwrap();
        assert_eq!(report.total_documents, 4);
        assert_eq!(report.bloat_warnings.len(), 1);
        assert!(report.bloat_warnings[0].contains("bloat.md is") && report.bloat_warnings[0].contains("lines long"));
        assert_eq!(report.schema_errors.len(), 1);
        assert!(report.schema_errors[0].contains("no_header.md: missing '---hyperkb'"));
        assert_eq!(report.depth_warnings.len(), 1);
        assert!(report.depth_warnings[0].contains("nested 4 directories deep"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
