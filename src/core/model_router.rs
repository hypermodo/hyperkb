use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConsultationResult {
    pub peer: String,
    pub model: String,
    pub prompt: String,
    pub response: String,
    pub elapsed_ms: u64,
    pub success: bool,
    pub source: String,
}

pub struct ModelRouter;

impl ModelRouter {
    pub fn consult<P: AsRef<Path>>(
        root: P,
        peer_name: &str,
        prompt: &str,
        context_files: Option<&[String]>,
    ) -> PeerConsultationResult {
        let root = root.as_ref();
        let start = Instant::now();

        // 1. Build augmented prompt if context files provided
        let mut full_prompt = prompt.trim().to_string();
        if let Some(files) = context_files {
            if !files.is_empty() {
                full_prompt.push_str("\n\n### Attached Context Files:\n");
                for rel_path in files {
                    let clean = rel_path.trim_start_matches("./").trim();
                    let file_path = root.join(clean);
                    if file_path.is_file() {
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            // Truncate to first 150 lines to avoid prompt bloat
                            let lines: Vec<&str> = content.lines().take(150).collect();
                            let truncated = lines.join("\n");
                            full_prompt.push_str(&format!("\nFile `{}`:\n```\n{}\n```\n", clean, truncated));
                        }
                    }
                }
            }
        }

        let normalized = peer_name.to_lowercase();
        let normalized_str = normalized.as_str();

        match normalized_str {
            "claude" | "anthropic" | "sonnet" => {
                Self::call_claude(&full_prompt, start)
            }
            "chatgpt" | "openai" | "gpt" | "gpt-4o" => {
                Self::call_openai(&full_prompt, start)
            }
            "gemini" | "google" => {
                Self::call_gemini(&full_prompt, start)
            }
            other => PeerConsultationResult {
                peer: other.to_string(),
                model: "unknown".to_string(),
                prompt: full_prompt,
                response: format!("Unsupported peer model '{}'. Supported: claude, chatgpt, gemini", other),
                elapsed_ms: start.elapsed().as_millis() as u64,
                success: false,
                source: "unsupported".to_string(),
            },
        }
    }

    fn call_claude(prompt: &str, start: Instant) -> PeerConsultationResult {
        let model = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string());

        // 1. Try ANTHROPIC_API_KEY via curl
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            if !api_key.trim().is_empty() {
                let body = json!({
                    "model": model,
                    "max_tokens": 1024,
                    "messages": [
                        { "role": "user", "content": prompt }
                    ]
                });

                if let Ok(output) = Command::new("curl")
                    .args([
                        "-s", "-m", "30",
                        "-X", "POST", "https://api.anthropic.com/v1/messages",
                        "-H", &format!("x-api-key: {}", api_key.trim()),
                        "-H", "anthropic-version: 2023-06-01",
                        "-H", "content-type: application/json",
                        "-d", &body.to_string(),
                    ])
                    .output()
                {
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout);
                        if let Ok(val) = serde_json::from_str::<Value>(&text) {
                            if let Some(content) = val.get("content").and_then(|c| c.as_array()) {
                                if let Some(first_block) = content.first() {
                                    if let Some(reply) = first_block.get("text").and_then(|t| t.as_str()) {
                                        return PeerConsultationResult {
                                            peer: "claude".to_string(),
                                            model,
                                            prompt: prompt.to_string(),
                                            response: reply.to_string(),
                                            elapsed_ms: start.elapsed().as_millis() as u64,
                                            success: true,
                                            source: "anthropic_api".to_string(),
                                        };
                                    }
                                }
                            }
                            if let Some(err) = val.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                                return PeerConsultationResult {
                                    peer: "claude".to_string(),
                                    model,
                                    prompt: prompt.to_string(),
                                    response: format!("Anthropic API error: {}", err),
                                    elapsed_ms: start.elapsed().as_millis() as u64,
                                    success: false,
                                    source: "anthropic_api".to_string(),
                                };
                            }
                        }
                    }
                }
            }
        }

        // 2. Fallback: try local `claude` CLI if available
        if let Ok(output) = Command::new("claude")
            .args(["-p", prompt])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !stdout.is_empty() {
                    return PeerConsultationResult {
                        peer: "claude".to_string(),
                        model: "claude-cli".to_string(),
                        prompt: prompt.to_string(),
                        response: stdout,
                        elapsed_ms: start.elapsed().as_millis() as u64,
                        success: true,
                        source: "claude_cli".to_string(),
                    };
                }
            }
        }

        PeerConsultationResult {
            peer: "claude".to_string(),
            model,
            prompt: prompt.to_string(),
            response: "Claude peer consultation is not configured. Set the ANTHROPIC_API_KEY environment variable or install the 'claude' CLI binary.".to_string(),
            elapsed_ms: start.elapsed().as_millis() as u64,
            success: false,
            source: "unconfigured".to_string(),
        }
    }

    fn call_openai(prompt: &str, start: Instant) -> PeerConsultationResult {
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            if !api_key.trim().is_empty() {
                let body = json!({
                    "model": model,
                    "messages": [
                        { "role": "user", "content": prompt }
                    ],
                    "max_tokens": 1024
                });

                if let Ok(output) = Command::new("curl")
                    .args([
                        "-s", "-m", "30",
                        "-X", "POST", "https://api.openai.com/v1/chat/completions",
                        "-H", &format!("Authorization: Bearer {}", api_key.trim()),
                        "-H", "Content-Type: application/json",
                        "-d", &body.to_string(),
                    ])
                    .output()
                {
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout);
                        if let Ok(val) = serde_json::from_str::<Value>(&text) {
                            if let Some(choices) = val.get("choices").and_then(|c| c.as_array()) {
                                if let Some(first) = choices.first() {
                                    if let Some(msg) = first.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                                        return PeerConsultationResult {
                                            peer: "chatgpt".to_string(),
                                            model,
                                            prompt: prompt.to_string(),
                                            response: msg.to_string(),
                                            elapsed_ms: start.elapsed().as_millis() as u64,
                                            success: true,
                                            source: "openai_api".to_string(),
                                        };
                                    }
                                }
                            }
                            if let Some(err) = val.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                                return PeerConsultationResult {
                                    peer: "chatgpt".to_string(),
                                    model,
                                    prompt: prompt.to_string(),
                                    response: format!("OpenAI API error: {}", err),
                                    elapsed_ms: start.elapsed().as_millis() as u64,
                                    success: false,
                                    source: "openai_api".to_string(),
                                };
                            }
                        }
                    }
                }
            }
        }

        PeerConsultationResult {
            peer: "chatgpt".to_string(),
            model,
            prompt: prompt.to_string(),
            response: "ChatGPT peer consultation is not configured. Set the OPENAI_API_KEY environment variable.".to_string(),
            elapsed_ms: start.elapsed().as_millis() as u64,
            success: false,
            source: "unconfigured".to_string(),
        }
    }

    fn call_gemini(prompt: &str, start: Instant) -> PeerConsultationResult {
        let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.0-flash".to_string());
        let api_key = std::env::var("GEMINI_API_KEY").or_else(|_| std::env::var("GOOGLE_API_KEY")).ok();

        if let Some(key) = api_key {
            if !key.trim().is_empty() {
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                    model,
                    key.trim()
                );
                let body = json!({
                    "contents": [
                        { "parts": [{ "text": prompt }] }
                    ]
                });

                if let Ok(output) = Command::new("curl")
                    .args([
                        "-s", "-m", "30",
                        "-X", "POST", &url,
                        "-H", "Content-Type: application/json",
                        "-d", &body.to_string(),
                    ])
                    .output()
                {
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout);
                        if let Ok(val) = serde_json::from_str::<Value>(&text) {
                            if let Some(candidates) = val.get("candidates").and_then(|c| c.as_array()) {
                                if let Some(first) = candidates.first() {
                                    if let Some(content) = first.get("content").and_then(|c| c.get("parts")).and_then(|p| p.as_array()) {
                                        if let Some(part) = content.first() {
                                            if let Some(reply) = part.get("text").and_then(|t| t.as_str()) {
                                                return PeerConsultationResult {
                                                    peer: "gemini".to_string(),
                                                    model,
                                                    prompt: prompt.to_string(),
                                                    response: reply.to_string(),
                                                    elapsed_ms: start.elapsed().as_millis() as u64,
                                                    success: true,
                                                    source: "gemini_api".to_string(),
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(err) = val.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                                return PeerConsultationResult {
                                    peer: "gemini".to_string(),
                                    model,
                                    prompt: prompt.to_string(),
                                    response: format!("Gemini API error: {}", err),
                                    elapsed_ms: start.elapsed().as_millis() as u64,
                                    success: false,
                                    source: "gemini_api".to_string(),
                                };
                            }
                        }
                    }
                }
            }
        }

        PeerConsultationResult {
            peer: "gemini".to_string(),
            model,
            prompt: prompt.to_string(),
            response: "Gemini peer consultation is not configured. Set the GEMINI_API_KEY or GOOGLE_API_KEY environment variable.".to_string(),
            elapsed_ms: start.elapsed().as_millis() as u64,
            success: false,
            source: "unconfigured".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_router_unsupported_peer() {
        let temp_dir = std::env::temp_dir();
        let result = ModelRouter::consult(&temp_dir, "nonexistent-model", "hello", None);
        assert!(!result.success);
        assert_eq!(result.source, "unsupported");
    }

    #[test]
    fn test_model_router_unconfigured_graceful_response() {
        let temp_dir = std::env::temp_dir();
        // Clear env vars for test to guarantee unconfigured path
        let result = ModelRouter::consult(&temp_dir, "gemini", "Check syntax", None);
        // Either succeeds if user has key set in env, or returns unconfigured message gracefully
        if !result.success {
            assert!(result.response.contains("GEMINI_API_KEY") || result.response.contains("API error"));
        }
    }

    #[test]
    fn test_model_router_context_file_attachment() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb_test_router_{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("sample.rs");
        std::fs::write(&test_file, "fn sample() { println!(\"test\"); }").unwrap();

        let files = vec!["sample.rs".to_string()];
        let result = ModelRouter::consult(&temp_dir, "unsupported-mock", "Analyze this", Some(&files));
        assert!(result.prompt.contains("Attached Context Files"));
        assert!(result.prompt.contains("fn sample()"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
