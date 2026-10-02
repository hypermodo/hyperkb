use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileHygieneReport {
    pub file_path: String,
    pub added_lines: usize,
    pub comment_lines: usize,
    pub comment_ratio: f64,
    pub is_excessive: bool,
}

pub struct Git;

impl Git {
    /// Discovers staged files (in the Git index) using `git diff --cached --name-only -z`.
    pub fn staged_files<P: AsRef<Path>>(root: P) -> Result<Vec<String>, String> {
        let root = root.as_ref();
        let output = Command::new("git")
            .arg("diff")
            .arg("--cached")
            .arg("--name-only")
            .arg("-z")
            .current_dir(root)
            .output()
            .map_err(|e| format!("failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git diff --cached failed: {}", stderr.trim()));
        }

        Self::parse_null_terminated(&output.stdout)
    }

    /// Discovers modified files in the working directory using `git diff --name-only -z`.
    pub fn changed_files<P: AsRef<Path>>(root: P) -> Result<Vec<String>, String> {
        let root = root.as_ref();
        let output = Command::new("git")
            .arg("diff")
            .arg("--name-only")
            .arg("-z")
            .current_dir(root)
            .output()
            .map_err(|e| format!("failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git diff failed: {}", stderr.trim()));
        }

        Self::parse_null_terminated(&output.stdout)
    }

    /// Parses null-byte separated output from Git (-z flag).
    fn parse_null_terminated(bytes: &[u8]) -> Result<Vec<String>, String> {
        let mut files = Vec::new();
        for chunk in bytes.split(|&b| b == 0) {
            if !chunk.is_empty() {
                let s = String::from_utf8(chunk.to_vec())
                    .map_err(|e| format!("invalid UTF-8 in git output: {}", e))?;
                files.push(s);
            }
        }
        Ok(files)
    }

    /// Checks whether the diff for a target file consists exclusively of comments, docstrings, and whitespace.
    pub fn file_diff_is_trivial<P: AsRef<Path>>(
        root: P,
        rel_path: &str,
        staged: bool,
    ) -> Result<bool, String> {
        let root = root.as_ref();
        let mut cmd = Command::new("git");
        cmd.arg("diff");
        if staged {
            cmd.arg("--cached");
        }
        cmd.arg("-U0");
        cmd.arg("--");
        cmd.arg(rel_path);
        cmd.current_dir(root);

        let output = cmd.output().map_err(|e| format!("failed to run git diff: {}", e))?;
        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(Self::is_diff_text_trivial(&stdout))
    }

    /// Analyzes raw diff text to verify if all additions/deletions are comments or whitespace.
    pub fn is_diff_text_trivial(diff_text: &str) -> bool {
        let mut has_changes = false;
        for line in diff_text.lines() {
            // Skip diff control and file header lines
            if line.starts_with("---")
                || line.starts_with("+++")
                || line.starts_with("@@")
                || line.starts_with("diff ")
                || line.starts_with("index ")
            {
                continue;
            }

            if let Some(added) = line.strip_prefix('+') {
                has_changes = true;
                let trimmed = added.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if Self::is_comment_or_doc(trimmed) {
                    continue;
                }
                return false;
            }

            if let Some(removed) = line.strip_prefix('-') {
                has_changes = true;
                let trimmed = removed.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if Self::is_comment_or_doc(trimmed) {
                    continue;
                }
                return false;
            }
        }

        has_changes
    }

    fn is_comment_or_doc(trimmed: &str) -> bool {
        trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
            || trimmed.ends_with("*/")
            || trimmed.starts_with('#')
            || trimmed.starts_with("--")
            || trimmed.starts_with("<!--")
            || trimmed.starts_with("///")
            || trimmed.starts_with("//!")
            || trimmed.starts_with("\"\"\"")
            || trimmed.starts_with("'''")
    }

    /// Analyzes added lines in diff text to count total added lines vs comment lines.
    pub fn analyze_diff_comment_hygiene(diff_text: &str) -> (usize, usize) {
        let mut added_lines = 0;
        let mut comment_lines = 0;

        for line in diff_text.lines() {
            if line.starts_with("---")
                || line.starts_with("+++")
                || line.starts_with("@@")
                || line.starts_with("diff ")
                || line.starts_with("index ")
            {
                continue;
            }

            if let Some(added) = line.strip_prefix('+') {
                let trimmed = added.trim();
                if trimmed.is_empty() {
                    continue;
                }
                added_lines += 1;
                if Self::is_comment_or_doc(trimmed) {
                    comment_lines += 1;
                }
            }
        }

        (added_lines, comment_lines)
    }

    /// Evaluates comment hygiene on a specific file's diff.
    pub fn check_file_comment_hygiene<P: AsRef<Path>>(
        root: P,
        rel_path: &str,
        staged: bool,
    ) -> Result<Option<FileHygieneReport>, String> {
        let root = root.as_ref();
        let mut cmd = Command::new("git");
        cmd.arg("diff");
        if staged {
            cmd.arg("--cached");
        }
        cmd.arg("-U0");
        cmd.arg("--");
        cmd.arg(rel_path);
        cmd.current_dir(root);

        let output = cmd.output().map_err(|e| format!("failed to run git diff: {}", e))?;
        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let (added, comments) = Self::analyze_diff_comment_hygiene(&stdout);
        if added == 0 {
            return Ok(None);
        }

        let ratio = comments as f64 / added as f64;
        let is_excessive = ratio > 0.35 && comments >= 5;

        Ok(Some(FileHygieneReport {
            file_path: rel_path.to_string(),
            added_lines: added,
            comment_lines: comments,
            comment_ratio: ratio,
            is_excessive,
        }))
    }

    /// Discovers untracked files in the working directory using `git ls-files --others --exclude-standard -z`.
    pub fn untracked_files<P: AsRef<Path>>(root: P) -> Result<Vec<String>, String> {
        let root = root.as_ref();
        let output = Command::new("git")
            .arg("ls-files")
            .arg("--others")
            .arg("--exclude-standard")
            .arg("-z")
            .current_dir(root)
            .output()
            .map_err(|e| format!("failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git ls-files failed: {}", stderr.trim()));
        }

        Self::parse_null_terminated(&output.stdout)
    }

    /// Discovers all staged, modified, and untracked files in the repository working tree.
    pub fn get_modified_and_untracked_files<P: AsRef<Path>>(root: P) -> Result<Vec<String>, String> {
        let root = root.as_ref();
        let mut files = Vec::new();
        if let Ok(staged) = Self::staged_files(root) {
            files.extend(staged);
        }
        if let Ok(changed) = Self::changed_files(root) {
            files.extend(changed);
        }
        if let Ok(untracked) = Self::untracked_files(root) {
            files.extend(untracked);
        }
        files.sort();
        files.dedup();
        Ok(files)
    }

    /// Installs a pre-commit risk interception hook into `.git/hooks/pre-commit`.
    pub fn install_pre_commit_hook<P: AsRef<Path>>(root: P) -> Result<PathBuf, String> {
        let root = root.as_ref();
        let git_dir = root.join(".git");
        if !git_dir.exists() {
            return Err("not a git repository (.git folder not found)".to_string());
        }

        let hooks_dir = git_dir.join("hooks");
        fs::create_dir_all(&hooks_dir)
            .map_err(|e| format!("failed to create .git/hooks directory: {}", e))?;

        let hook_path = hooks_dir.join("pre-commit");

        // Determine current exe if available for seamless fallback across sibling repos
        let current_exe_snippet = if let Ok(exe_p) = std::env::current_exe() {
            if let Ok(canon) = exe_p.canonicalize() {
                format!("\nelif [ -x \"{}\" ]; then\n    \"{}\" -r \"$REPO_ROOT\" check-work --staged || exit 1", canon.display(), canon.display())
            } else {
                format!("\nelif [ -x \"{}\" ]; then\n    \"{}\" -r \"$REPO_ROOT\" check-work --staged || exit 1", exe_p.display(), exe_p.display())
            }
        } else {
            String::new()
        };

        let script = format!(r#"#!/bin/sh
# HyperKB Pre-Commit Risk Interception Hook
# Ensures both human developers and AI agents verify cited open risks before committing code.

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

if command -v hyperkb >/dev/null 2>&1; then
    hyperkb -r "$REPO_ROOT" check-work --staged || exit 1
elif [ -x "$REPO_ROOT/target/release/hyperkb" ]; then
    "$REPO_ROOT/target/release/hyperkb" -r "$REPO_ROOT" check-work --staged || exit 1
elif command -v hyperkb-rs >/dev/null 2>&1; then
    hyperkb-rs -r "$REPO_ROOT" check-work --staged || exit 1
elif [ -x "$REPO_ROOT/target/release/hyperkb-rs" ]; then
    "$REPO_ROOT/target/release/hyperkb-rs" -r "$REPO_ROOT" check-work --staged || exit 1{}
fi
"#, current_exe_snippet);

        fs::write(&hook_path, script.as_bytes())
            .map_err(|e| format!("failed to write pre-commit hook: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&hook_path)
                .map_err(|e| format!("failed to read hook metadata: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms)
                .map_err(|e| format!("failed to set executable permission on hook: {}", e))?;
        }

        Ok(hook_path)
    }

    /// Installs pre-commit hooks across the current repository and optionally all sibling git repositories.
    pub fn install_hooks_all<P: AsRef<Path>>(
        root: P,
        all_siblings: bool,
    ) -> Result<HookInstallReport, String> {
        let root = root.as_ref();
        let abs_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let mut installed = Vec::new();
        let mut skipped = Vec::new();

        // 1. Current repository
        match Self::install_pre_commit_hook(&abs_root) {
            Ok(p) => installed.push((abs_root.clone(), p)),
            Err(e) => skipped.push((abs_root.clone(), e)),
        }

        // 2. Siblings if requested
        if all_siblings {
            if let Some(parent) = abs_root.parent() {
                if let Ok(entries) = fs::read_dir(parent) {
                    let mut dirs: Vec<PathBuf> = entries
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.is_dir() && p != &abs_root)
                        .collect();
                    dirs.sort();

                    for dir in dirs {
                        if dir.join(".git").exists() {
                            match Self::install_pre_commit_hook(&dir) {
                                Ok(p) => installed.push((dir, p)),
                                Err(e) => skipped.push((dir, e)),
                            }
                        }
                    }
                }
            }
        }

        Ok(HookInstallReport { installed, skipped })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookInstallReport {
    pub installed: Vec<(PathBuf, PathBuf)>,
    pub skipped: Vec<(PathBuf, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_null_terminated() {
        let data = b"src/main.rs\0docs/decisions/test.md\0";
        let parsed = Git::parse_null_terminated(data).unwrap();
        assert_eq!(parsed, vec!["src/main.rs", "docs/decisions/test.md"]);
    }

    #[test]
    fn test_is_diff_text_trivial() {
        let comment_diff = r#"
diff --git a/src/storage/db.rs b/src/storage/db.rs
index e69de29..b6238b6 100644
--- a/src/storage/db.rs
+++ b/src/storage/db.rs
@@ -10,0 +11,3 @@
+// Note: WAL mode avoids reader-writer blocking.
+/// This is a doc comment
+   
"#;
        assert!(Git::is_diff_text_trivial(comment_diff));

        let code_diff = r#"
diff --git a/src/storage/db.rs b/src/storage/db.rs
index e69de29..b6238b6 100644
--- a/src/storage/db.rs
+++ b/src/storage/db.rs
@@ -10,0 +11,2 @@
+let timeout = 5000;
+conn.execute("PRAGMA busy_timeout = 5000;", [])?;
"#;
        assert!(!Git::is_diff_text_trivial(code_diff));
    }

    #[test]
    fn test_analyze_diff_comment_hygiene() {
        let bloat_diff = r#"
diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,3 +1,12 @@
+// First we initialize the token validator
+// This ensures the token has not expired
+// Then we check the signature against HMAC secret
+// We return an error if invalid
+// Now we create the claims struct
+let claims = verify_jwt(token)?;
+// Everything succeeded so we return ok
+Ok(claims)
"#;
        let (added, comments) = Git::analyze_diff_comment_hygiene(bloat_diff);
        assert_eq!(added, 8);
        assert_eq!(comments, 6);
        let ratio = comments as f64 / added as f64;
        assert!(ratio > 0.70);
    }

    #[test]
    fn test_install_hooks_all_siblings() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-hook-test-{}", uuid::Uuid::now_v7()));
        let repo_a = temp_dir.join("repo-a");
        let repo_b = temp_dir.join("repo-b");
        let non_git = temp_dir.join("non-git");

        fs::create_dir_all(repo_a.join(".git")).unwrap();
        fs::create_dir_all(repo_b.join(".git")).unwrap();
        fs::create_dir_all(&non_git).unwrap();

        let report = Git::install_hooks_all(&repo_a, true).unwrap();
        assert_eq!(report.installed.len(), 2);

        let hook_a = repo_a.join(".git/hooks/pre-commit");
        let hook_b = repo_b.join(".git/hooks/pre-commit");
        assert!(hook_a.exists());
        assert!(hook_b.exists());

        let content_a = fs::read_to_string(&hook_a).unwrap();
        assert!(content_a.contains("check-work --staged"));
        let content_b = fs::read_to_string(&hook_b).unwrap();
        assert!(content_b.contains("check-work --staged"));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
