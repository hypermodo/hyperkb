use crate::domain::{
    HarnessConfig, HarnessDefinition, HarnessGovernanceStatus, HarnessProtocol, HyperControlPolicy,
};
use chrono::Utc;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

pub struct HarnessDiscovery;

impl HarnessDiscovery {
    /// Probe the local developer machine and combine with manifest configuration and HyperControl policy.
    pub fn discover(manifest_config: &HarnessConfig) -> Vec<HarnessDefinition> {
        let mut results: Vec<HarnessDefinition> = Vec::new();
        let mut seen_ids: HashSet<String> = HashSet::new();

        // 1. First, include all explicitly registered harnesses from hyperkb.json
        for reg in &manifest_config.registered {
            if !seen_ids.contains(&reg.id) {
                seen_ids.insert(reg.id.clone());
                results.push(reg.clone());
            }
        }

        // 2. If auto-discovery is enabled, scan PATH, local listeners, and agent environments
        if manifest_config.auto_discovery {
            // Probe common developer AI harnesses
            let candidates = [
                ("opencode", "OpenCode AI Harness", vec!["code_generation", "interactive_tui", "context_provider"], vec!["chatgpt-4o", "claude-3-7-sonnet", "local"]),
                ("claude", "Claude Code CLI", vec!["code_generation", "subagents", "tool_calling"], vec!["claude-3-7-sonnet", "claude-3-5-haiku"]),
                ("codex", "Codex / OpenAI CLI", vec!["code_generation", "inline_completion"], vec!["o1", "o3-mini", "gpt-4o"]),
                ("ollama", "Ollama Local Engine", vec!["offline_local_inference", "embeddings", "code_generation"], vec!["llama3.3", "qwen2.5-coder", "deepseek-r1"]),
                ("gemini", "Gemini Developer CLI", vec!["multimodal", "code_generation", "tool_calling"], vec!["gemini-2.5-pro", "gemini-2.5-flash"]),
                ("cursor", "Cursor Agent CLI", vec!["code_generation", "composer", "diff_patch"], vec!["claude-3-7-sonnet", "gpt-4o"]),
                ("aider", "Aider AI Pair Programmer", vec!["git_commit_pairing", "architectural_edit"], vec!["claude-3-7-sonnet", "gpt-4o"]),
            ];

            for (binary_name, name, caps, models) in candidates {
                if !seen_ids.contains(binary_name) {
                    if let Some(bin_path) = Self::find_binary_in_path(binary_name) {
                        seen_ids.insert(binary_name.to_string());
                        results.push(HarnessDefinition {
                            id: binary_name.to_string(),
                            name: name.to_string(),
                            protocol: HarnessProtocol::CliSubprocess {
                                binary: binary_name.to_string(),
                                default_args: Vec::new(),
                            },
                            capabilities: caps.into_iter().map(String::from).collect(),
                            detected_models: models.into_iter().map(String::from).collect(),
                            governance_status: HarnessGovernanceStatus::Discovered,
                            governance_reason: Some(format!("Discovered executable in PATH: {}", bin_path.display())),
                            binary_path: Some(bin_path.to_string_lossy().to_string()),
                            last_seen: Some(Utc::now()),
                        });
                    }
                }
            }

            // Probe Antigravity Agent & IDE Environment
            if !seen_ids.contains("antigravity") {
                if Self::is_antigravity_environment() {
                    seen_ids.insert("antigravity".to_string());
                    results.push(HarnessDefinition {
                        id: "antigravity".to_string(),
                        name: "Google Antigravity (DeepMind Agent Environment)".to_string(),
                        protocol: HarnessProtocol::McpBridge {
                            socket_or_url: "antigravity-mcp://local".to_string(),
                            transport: "stdio/jsonrpc".to_string(),
                        },
                        capabilities: vec![
                            "agentic_coding".to_string(),
                            "mcp_host".to_string(),
                            "sidecar_telemetry".to_string(),
                            "governance_hooks".to_string(),
                        ],
                        detected_models: vec![
                            "gemini-2.5-pro".to_string(),
                            "gemini-2.5-flash".to_string(),
                        ],
                        governance_status: HarnessGovernanceStatus::Discovered,
                        governance_reason: Some("Detected active Antigravity IDE agent session environment".to_string()),
                        binary_path: None,
                        last_seen: Some(Utc::now()),
                    });
                }
            }
        }

        // 3. Apply HyperControl CISO Policy (if configured)
        if let Some(ref policy) = manifest_config.hypercontrol {
            Self::apply_hypercontrol_policy(&mut results, policy);
        }

        results
    }

    /// Check if an executable exists in PATH
    pub fn find_binary_in_path(binary_name: &str) -> Option<PathBuf> {
        let path_var = env::var_os("PATH")?;
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join(binary_name);
            if candidate.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = candidate.metadata() {
                        if meta.permissions().mode() & 0o111 != 0 {
                            return Some(candidate);
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    return Some(candidate);
                }
            }
        }
        None
    }

    /// Check if current execution environment is Antigravity
    pub fn is_antigravity_environment() -> bool {
        if env::var("ANTIGRAVITY_SESSION_ID").is_ok() {
            return true;
        }
        if let Ok(home) = env::var("HOME") {
            let antigravity_dir = Path::new(&home).join(".gemini/antigravity-ide");
            if antigravity_dir.exists() {
                return true;
            }
        }
        false
    }

    /// Apply corporate CISO governance constraints from HyperControl
    fn apply_hypercontrol_policy(harnesses: &mut [HarnessDefinition], policy: &HyperControlPolicy) {
        let policy_tag = policy.policy_id.as_deref().unwrap_or("CISO-DEFAULT");

        for harness in harnesses.iter_mut() {
            // Check blocked harnesses
            if policy.blocked_harness_ids.iter().any(|b| b.eq_ignore_ascii_case(&harness.id)) {
                harness.governance_status = HarnessGovernanceStatus::Blocked;
                harness.governance_reason = Some(format!(
                    "Explicitly blocked by corporate HyperControl policy #{}: unauthorized harness",
                    policy_tag
                ));
                continue;
            }

            // Check if whitelisting is enforced
            if !policy.allowed_harness_ids.is_empty() {
                if policy.allowed_harness_ids.iter().any(|a| a.eq_ignore_ascii_case(&harness.id)) {
                    harness.governance_status = HarnessGovernanceStatus::Allowed;
                    harness.governance_reason = Some(format!(
                        "Approved by corporate HyperControl policy #{}",
                        policy_tag
                    ));
                } else {
                    harness.governance_status = HarnessGovernanceStatus::Blocked;
                    harness.governance_reason = Some(format!(
                        "Prohibited by HyperControl policy #{}: not in CISO approved registry",
                        policy_tag
                    ));
                }
            } else {
                harness.governance_status = HarnessGovernanceStatus::Allowed;
                harness.governance_reason = Some(format!(
                    "Permitted under default HyperControl policy #{}",
                    policy_tag
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hypercontrol_ciso_blocking_and_whitelisting() {
        let mut config = HarnessConfig::default();
        config.auto_discovery = false;
        config.registered = vec![
            HarnessDefinition {
                id: "opencode".to_string(),
                name: "OpenCode".to_string(),
                protocol: HarnessProtocol::CliSubprocess {
                    binary: "opencode".to_string(),
                    default_args: vec![],
                },
                capabilities: vec![],
                detected_models: vec!["chatgpt".to_string()],
                governance_status: HarnessGovernanceStatus::Discovered,
                governance_reason: None,
                binary_path: None,
                last_seen: None,
            },
            HarnessDefinition {
                id: "unapproved-llm".to_string(),
                name: "Unapproved Shadow AI".to_string(),
                protocol: HarnessProtocol::CliSubprocess {
                    binary: "shadow-ai".to_string(),
                    default_args: vec![],
                },
                capabilities: vec![],
                detected_models: vec!["unknown".to_string()],
                governance_status: HarnessGovernanceStatus::Discovered,
                governance_reason: None,
                binary_path: None,
                last_seen: None,
            },
        ];

        // 1. CISO policy with whitelist
        config.hypercontrol = Some(HyperControlPolicy {
            policy_id: Some("CISO-POL-2026".to_string()),
            ciso_server: Some("https://hypercontrol.corp.internal".to_string()),
            allowed_harness_ids: vec!["opencode".to_string()],
            blocked_harness_ids: vec!["unapproved-llm".to_string()],
            allowed_models: vec![],
            blocked_models: vec![],
            require_signed_grants: true,
        });

        let discovered = HarnessDiscovery::discover(&config);
        assert_eq!(discovered.len(), 2);

        let opencode = discovered.iter().find(|h| h.id == "opencode").unwrap();
        assert_eq!(opencode.governance_status, HarnessGovernanceStatus::Allowed);

        let unapproved = discovered.iter().find(|h| h.id == "unapproved-llm").unwrap();
        assert_eq!(unapproved.governance_status, HarnessGovernanceStatus::Blocked);
        assert!(unapproved.governance_reason.as_ref().unwrap().contains("Explicitly blocked by corporate HyperControl"));
    }
}
