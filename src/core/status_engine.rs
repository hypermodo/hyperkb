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

        // Sync with status.md active_task critical path
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
        "No goal specified".to_string()
    }
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

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
