use serde::{Deserialize, Serialize};

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
}

impl ProjectSummary {
    pub fn open_tasks_count(&self) -> usize {
        self.tasks_pending + self.tasks_in_progress + self.tasks_blocked
    }

    pub fn is_healthy(&self) -> bool {
        self.has_status_doc && self.tasks_blocked == 0 && self.open_risks == 0
    }
}
