use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub title: String,
    pub content: String,
    pub kind: String,
    pub origin: String,
    pub created_at: String,
}

