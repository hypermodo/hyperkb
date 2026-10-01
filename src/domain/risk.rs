use super::document::{Document, Hit};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskApplicability {
    Applies,
    Unknown,
    NotApplicable,
}

impl RiskApplicability {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskApplicability::Applies => "applies",
            RiskApplicability::Unknown => "unknown",
            RiskApplicability::NotApplicable => "not_applicable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskMatch {
    pub document: Document,
    pub matched_paths: Vec<String>,
    pub reason: String,
    pub applicability: RiskApplicability,
    pub acknowledged: bool,
    #[serde(default)]
    pub acknowledgement: Option<String>,
    #[serde(default)]
    pub external_issue_freshness: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskCheck {
    pub matches: Vec<RiskMatch>,
    #[serde(default)]
    pub historical_candidates: Vec<Hit>,
    pub checked_paths: Vec<String>,
    pub coverage_complete: bool,
    pub applicability_known: bool,
    pub message: String,
}

impl RiskCheck {
    pub fn has_open_risks(&self) -> bool {
        self.matches
            .iter()
            .any(|m| m.applicability == RiskApplicability::Applies && !m.acknowledged)
    }
}

