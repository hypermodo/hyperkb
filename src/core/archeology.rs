use crate::core::risks::RiskWorkflow;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentCommit {
    pub sha: String,
    pub author: String,
    pub date: String,
    pub subject: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcheologyCandidate {
    pub title: String,
    pub affected_paths: Vec<String>,
    pub rationale: String,
    pub incident_count: usize,
    pub cited_commits: Vec<String>,
    pub created_path: Option<String>,
    pub created_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcheologyReport {
    pub analyzed_commits: usize,
    pub incident_commits: usize,
    pub candidates: Vec<ArcheologyCandidate>,
}

pub struct Archeology;

impl Archeology {
    /// Detects if a commit message indicates a historical incident, bug fix, regression, or vulnerability.
    pub fn is_incident_message(msg: &str) -> bool {
        let lower = msg.to_lowercase();
        let trimmed = lower.trim();

        // Conventional commit patterns
        if trimmed.starts_with("fix:")
            || trimmed.starts_with("fix(")
            || trimmed.starts_with("hotfix:")
            || trimmed.starts_with("hotfix(")
            || trimmed.starts_with("bug:")
            || trimmed.starts_with("bugfix:")
            || trimmed.starts_with("revert:")
            || trimmed.starts_with("revert(")
            || trimmed.starts_with("patch:")
        {
            return true;
        }

        // Keywords in subject line
        let keywords = [
            "regression",
            "race condition",
            "deadlock",
            "memory leak",
            "buffer overflow",
            "cve-",
            "vulnerability",
            "security advisory",
            "segfault",
            "panic",
            "fixes #",
            "resolves #",
            "closed #",
            "crash",
        ];

        keywords.iter().any(|&kw| lower.contains(kw))
    }

    /// Retrieves the current Git author name from `git config user.name`, falling back to env or "Developer".
    pub fn get_git_author<P: AsRef<Path>>(root: P) -> String {
        let output = Command::new("git")
            .arg("config")
            .arg("user.name")
            .current_dir(root)
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !name.is_empty() {
                    return name;
                }
            }
        }

        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "Developer".to_string())
    }

    /// Parses raw `git log` output formatted with field delimiters.
    pub fn parse_git_log_output(output: &str) -> (usize, Vec<IncidentCommit>) {
        let mut total_commits = 0;
        let mut incidents = Vec::new();

        let mut current_commit: Option<IncidentCommit> = None;

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(header) = line.strip_prefix("COMMIT\x1f") {
                total_commits += 1;
                // If we had a previous incident commit, push it
                if let Some(comm) = current_commit.take() {
                    if !comm.files.is_empty() {
                        incidents.push(comm);
                    }
                }

                let parts: Vec<&str> = header.split('\x1f').collect();
                if parts.len() >= 4 {
                    let sha = parts[0].to_string();
                    let author = parts[1].to_string();
                    let date = parts[2].to_string();
                    let subject = parts[3].to_string();

                    if Self::is_incident_message(&subject) {
                        current_commit = Some(IncidentCommit {
                            sha,
                            author,
                            date,
                            subject,
                            files: Vec::new(),
                        });
                    }
                }
            } else if let Some(ref mut comm) = current_commit {
                // File lines under the incident commit
                comm.files.push(line.to_string());
            }
        }

        if let Some(comm) = current_commit {
            if !comm.files.is_empty() {
                incidents.push(comm);
            }
        }

        (total_commits, incidents)
    }

    /// Synthesizes candidate risk policies from incident commits by grouping touched files.
    pub fn generate_candidates(
        incidents: &[IncidentCommit],
        limit: usize,
    ) -> Vec<ArcheologyCandidate> {
        let mut file_incidents: HashMap<String, Vec<&IncidentCommit>> = HashMap::new();

        for inc in incidents {
            for file in &inc.files {
                // Filter out non-code or trivial documentation metadata files if desired
                let lower = file.to_lowercase();
                if lower == ".gitignore" || lower == "license" {
                    continue;
                }
                file_incidents
                    .entry(file.clone())
                    .or_default()
                    .push(inc);
            }
        }

        let mut sorted_files: Vec<(String, Vec<&IncidentCommit>)> = file_incidents.into_iter().collect();
        // Sort by number of incident commits descending
        sorted_files.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        let mut candidates = Vec::new();

        for (file, commits) in sorted_files.into_iter().take(limit) {
            let count = commits.len();
            let short_file = Path::new(&file)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| file.clone());

            let title = format!("Incident Hotspot: {}", short_file);
            let mut cited_shas = Vec::new();

            let mut commit_bullet_points = String::new();
            for c in &commits {
                let short_sha = if c.sha.len() >= 8 { &c.sha[..8] } else { &c.sha };
                cited_shas.push(c.sha.clone());
                commit_bullet_points.push_str(&format!(
                    "- `{}` ({}, {}): {}\n",
                    short_sha, c.date, c.author, c.subject
                ));
            }

            let rationale = format!(
                "### Automated Git Archeology Incident Report\n\n\
                Automated Git history analysis (`hyperkb bootstrap`) identified `{}` as a recurring \
                incident hotspot across {} historical fix/regression commits.\n\n\
                #### Historical Incident Log:\n\
                {}\n\
                #### Architectural Boundary Invariant:\n\
                Prior to modifying this path:\n\
                1. Run `hyperkb check-work` to verify no active risks or uncommitted conflicts.\n\
                2. Verify regression test coverage for historical failure modes identified above.\n\
                3. If making structural or behavioral changes, draft an architecture decision (`hyperkb draft-decision`).\n",
                file, count, commit_bullet_points
            );

            candidates.push(ArcheologyCandidate {
                title,
                affected_paths: vec![file],
                rationale,
                incident_count: count,
                cited_commits: cited_shas,
                created_path: None,
                created_id: None,
            });
        }

        candidates
    }

    /// Performs Git archeology on the repository and drafts risk records.
    pub fn bootstrap<P: AsRef<Path>>(
        root: P,
        conn: &Connection,
        collection_id: &str,
        limit: usize,
        max_commits: usize,
        dry_run: bool,
        owner_override: Option<&str>,
    ) -> Result<ArcheologyReport, String> {
        let root = root.as_ref();

        let output = Command::new("git")
            .arg("log")
            .arg("--no-merges")
            .arg("-n")
            .arg(max_commits.to_string())
            .arg("--date=short")
            .arg("--pretty=format:COMMIT\x1f%H\x1f%an\x1f%ad\x1f%s")
            .arg("--name-only")
            .current_dir(root)
            .output()
            .map_err(|e| format!("failed to execute git log: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git log failed: {}", stderr.trim()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let (analyzed, incidents) = Self::parse_git_log_output(&stdout);

        let mut candidates = Self::generate_candidates(&incidents, limit);

        if !dry_run {
            let author = match owner_override {
                Some(o) if !o.trim().is_empty() => o.trim().to_string(),
                _ => Self::get_git_author(root),
            };

            for candidate in &mut candidates {
                let draft = RiskWorkflow::draft_risk(
                    root,
                    conn,
                    collection_id,
                    &candidate.title,
                    &candidate.rationale,
                    &author,
                    candidate.affected_paths.clone(),
                    vec![],
                    vec![],
                )?;
                candidate.created_path = Some(draft.path);
                candidate.created_id = Some(draft.id);
            }
        }

        Ok(ArcheologyReport {
            analyzed_commits: analyzed,
            incident_commits: incidents.len(),
            candidates,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[test]
    fn test_is_incident_message() {
        assert!(Archeology::is_incident_message("fix: resolve race condition in connection pool"));
        assert!(Archeology::is_incident_message("fix(auth): prevent token replay attack"));
        assert!(Archeology::is_incident_message("hotfix: patch memory leak on shutdown"));
        assert!(Archeology::is_incident_message("revert: restore previous locking behavior"));
        assert!(Archeology::is_incident_message("security: address CVE-2024-1234"));
        assert!(Archeology::is_incident_message("bugfix: handles panic when input is empty"));

        // Non-incidents
        assert!(!Archeology::is_incident_message("feat: add new user profile endpoint"));
        assert!(!Archeology::is_incident_message("docs: update README with installation steps"));
        assert!(!Archeology::is_incident_message("chore: bump dependency versions"));
    }

    #[test]
    fn test_parse_git_log_output_and_generate_candidates() {
        let raw = "\
COMMIT\x1faaa111\x1fAlice\x1f2026-09-28\x1ffix: resolve race in worker queue
src/worker.rs
src/queue.rs

COMMIT\x1fbbb222\x1fBob\x1f2026-09-29\x1ffea: add dashboard
src/ui/dashboard.rs

COMMIT\x1fccc333\x1fAlice\x1f2026-09-30\x1fhotfix: memory leak in worker
src/worker.rs
";
        let (total, incidents) = Archeology::parse_git_log_output(raw);
        assert_eq!(total, 3);
        assert_eq!(incidents.len(), 2);

        let candidates = Archeology::generate_candidates(&incidents, 5);
        assert_eq!(candidates.len(), 2); // worker.rs (2 incidents) and queue.rs (1 incident)

        // worker.rs should be the top incident hotspot
        assert_eq!(candidates[0].affected_paths, vec!["src/worker.rs"]);
        assert_eq!(candidates[0].incident_count, 2);
        assert!(candidates[0].rationale.contains("Automated Git Archeology"));
        assert!(candidates[0].cited_commits.contains(&"aaa111".to_string()));
        assert!(candidates[0].cited_commits.contains(&"ccc333".to_string()));
    }

    #[test]
    fn test_dry_run_bootstrap_workflow() {
        let _db = Database::open_in_memory("test_collection", "default").unwrap();

        let raw = "\
COMMIT\x1f11112222\x1fDeveloper\x1f2026-09-30\x1ffix: fix panic in parser
src/parser.rs
";
        let (_total, incidents) = Archeology::parse_git_log_output(raw);
        let candidates = Archeology::generate_candidates(&incidents, 1);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].title, "Incident Hotspot: parser.rs");
        assert!(candidates[0].created_path.is_none());
    }

    #[test]
    fn test_bootstrap_candidate_drafting() {
        use std::fs;
        use uuid::Uuid;
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-arch-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);

        let db = Database::open_in_memory("test_collection", "default").unwrap();

        let raw = "\
COMMIT\x1fa1b2c3d4\x1fEngineer\x1f2026-09-30\x1ffix(storage): resolve deadlock under high concurrency
src/storage/pool.rs
";
        let (_total, incidents) = Archeology::parse_git_log_output(raw);
        let candidates = Archeology::generate_candidates(&incidents, 1);
        assert_eq!(candidates.len(), 1);

        let candidate = &candidates[0];
        let draft = RiskWorkflow::draft_risk(
            &temp_dir,
            db.conn(),
            "test_collection",
            &candidate.title,
            &candidate.rationale,
            "Engineer",
            candidate.affected_paths.clone(),
            vec![],
            vec![],
        )
        .unwrap();

        assert!(draft.path.starts_with("docs/risks/"));
        assert!(temp_dir.join(&draft.path).exists());

        let content = fs::read_to_string(temp_dir.join(&draft.path)).unwrap();
        assert!(content.contains("Automated Git Archeology Incident Report"));
        assert!(content.contains("src/storage/pool.rs"));
        assert!(content.contains("a1b2c3d4"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
