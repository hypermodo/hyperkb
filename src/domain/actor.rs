use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    ProposeDecision,
    AcceptDecision,
    AcknowledgeRisk,
    AutoRepair,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantConstraints {
    pub max_line_diff: Option<usize>,
    pub require_tests_pass: bool,
    pub allow_supersede: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Default for GrantConstraints {
    fn default() -> Self {
        Self {
            max_line_diff: Some(200),
            require_tests_pass: false,
            allow_supersede: true,
            expires_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityGrant {
    pub grant_id: Uuid,
    pub granted_by: String,
    pub allowed_actions: Vec<ActionKind>,
    pub allowed_scope_patterns: Vec<String>,
    pub constraints: GrantConstraints,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Actor {
    Human {
        username: String,
    },
    Agent {
        agent_id: String,
        model: String,
        grant: AuthorityGrant,
    },
}

impl Actor {
    pub fn name(&self) -> &str {
        match self {
            Actor::Human { username } => username,
            Actor::Agent { agent_id, .. } => agent_id,
        }
    }

    pub fn can_perform(&self, action: &ActionKind, target_path: &str) -> bool {
        match self {
            Actor::Human { .. } => true, // Humans have default owner authority
            Actor::Agent { grant, .. } => {
                if let Some(exp) = grant.constraints.expires_at {
                    if Utc::now() > exp {
                        return false;
                    }
                }
                if !grant.allowed_actions.contains(action) {
                    return false;
                }
                grant.allowed_scope_patterns.iter().any(|pattern| {
                    if pattern == "*" || pattern == "**" {
                        return true;
                    }
                    if pattern.ends_with("/**") {
                        let prefix = pattern.trim_end_matches("/**");
                        return target_path.starts_with(prefix);
                    }
                    target_path == pattern
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_has_full_authority() {
        let actor = Actor::Human {
            username: "wiqar".into(),
        };
        assert!(actor.can_perform(&ActionKind::AcceptDecision, "docs/decisions/001.md"));
    }

    #[test]
    fn test_agent_scope_clamping() {
        let grant = AuthorityGrant {
            grant_id: Uuid::now_v7(),
            granted_by: "wiqar".into(),
            allowed_actions: vec![ActionKind::AutoRepair, ActionKind::AcceptDecision],
            allowed_scope_patterns: vec!["projects/audits/**".into()],
            constraints: GrantConstraints::default(),
            signature: None,
        };
        let agent = Actor::Agent {
            agent_id: "opencode".into(),
            model: "gemini-3.8-flash".into(),
            grant,
        };

        assert!(agent.can_perform(&ActionKind::AcceptDecision, "projects/audits/audit-1.md"));
        assert!(!agent.can_perform(&ActionKind::AcceptDecision, "docs/decisions/core.md"));
        assert!(!agent.can_perform(&ActionKind::AcknowledgeRisk, "projects/audits/audit-1.md"));
    }
}

