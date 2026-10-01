use std::collections::HashMap;
use std::sync::OnceLock;

static SYNONYM_MAP: OnceLock<HashMap<&'static str, &'static [&'static str]>> = OnceLock::new();

/// Curated offline technical synonym clusters for software engineering & architecture.
const SYNONYM_CLUSTERS: &[&[&str]] = &[
    // Authentication / Authorization / Identity
    &["auth", "authentication", "authorization", "credentials", "token", "jwt", "oauth"],
    // Database / Storage
    &["db", "database", "sqlite", "storage", "sql", "postgres", "mysql"],
    // Performance / Latency / Throughput
    &["perf", "performance", "latency", "throughput", "speed", "benchmark", "profiling", "bottleneck"],
    // Errors / Failures / Crashes
    &["err", "error", "failure", "panic", "exception", "fault", "crash"],
    // Memory / Leaks / Allocation
    &["mem", "memory", "leak", "allocation", "heap", "stack", "oom"],
    // Security / Vulnerabilities
    &["sec", "security", "vulnerability", "cve", "exploit", "sanitize"],
    // Synchronization / Concurrency / Locks
    &["sync", "synchronization", "concurrency", "async", "lock", "mutex", "deadlock", "race"],
    // Configuration / Environment
    &["cfg", "config", "configuration", "settings", "options", "env", "environment"],
    // Networking / Sockets / HTTP
    &["net", "network", "socket", "http", "tcp", "connection", "endpoint"],
    // Logging / Observability / Telemetry
    &["log", "logging", "logger", "tracing", "audit", "telemetry"],
    // Repository / Version Control
    &["repo", "repository", "git", "vcs"],
    // API / Interfaces
    &["api", "interface", "endpoint", "rest", "rpc", "grpc"],
    // CLI / Terminal / Commands
    &["cli", "command", "terminal", "flag", "args"],
    // Testing / Specs / Assertions
    &["test", "testing", "spec", "mock", "assert", "fixture"],
    // Documentation / Specs
    &["doc", "docs", "documentation", "specification", "manual", "guide"],
    // Dependencies / Packages / Crates
    &["dep", "deps", "dependency", "dependencies", "package", "crate"],
    // Messaging / Events
    &["msg", "message", "event", "payload", "queue"],
    // Directory / Filesystem
    &["dir", "directory", "folder", "path"],
];

fn get_synonym_map() -> &'static HashMap<&'static str, &'static [&'static str]> {
    SYNONYM_MAP.get_or_init(|| {
        let mut map = HashMap::new();
        for cluster in SYNONYM_CLUSTERS {
            for &term in *cluster {
                map.insert(term, *cluster);
            }
        }
        map
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueryToken {
    Word(String),
    Phrase(String),
}

pub struct QueryExpander;

impl QueryExpander {
    /// Parses a user search query into words and quoted phrases.
    pub fn parse_tokens(query: &str) -> Vec<QueryToken> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = query.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }

            // Quoted phrase
            if chars[i] == '"' {
                i += 1;
                let mut phrase = String::new();
                while i < chars.len() && chars[i] != '"' {
                    phrase.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() && chars[i] == '"' {
                    i += 1; // consume closing quote
                }
                let trimmed = phrase.trim();
                if !trimmed.is_empty() {
                    tokens.push(QueryToken::Phrase(trimmed.to_string()));
                }
            } else {
                // Word token
                let mut word = String::new();
                while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '"' {
                    if chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '-' {
                        word.push(chars[i]);
                    }
                    i += 1;
                }
                let trimmed = word.trim();
                if !trimmed.is_empty() {
                    tokens.push(QueryToken::Word(trimmed.to_lowercase()));
                }
            }
        }

        tokens
    }

    /// Expands tokens using technical synonym clusters into an FTS5 MATCH expression.
    pub fn build_fts5_expr(tokens: &[QueryToken], join_op: &str) -> String {
        if tokens.is_empty() {
            return String::new();
        }

        let map = get_synonym_map();
        let mut groups = Vec::new();

        for token in tokens {
            match token {
                QueryToken::Phrase(phrase) => {
                    // Escape internal double quotes if any
                    let clean = phrase.replace('"', "\"\"");
                    groups.push(format!("\"{}\"", clean));
                }
                QueryToken::Word(word) => {
                    if let Some(synonyms) = map.get(word.as_str()) {
                        let sub_expr = synonyms
                            .iter()
                            .map(|s| format!("\"{}\"", s))
                            .collect::<Vec<_>>()
                            .join(" OR ");
                        groups.push(format!("({})", sub_expr));
                    } else {
                        groups.push(format!("\"{}\"", word));
                    }
                }
            }
        }

        groups.join(&format!(" {} ", join_op))
    }

    /// Generates the primary (AND) and broadened (OR) FTS5 expressions for a search query.
    pub fn expand(query: &str) -> (String, Option<String>) {
        let tokens = Self::parse_tokens(query);
        if tokens.is_empty() {
            return (String::new(), None);
        }

        let and_expr = Self::build_fts5_expr(&tokens, "AND");
        let or_expr = if tokens.len() >= 2 {
            Some(Self::build_fts5_expr(&tokens, "OR"))
        } else {
            None
        };

        (and_expr, or_expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tokens() {
        let q = r#"auth "sqlite busy" db"#;
        let tokens = QueryExpander::parse_tokens(q);
        assert_eq!(
            tokens,
            vec![
                QueryToken::Word("auth".to_string()),
                QueryToken::Phrase("sqlite busy".to_string()),
                QueryToken::Word("db".to_string()),
            ]
        );
    }

    #[test]
    fn test_expand_single_synonym() {
        let (and_expr, or_expr) = QueryExpander::expand("auth");
        assert!(and_expr.starts_with('('));
        assert!(and_expr.contains("\"auth\""));
        assert!(and_expr.contains("\"authentication\""));
        assert!(and_expr.contains("\"token\""));
        assert!(or_expr.is_none());
    }

    #[test]
    fn test_expand_multi_word_with_phrases() {
        let (and_expr, or_expr) = QueryExpander::expand(r#"auth "sqlite WAL""#);
        assert!(and_expr.contains(" AND "));
        assert!(and_expr.contains("\"sqlite WAL\""));
        assert!(and_expr.contains("\"authentication\""));

        assert!(or_expr.is_some());
        let or_str = or_expr.unwrap();
        assert!(or_str.contains(" OR "));
        assert!(or_str.contains("\"sqlite WAL\""));
    }

    #[test]
    fn test_word_without_synonyms() {
        let (and_expr, _) = QueryExpander::expand("customxyz");
        assert_eq!(and_expr, "\"customxyz\"");
    }
}
