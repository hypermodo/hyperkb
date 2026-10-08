use serde::{Deserialize, Serialize};

fn default_health() -> HealthState {
    HealthState::Healthy
}

fn default_ttl_days() -> u32 {
    14
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatusState {
    Planned,
    Active,
    Blocked,
    Completed,
    Archived,
}

impl StatusState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Active => "active",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "active" | "in_progress" | "started" => Self::Active,
            "blocked" | "halted" => Self::Blocked,
            "completed" | "done" | "shipped" | "closed" => Self::Completed,
            "archived" | "refuted" | "abandoned" => Self::Archived,
            _ => Self::Planned,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    AtRisk,
    Blocked,
}

impl HealthState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::AtRisk => "at_risk",
            Self::Blocked => "blocked",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "blocked" | "red" => Self::Blocked,
            "at_risk" | "amber" | "yellow" | "degraded" => Self::AtRisk,
            _ => Self::Healthy,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Pending,
    InProgress,
    Completed,
    Blocked,
}

impl TaskState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "in_progress" | "inprogress" | "active" | "wip" => Self::InProgress,
            "completed" | "done" | "closed" => Self::Completed,
            "blocked" => Self::Blocked,
            _ => Self::Pending,
        }
    }
}

fn default_metadata_json() -> String {
    "{}".to_string()
}

fn default_knowledge_kind() -> KnowledgeKind {
    KnowledgeKind::Note
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    pub collection_id: String,
    pub project: String,
    #[serde(default)]
    pub session_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_task_state")]
    pub status: TaskState,
    #[serde(default)]
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default = "default_metadata_json")]
    pub metadata_json: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    Warning,
    Pattern,
    Decision,
    Note,
    Preference,
}

impl KnowledgeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Pattern => "pattern",
            Self::Decision => "decision",
            Self::Note => "note",
            Self::Preference => "preference",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "warning" | "warn" | "hazard" | "danger" => Self::Warning,
            "pattern" | "recipe" | "convention" => Self::Pattern,
            "decision" | "adr" => Self::Decision,
            "preference" | "pref" => Self::Preference,
            _ => Self::Note,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeRecord {
    pub id: String,
    pub collection_id: String,
    #[serde(default)]
    pub project: String,
    pub title: String,
    pub content: String,
    #[serde(default = "default_knowledge_kind")]
    pub kind: KnowledgeKind,
    #[serde(default)]
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_metadata_json")]
    pub metadata_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockerItem {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub resolved: bool,
    #[serde(default)]
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MilestoneItem {
    pub id: String,
    pub title: String,
    #[serde(default = "default_task_state")]
    pub status: TaskState,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub tasks: Vec<String>,
}

fn default_task_state() -> TaskState {
    TaskState::Pending
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExitCriteria {
    pub command: String,
    #[serde(default)]
    pub expected_exit_code: i32,
    #[serde(default)]
    pub verified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusDocument {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default = "default_status_state")]
    pub status: StatusState,
    #[serde(default = "default_health")]
    pub health: HealthState,
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub out_of_charter: Option<String>,
    #[serde(default)]
    pub baseline: Option<String>,
    #[serde(default)]
    pub active_task: Option<String>,
    #[serde(default)]
    pub blockers: Vec<BlockerItem>,
    #[serde(default)]
    pub milestones: Vec<MilestoneItem>,
    #[serde(default)]
    pub exit_criteria: Option<ExitCriteria>,
    #[serde(default)]
    pub last_updated: Option<String>,
}

fn default_status_state() -> StatusState {
    StatusState::Active
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskDocument {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default = "default_task_state")]
    pub status: TaskState,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub milestone: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub blocked_by: Option<String>,
    #[serde(default)]
    pub last_transitioned_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractItem {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub schema_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecDocument {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub non_goals: Vec<String>,
    #[serde(default)]
    pub contracts: Vec<ContractItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointItem {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub verified_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanDocument {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default = "default_ttl_days")]
    pub ttl_days: u32,
    #[serde(default)]
    pub checkpoints: Vec<CheckpointItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditVerdict {
    Pass,
    Fail,
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViolationItem {
    pub rule: String,
    pub file: String,
    #[serde(default)]
    pub line: Option<usize>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditDocument {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub commit_sha: Option<String>,
    pub verdict: AuditVerdict,
    #[serde(default)]
    pub rules_evaluated: Vec<String>,
    #[serde(default)]
    pub violations: Vec<ViolationItem>,
}
