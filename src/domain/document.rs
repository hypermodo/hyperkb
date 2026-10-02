use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Proposed,
    Accepted,
    Superseded,
    Open,
    Acknowledged,
    Resolved,
    Pending,
    InProgress,
    Completed,
    Blocked,
    Archived,
    Unknown,
    Conflict,
}

impl Default for DocumentStatus {
    fn default() -> Self {
        DocumentStatus::Unknown
    }
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentStatus::Proposed => "proposed",
            DocumentStatus::Accepted => "accepted",
            DocumentStatus::Superseded => "superseded",
            DocumentStatus::Open => "open",
            DocumentStatus::Acknowledged => "acknowledged",
            DocumentStatus::Resolved => "resolved",
            DocumentStatus::Pending => "pending",
            DocumentStatus::InProgress => "in_progress",
            DocumentStatus::Completed => "completed",
            DocumentStatus::Blocked => "blocked",
            DocumentStatus::Archived => "archived",
            DocumentStatus::Unknown => "unknown",
            DocumentStatus::Conflict => "conflict",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "proposed" => DocumentStatus::Proposed,
            "accepted" => DocumentStatus::Accepted,
            "superseded" => DocumentStatus::Superseded,
            "open" => DocumentStatus::Open,
            "acknowledged" => DocumentStatus::Acknowledged,
            "resolved" => DocumentStatus::Resolved,
            "pending" | "todo" => DocumentStatus::Pending,
            "in_progress" | "inprogress" | "active" => DocumentStatus::InProgress,
            "completed" | "done" => DocumentStatus::Completed,
            "blocked" => DocumentStatus::Blocked,
            "archived" => DocumentStatus::Archived,
            "conflict" => DocumentStatus::Conflict,
            _ => DocumentStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    Document,
    Decision,
    Risk,
    Task,
    Audit,
    Spec,
    Plan,
}

impl Default for DocumentKind {
    fn default() -> Self {
        DocumentKind::Document
    }
}

impl DocumentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentKind::Document => "document",
            DocumentKind::Decision => "decision",
            DocumentKind::Risk => "risk",
            DocumentKind::Task => "task",
            DocumentKind::Audit => "audit",
            DocumentKind::Spec => "spec",
            DocumentKind::Plan => "plan",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "decision" => DocumentKind::Decision,
            "risk" => DocumentKind::Risk,
            "task" => DocumentKind::Task,
            "audit" => DocumentKind::Audit,
            "spec" => DocumentKind::Spec,
            "plan" => DocumentKind::Plan,
            _ => DocumentKind::Document,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub collection_id: String,
    pub path: String,
    pub title: String,
    pub topic: String,
    pub status: DocumentStatus,
    pub kind: DocumentKind,
    pub owner: String,
    pub issue: String,
    pub replacement_id: Option<String>,
    pub supersedes: Option<String>,
    pub content: String,
    pub source: String,
    pub available: bool,
    pub stale: bool,
    pub declared_status: Option<String>,
    pub checksum: String,
    pub worktree_state: Option<String>,
    #[serde(default)]
    pub is_tombstone: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub id: String,
    pub collection_id: Option<String>,
    pub source: String,
    pub path: Option<String>,
    pub title: String,
    pub topic: Option<String>,
    pub status: DocumentStatus,
    pub kind: DocumentKind,
    pub replacement_id: Option<String>,
    pub supersedes: Option<String>,
    pub snippet: String,
    pub score: f64,
    pub available: bool,
    pub stale: bool,
    pub declared_status: Option<String>,
    pub worktree_state: Option<String>,
    pub broadened: bool,
    #[serde(default)]
    pub is_tombstone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordMeta {
    pub id: String,
    pub kind: String,
    pub status: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub issue: Option<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub environments: Vec<String>,
    #[serde(default)]
    pub supersedes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegation: Option<crate::domain::DelegationMeta>,
}

impl Default for RecordMeta {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: String::new(),
            status: String::new(),
            owner: String::new(),
            issue: None,
            paths: Vec::new(),
            versions: Vec::new(),
            environments: Vec::new(),
            supersedes: None,
            delegation: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowseOptions {
    pub category: String,
    pub collection_id: Option<String>,
    pub project: Option<String>,
    pub topic: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    pub recent: bool,
    pub limit: usize,
    pub offset: usize,
    #[serde(default)]
    pub include_archived: bool,
}

impl Default for BrowseOptions {
    fn default() -> Self {
        Self {
            category: "all".into(),
            collection_id: None,
            project: None,
            topic: None,
            status: None,
            kind: None,
            recent: false,
            limit: 20,
            offset: 0,
            include_archived: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexReport {
    pub scanned: usize,
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub errors: usize,
    pub prune_refused: bool,
    pub coverage_complete: bool,
    pub pending_moves: Vec<MoveSuggestion>,
    pub unavailable_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveSuggestion {
    pub old_path: String,
    pub new_path: String,
    pub source_id: String,
    pub ambiguous: bool,
}

