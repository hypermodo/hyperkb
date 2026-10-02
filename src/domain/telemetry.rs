use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectVelocityTelemetry {
    pub project: String,
    pub session_count: usize,
    pub completed_milestones: usize,
    pub open_tasks: usize,
    pub churn_ratio: f64,
    pub is_churning: bool,
}

impl ProjectVelocityTelemetry {
    pub const CHURN_SESSION_THRESHOLD: usize = 3;
    pub const CHURN_RATIO_THRESHOLD: f64 = 3.0;

    pub fn evaluate(
        project: &str,
        session_count: usize,
        completed_milestones: usize,
        open_tasks: usize,
    ) -> Self {
        let completed = completed_milestones.max(1);
        let churn_ratio = session_count as f64 / completed as f64;
        let is_churning = session_count >= Self::CHURN_SESSION_THRESHOLD
            && open_tasks > 0
            && (completed_milestones == 0 || churn_ratio >= Self::CHURN_RATIO_THRESHOLD);

        Self {
            project: project.to_string(),
            session_count,
            completed_milestones,
            open_tasks,
            churn_ratio,
            is_churning,
        }
    }
}

pub fn compute_project_churn(
    conn: &Connection,
    _collection_id: &str,
    project_name: &str,
    completed_milestones: usize,
    open_tasks: usize,
) -> bool {
    let sql = "SELECT count(DISTINCT session_id) FROM session_events 
               WHERE target_path LIKE 'projects/' || ?1 || '/%' 
                  OR target_path LIKE ?1 || '/%';";
    let count: usize = conn
        .query_row(sql, [project_name], |row| row.get(0))
        .unwrap_or(0);

    let telem = ProjectVelocityTelemetry::evaluate(project_name, count, completed_milestones, open_tasks);
    telem.is_churning
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_velocity_telemetry_churn_detection() {
        let t1 = ProjectVelocityTelemetry::evaluate("proj", 2, 0, 5);
        assert!(!t1.is_churning);

        let t2 = ProjectVelocityTelemetry::evaluate("proj", 3, 0, 5);
        assert!(t2.is_churning);

        let t3 = ProjectVelocityTelemetry::evaluate("proj", 4, 2, 2);
        assert!(!t3.is_churning);

        let t4 = ProjectVelocityTelemetry::evaluate("proj", 10, 2, 3);
        assert!(t4.is_churning);

        let t5 = ProjectVelocityTelemetry::evaluate("proj", 15, 5, 0);
        assert!(!t5.is_churning);
    }
}
