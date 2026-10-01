use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HarnessProtocol {
    #[serde(rename = "cli")]
    CliSubprocess {
        binary: String,
        #[serde(default)]
        default_args: Vec<String>,
    },
    #[serde(rename = "http")]
    HttpApi {
        endpoint: String,
        #[serde(default)]
        auth_env_var: Option<String>,
    },
    #[serde(rename = "mcp")]
    McpBridge {
        socket_or_url: String,
        transport: String,
    },
    #[serde(rename = "custom")]
    Custom {
        kind: String,
        endpoint: String,
    },
}

impl HarnessProtocol {
    pub fn protocol_label(&self) -> &'static str {
        match self {
            HarnessProtocol::CliSubprocess { .. } => "CLI Subprocess",
            HarnessProtocol::HttpApi { .. } => "HTTP REST / SSE",
            HarnessProtocol::McpBridge { .. } => "Model Context Protocol (MCP)",
            HarnessProtocol::Custom { .. } => "Custom Bridge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarnessGovernanceStatus {
    #[serde(rename = "discovered")]
    Discovered,
    #[serde(rename = "allowed")]
    Allowed,
    #[serde(rename = "blocked")]
    Blocked,
    #[serde(rename = "enforced")]
    Enforced,
}

impl HarnessGovernanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HarnessGovernanceStatus::Discovered => "DISCOVERED",
            HarnessGovernanceStatus::Allowed => "ALLOWED",
            HarnessGovernanceStatus::Blocked => "BLOCKED",
            HarnessGovernanceStatus::Enforced => "ENFORCED",
        }
    }
}

fn default_governance_status() -> HarnessGovernanceStatus {
    HarnessGovernanceStatus::Discovered
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessDefinition {
    pub id: String,
    pub name: String,
    pub protocol: HarnessProtocol,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub detected_models: Vec<String>,
    #[serde(default = "default_governance_status")]
    pub governance_status: HarnessGovernanceStatus,
    #[serde(default)]
    pub governance_reason: Option<String>,
    #[serde(default)]
    pub binary_path: Option<String>,
    #[serde(default)]
    pub last_seen: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HyperControlPolicy {
    #[serde(default)]
    pub policy_id: Option<String>,
    #[serde(default)]
    pub ciso_server: Option<String>,
    #[serde(default)]
    pub allowed_harness_ids: Vec<String>,
    #[serde(default)]
    pub blocked_harness_ids: Vec<String>,
    #[serde(default)]
    pub allowed_models: Vec<String>,
    #[serde(default)]
    pub blocked_models: Vec<String>,
    #[serde(default)]
    pub require_signed_grants: bool,
}

fn default_auto_discovery() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessConfig {
    #[serde(default = "default_auto_discovery")]
    pub auto_discovery: bool,
    #[serde(default)]
    pub active_harness_id: Option<String>,
    #[serde(default)]
    pub registered: Vec<HarnessDefinition>,
    #[serde(default)]
    pub hypercontrol: Option<HyperControlPolicy>,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            auto_discovery: default_auto_discovery(),
            active_harness_id: None,
            registered: Vec::new(),
            hypercontrol: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_definition_serde_roundtrip() {
        let def = HarnessDefinition {
            id: "opencode".to_string(),
            name: "OpenCode AI Cockpit".to_string(),
            protocol: HarnessProtocol::CliSubprocess {
                binary: "opencode".to_string(),
                default_args: vec!["--headless".to_string()],
            },
            capabilities: vec!["code_generation".to_string(), "tool_calling".to_string()],
            detected_models: vec!["chatgpt-4o".to_string(), "claude-3-7-sonnet".to_string()],
            governance_status: HarnessGovernanceStatus::Allowed,
            governance_reason: Some("Approved developer coding assistant".to_string()),
            binary_path: Some("/usr/local/bin/opencode".to_string()),
            last_seen: Some(Utc::now()),
        };

        let json = serde_json::to_string(&def).unwrap();
        let deserialized: HarnessDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "opencode");
        assert_eq!(deserialized.governance_status, HarnessGovernanceStatus::Allowed);
        assert_eq!(deserialized.protocol.protocol_label(), "CLI Subprocess");
    }
}
