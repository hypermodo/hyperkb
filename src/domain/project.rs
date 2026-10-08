use serde::{Deserialize, Serialize};

fn default_project_status() -> String {
    "active".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub path: String,
    pub total_documents: usize,
    pub tasks_pending: usize,
    pub tasks_in_progress: usize,
    pub tasks_completed: usize,
    pub tasks_blocked: usize,
    pub open_risks: usize,
    pub decisions_count: usize,
    pub has_status_doc: bool,
    #[serde(default)]
    pub health: String,
    #[serde(default = "default_project_status")]
    pub status: String,
    #[serde(default)]
    pub active_task: Option<String>,
    #[serde(default)]
    pub exit_criteria: Option<String>,
    #[serde(default)]
    pub exit_verified: bool,
    #[serde(default)]
    pub churn_warning: bool,
}

impl ProjectSummary {
    pub fn open_tasks_count(&self) -> usize {
        self.tasks_pending + self.tasks_in_progress + self.tasks_blocked
    }

    pub fn is_healthy(&self) -> bool {
        self.has_status_doc && self.tasks_blocked == 0 && self.open_risks == 0 && self.health != "blocked"
    }

    pub fn is_blocked(&self) -> bool {
        self.tasks_blocked > 0 || self.health == "blocked"
    }

    pub fn is_closed(&self) -> bool {
        self.exit_verified || self.status == "completed" || self.status == "archived" || (self.tasks_completed > 0 && self.open_tasks_count() == 0 && self.open_risks == 0)
    }

    pub fn is_active_or_blocked(&self) -> bool {
        self.is_blocked() || self.tasks_in_progress > 0 || self.tasks_pending > 0 || self.status == "active" || self.status == "reopened" || !self.is_closed()
    }
}
