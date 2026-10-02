use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use crate::domain::frontmatter::FrontmatterSplicer;
use crate::domain::schema::{
    HealthState, StatusDocument, StatusState, TaskDocument, TaskState,
};

pub struct StatusEngine;

impl StatusEngine {
    pub fn find_status_file(project_dir: &Path) -> Option<PathBuf> {
        let candidates = ["status.md", "STATUS.md", "status.markdown", "STATUS.markdown"];
        for c in &candidates {
            let p = project_dir.join(c);
            if p.is_file() {
                return Some(p);
            }
        }
        None
    }

    pub fn ensure_status_file(project_dir: &Path, project_name: &str) -> Result<PathBuf, String> {
        if let Some(existing) = Self::find_status_file(project_dir) {
            return Ok(existing);
        }

        fs::create_dir_all(project_dir)
            .map_err(|e| format!("Failed to create project directory '{}': {}", project_dir.display(), e))?;

        let status_path = project_dir.join("status.md");
        let initial_doc = format!(
            "---\nid: status-{proj}\nstatus: active\nhealth: healthy\ngoal: \"Deliver {proj}\"\nlast_updated: \"{now}\"\n---\n# {proj} Project Status\n\n## Goal\nDeliver {proj}\n",
            proj = project_name,
            now = Utc::now().to_rfc3339(),
        );

        fs::write(&status_path, initial_doc)
            .map_err(|e| format!("Failed to write initial status.md: {}", e))?;

        Ok(status_path)
    }

    pub fn find_task_file(project_dir: &Path, task_id: &str) -> Option<PathBuf> {
        let tasks_dir = project_dir.join("tasks");
        if tasks_dir.is_dir() {
            let exact = tasks_dir.join(format!("{}.md", task_id));
            if exact.is_file() {
                return Some(exact);
            }
            let prefixed = tasks_dir.join(format!("task-{}.md", task_id));
            if prefixed.is_file() {
                return Some(prefixed);
            }

            if let Ok(entries) = fs::read_dir(&tasks_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("md") {
                        let fname = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        if fname.contains(task_id) {
                            return Some(path);
                        }
                    }
                }
            }
        }

        // Also check directly under project directory
        let exact = project_dir.join(format!("{}.md", task_id));
        if exact.is_file() {
            return Some(exact);
        }

        None
    }

    pub fn transition_project_health(
        root: &Path,
        project: &str,
        health: HealthState,
        reason: Option<&str>,
        blocker: Option<&str>,
    ) -> Result<StatusDocument, String> {
        let project_dir = root.join("projects").join(project);
        let status_path = Self::ensure_status_file(&project_dir, project)?;

        FrontmatterSplicer::splice_file(&status_path, |val| {
            let map = val.as_mapping_mut()
                .ok_or_else(|| "Frontmatter is not a YAML mapping".to_string())?;

            map.insert(
                serde_yaml::Value::String("health".to_string()),
                serde_yaml::Value::String(health.as_str().to_string()),
            );

            if health == HealthState::Blocked {
                map.insert(
                    serde_yaml::Value::String("status".to_string()),
                    serde_yaml::Value::String("blocked".to_string()),
                );
            } else if health == HealthState::Healthy {
                if let Some(status_val) = map.get(&serde_yaml::Value::String("status".to_string())) {
                    if status_val.as_str() == Some("blocked") {
                        map.insert(
                            serde_yaml::Value::String("status".to_string()),
                            serde_yaml::Value::String("active".to_string()),
                        );
                    }
                }
            }

            if let Some(r) = reason {
                map.insert(
                    serde_yaml::Value::String("reason".to_string()),
                    serde_yaml::Value::String(r.to_string()),
                );
            }

            if let Some(b) = blocker {
                let blk_key = serde_yaml::Value::String("blockers".to_string());
                let blockers_seq = map.entry(blk_key).or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
                if let Some(seq) = blockers_seq.as_sequence_mut() {
                    let mut item_map = serde_yaml::Mapping::new();
                    let short_id = format!("blk-{}", &uuid::Uuid::now_v7().to_string()[..8]);
                    item_map.insert(serde_yaml::Value::String("id".to_string()), serde_yaml::Value::String(short_id));
                    item_map.insert(serde_yaml::Value::String("description".to_string()), serde_yaml::Value::String(b.to_string()));
                    item_map.insert(serde_yaml::Value::String("resolved".to_string()), serde_yaml::Value::Bool(false));
                    seq.push(serde_yaml::Value::Mapping(item_map));
                }
            }

            map.insert(
                serde_yaml::Value::String("last_updated".to_string()),
                serde_yaml::Value::String(Utc::now().to_rfc3339()),
            );

            Ok(())
        })?;

        Self::get_project_status(root, project)
    }

    pub fn transition_task(
        root: &Path,
        project: &str,
        task_id: &str,
        to_state: TaskState,
        reason: Option<&str>,
    ) -> Result<TaskDocument, String> {
        let project_dir = root.join("projects").join(project);
        let task_file = match Self::find_task_file(&project_dir, task_id) {
            Some(p) => p,
            None => {
                let tasks_dir = project_dir.join("tasks");
                fs::create_dir_all(&tasks_dir).map_err(|e| format!("Failed to create tasks directory: {}", e))?;
                let new_path = tasks_dir.join(format!("{}.md", task_id));
                let initial_task = format!(
                    "---\nid: {id}\nstatus: pending\nkind: task\n---\n# Task: {id}\n\nTask details.\n",
                    id = task_id,
                );
                fs::write(&new_path, initial_task).map_err(|e| format!("Failed to write task file: {}", e))?;
                new_path
            }
        };

        FrontmatterSplicer::splice_file(&task_file, |val| {
            let map = val.as_mapping_mut()
                .ok_or_else(|| "Task frontmatter is not a YAML mapping".to_string())?;

            map.insert(
                serde_yaml::Value::String("status".to_string()),
                serde_yaml::Value::String(to_state.as_str().to_string()),
            );

            if let Some(r) = reason {
                map.insert(
                    serde_yaml::Value::String("reason".to_string()),
                    serde_yaml::Value::String(r.to_string()),
                );
                if to_state == TaskState::Blocked {
                    map.insert(
                        serde_yaml::Value::String("blocked_by".to_string()),
                        serde_yaml::Value::String(r.to_string()),
                    );
                }
            }

            map.insert(
                serde_yaml::Value::String("last_transitioned_at".to_string()),
                serde_yaml::Value::String(Utc::now().to_rfc3339()),
            );

            Ok(())
        })?;

        // Sync with status.md active_task critical path and blockers
        if let Ok(status_path) = Self::ensure_status_file(&project_dir, project) {
            let _ = FrontmatterSplicer::splice_file(&status_path, |val| {
                if let Some(map) = val.as_mapping_mut() {
                    let active_key = serde_yaml::Value::String("active_task".to_string());
                    if to_state == TaskState::InProgress {
                        map.insert(active_key, serde_yaml::Value::String(task_id.to_string()));
                    } else if to_state == TaskState::Completed {
                        if let Some(curr) = map.get(&active_key) {
                            if curr.as_str() == Some(task_id) {
                                map.remove(&active_key);
                            }
                        }
                    } else if to_state == TaskState::Blocked {
                        map.insert(
                            serde_yaml::Value::String("health".to_string()),
                            serde_yaml::Value::String("blocked".to_string()),
                        );
                        map.insert(
                            serde_yaml::Value::String("status".to_string()),
                            serde_yaml::Value::String("blocked".to_string()),
                        );
                        if let Some(r) = reason {
                            let blk_key = serde_yaml::Value::String("blockers".to_string());
                            let blockers_seq = map.entry(blk_key).or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
                            if let Some(seq) = blockers_seq.as_sequence_mut() {
                                let mut item_map = serde_yaml::Mapping::new();
                                let short_id = format!("blk-{}", &uuid::Uuid::now_v7().to_string()[..8]);
                                item_map.insert(serde_yaml::Value::String("id".to_string()), serde_yaml::Value::String(short_id));
                                item_map.insert(serde_yaml::Value::String("description".to_string()), serde_yaml::Value::String(format!("{}: {}", task_id, r)));
                                item_map.insert(serde_yaml::Value::String("resolved".to_string()), serde_yaml::Value::Bool(false));
                                seq.push(serde_yaml::Value::Mapping(item_map));
                            }
                        }
                    }
                    map.insert(
                        serde_yaml::Value::String("last_updated".to_string()),
                        serde_yaml::Value::String(Utc::now().to_rfc3339()),
                    );
                }
                Ok(())
            });
        }

        let task_content = fs::read_to_string(&task_file)
            .map_err(|e| format!("Failed to read mutated task file: {}", e))?;
        let (parsed, _) = FrontmatterSplicer::parse::<TaskDocument>(&task_content)?;

        Ok(parsed.unwrap_or(TaskDocument {
            id: task_id.to_string(),
            title: Some(task_id.to_string()),
            status: to_state,
            owner: None,
            milestone: None,
            reason: reason.map(|s| s.to_string()),
            blocked_by: None,
            last_transitioned_at: Some(Utc::now().to_rfc3339()),
        }))
    }

    pub fn get_project_status(root: &Path, project: &str) -> Result<StatusDocument, String> {
        let project_dir = root.join("projects").join(project);
        let status_path = Self::find_status_file(&project_dir)
            .ok_or_else(|| format!("No status file found for project '{}'", project))?;

        let content = fs::read_to_string(&status_path)
            .map_err(|e| format!("Cannot read '{}': {}", status_path.display(), e))?;

        let (doc_opt, body) = FrontmatterSplicer::parse::<StatusDocument>(&content)?;
        if let Some(mut doc) = doc_opt {
            if doc.goal.is_empty() {
                doc.goal = Self::extract_goal_from_body(body);
            }
            Ok(doc)
        } else {
            // Synthesize StatusDocument from plain markdown body
            Ok(StatusDocument {
                id: Some(format!("status-{}", project)),
                title: Some(format!("{} Status", project)),
                status: StatusState::Active,
                health: HealthState::Healthy,
                goal: Self::extract_goal_from_body(body),
                baseline: None,
                active_task: None,
                blockers: Vec::new(),
                milestones: Vec::new(),
                exit_criteria: None,
                last_updated: None,
            })
        }
    }

    fn extract_goal_from_body(body: &str) -> String {
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("| Goal |") || trimmed.starts_with("| goal |") {
                let parts: Vec<&str> = trimmed.split('|').collect();
                if parts.len() >= 3 {
                    return parts[2].trim().to_string();
                }
            } else if trimmed.starts_with("Goal:") {
                return trimmed["Goal:".len()..].trim().to_string();
            }
        }

        // Fallback: extract the first non-header, non-table descriptive paragraph
        let mut in_table = false;
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('|') {
                in_table = true;
                continue;
            }
            if in_table && trimmed.starts_with('|') {
                continue;
            }
            in_table = false;
            if trimmed.len() > 15 && !trimmed.starts_with("---") {
                let snippet = if trimmed.len() > 120 {
                    format!("{}...", &trimmed[..117])
                } else {
                    trimmed.to_string()
                };
                return snippet;
            }
        }

        "No goal specified".to_string()
    }

    pub fn defer_finding(
        root: &Path,
        project: &str,
        title: &str,
        details: &str,
        severity: Option<&str>,
    ) -> Result<String, String> {
        let project_dir = root.join("projects").join(project);
        fs::create_dir_all(&project_dir)
            .map_err(|e| format!("Failed to create project directory: {}", e))?;

        let backlog_path = project_dir.join("BACKLOG.md");
        let sev = severity.unwrap_or("debt").to_lowercase();
        let timestamp = Utc::now().format("%Y-%m-%d").to_string();
        let short_id = format!("fnd-{}", &uuid::Uuid::now_v7().to_string()[..8]);

        let entry = format!(
            "\n### [{sev}] {title} (`{short_id}`)\n- **Recorded**: {timestamp}\n- **Severity**: {sev}\n- **Details**: {details}\n",
            sev = sev.to_uppercase(),
            title = title,
            short_id = short_id,
            timestamp = timestamp,
            details = details.trim(),
        );

        if !backlog_path.exists() {
            let initial = format!(
                "# {} Backlog & Deferred Findings\n\nSide-quests and adjacent findings deferred during active sprints to prevent context churn.\n{}",
                project, entry
            );
            fs::write(&backlog_path, initial)
                .map_err(|e| format!("Failed to create BACKLOG.md: {}", e))?;
        } else {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&backlog_path)
                .map_err(|e| format!("Failed to open BACKLOG.md: {}", e))?;
            file.write_all(entry.as_bytes())
                .map_err(|e| format!("Failed to append to BACKLOG.md: {}", e))?;
        }

        Ok(short_id)
    }

    pub fn verify_exit_criteria(
        root: &Path,
        project: &str,
    ) -> Result<ExitVerificationResult, String> {
        let status_doc = Self::get_project_status(root, project)?;
        let exit_crit = status_doc
            .exit_criteria
            .ok_or_else(|| format!("No exit criteria defined in status.md for project '{}'", project))?;

        let project_dir = root.join("projects").join(project);
        let working_dir = if project_dir.is_dir() { &project_dir } else { root };

        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&exit_crit.command)
            .current_dir(working_dir)
            .output()
            .map_err(|e| format!("Failed to execute exit verification command '{}': {}", exit_crit.command, e))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let passed = exit_code == exit_crit.expected_exit_code;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if passed {
            let status_path = Self::find_status_file(&project_dir)
                .ok_or_else(|| "Status file not found".to_string())?;

            let now = Utc::now().to_rfc3339();
            FrontmatterSplicer::splice_file(&status_path, |val| {
                let map = val.as_mapping_mut()
                    .ok_or_else(|| "Frontmatter is not a mapping".to_string())?;
                map.insert(
                    serde_yaml::Value::String("status".to_string()),
                    serde_yaml::Value::String("completed".to_string()),
                );
                map.insert(
                    serde_yaml::Value::String("health".to_string()),
                    serde_yaml::Value::String("healthy".to_string()),
                );
                let crit_key = serde_yaml::Value::String("exit_criteria".to_string());
                if let Some(crit_val) = map.get_mut(&crit_key) {
                    if let Some(crit_map) = crit_val.as_mapping_mut() {
                        crit_map.insert(
                            serde_yaml::Value::String("verified_at".to_string()),
                            serde_yaml::Value::String(now.clone()),
                        );
                    }
                }
                map.insert(
                    serde_yaml::Value::String("last_updated".to_string()),
                    serde_yaml::Value::String(now),
                );
                Ok(())
            })?;
        }

        Ok(ExitVerificationResult {
            project: project.to_string(),
            command: exit_crit.command,
            exit_code,
            expected_exit_code: exit_crit.expected_exit_code,
            passed,
            stdout,
            stderr,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ExitVerificationResult {
    pub project: String,
    pub command: String,
    pub exit_code: i32,
    pub expected_exit_code: i32,
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_engine_health_transition_and_atomic_splicing() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb_test_status_{}", uuid::Uuid::now_v7()));
        let project_name = "data-pipeline";

        // 1. Initial transition creates status.md
        let status = StatusEngine::transition_project_health(
            &temp_dir,
            project_name,
            HealthState::Blocked,
            Some("Upstream database connection timed out"),
            Some("DB Connection Timeout"),
        ).expect("should transition health successfully");

        assert_eq!(status.health, HealthState::Blocked);
        assert_eq!(status.status, StatusState::Blocked);
        assert_eq!(status.blockers.len(), 1);
        assert_eq!(status.blockers[0].description, "DB Connection Timeout");

        // 2. Unblock project
        let unblocked = StatusEngine::transition_project_health(
            &temp_dir,
            project_name,
            HealthState::Healthy,
            Some("Resolved connection string"),
            None,
        ).expect("should unblock successfully");

        assert_eq!(unblocked.health, HealthState::Healthy);
        assert_eq!(unblocked.status, StatusState::Active);

        // 3. Verify task transition updates active_task
        let task = StatusEngine::transition_task(
            &temp_dir,
            project_name,
            "task-42",
            TaskState::InProgress,
            Some("Starting implementation"),
        ).expect("should transition task");

        assert_eq!(task.status, TaskState::InProgress);
        assert_eq!(task.id, "task-42");

        let status_after_task = StatusEngine::get_project_status(&temp_dir, project_name)
            .expect("should read status");
        assert_eq!(status_after_task.active_task, Some("task-42".to_string()));

        // Complete the task and ensure active_task is cleared
        let completed_task = StatusEngine::transition_task(
            &temp_dir,
            project_name,
            "task-42",
            TaskState::Completed,
            Some("All unit tests passing"),
        ).expect("should complete task");
        assert_eq!(completed_task.status, TaskState::Completed);

        let status_after_complete = StatusEngine::get_project_status(&temp_dir, project_name)
            .expect("should read status");
        assert_eq!(status_after_complete.active_task, None);

        // 4. Verify transitioning task to Blocked automatically updates status.md health and blockers
        let blocked_task = StatusEngine::transition_task(
            &temp_dir,
            project_name,
            "task-43",
            TaskState::Blocked,
            Some("Upstream dependency broken"),
        ).expect("should block task");
        assert_eq!(blocked_task.status, TaskState::Blocked);

        let status_after_block = StatusEngine::get_project_status(&temp_dir, project_name)
            .expect("should read status");
        assert_eq!(status_after_block.health, HealthState::Blocked);
        assert_eq!(status_after_block.status, StatusState::Blocked);
        assert!(!status_after_block.blockers.is_empty());
        assert!(status_after_block.blockers.iter().any(|b| b.description.contains("Upstream dependency broken")));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_status_engine_defer_finding() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb_test_defer_{}", uuid::Uuid::now_v7()));
        let project = "auth-service";

        let finding_id = StatusEngine::defer_finding(
            &temp_dir,
            project,
            "Refactor Legacy JWT Parser",
            "Observed during login bugfix; token validation allocates unnecessarily.",
            Some("medium"),
        ).expect("should defer finding");

        assert!(finding_id.starts_with("fnd-"));
        let backlog_path = temp_dir.join("projects").join(project).join("BACKLOG.md");
        assert!(backlog_path.exists());
        let content = fs::read_to_string(&backlog_path).expect("read backlog");
        assert!(content.contains("[MEDIUM] Refactor Legacy JWT Parser"));
        assert!(content.contains(&finding_id));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_status_engine_verify_exit_criteria() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb_test_exit_{}", uuid::Uuid::now_v7()));
        let project = "cli-tool";
        let project_dir = temp_dir.join("projects").join(project);
        fs::create_dir_all(&project_dir).unwrap();

        let status_md = format!(
            "---\nid: status-{proj}\nstatus: active\nhealth: healthy\ngoal: \"Make CLI fast\"\nexit_criteria:\n  command: \"echo exit_test_ok\"\n  expected_exit_code: 0\n---\n# CLI Status\n",
            proj = project
        );
        fs::write(project_dir.join("status.md"), status_md).unwrap();

        let res = StatusEngine::verify_exit_criteria(&temp_dir, project).expect("should verify exit criteria");
        assert!(res.passed);
        assert_eq!(res.exit_code, 0);

        let updated_status = StatusEngine::get_project_status(&temp_dir, project).unwrap();
        assert_eq!(updated_status.status, StatusState::Completed);
        assert!(updated_status.exit_criteria.unwrap().verified_at.is_some());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
