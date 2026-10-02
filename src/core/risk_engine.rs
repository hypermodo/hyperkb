use crate::domain::{Document, RiskApplicability, RiskCheck, RiskMatch};
use std::path::Path;

pub struct RiskEngine;

impl RiskEngine {
    pub fn normalize_path<P: AsRef<Path>>(path: &str, root: Option<P>) -> String {
        let mut p = path.trim().replace('\\', "/");
        if let Some(r) = root {
            let root_ref = r.as_ref();
            if let Ok(canon_root) = root_ref.canonicalize() {
                let canon_str = canon_root.to_string_lossy().replace('\\', "/");
                if p.starts_with(&canon_str) {
                    p = p[canon_str.len()..].to_string();
                }
            }
            let root_str = root_ref.to_string_lossy().replace('\\', "/");
            if p.starts_with(&root_str) {
                p = p[root_str.len()..].to_string();
            }
        }
        let segments: Vec<&str> = p
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .collect();
        segments.join("/")
    }

    pub fn matches_path(pattern: &str, target_path: &str) -> bool {
        let clean_pattern = pattern.trim().replace('\\', "/");
        let clean_target = target_path.trim().replace('\\', "/");

        if clean_pattern == "*" || clean_pattern == "**" || clean_pattern == "**/*" {
            return true;
        }

        let pattern_parts: Vec<&str> = clean_pattern
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .collect();
        let target_parts: Vec<&str> = clean_target
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .collect();

        if pattern_parts.is_empty() {
            return target_parts.is_empty();
        }

        // 1. Direct segment match: e.g. "src/storage/**" matches "src/storage/db.rs"
        if Self::match_segments(&pattern_parts, &target_parts) {
            return true;
        }

        // 2. Generic / Unrooted pattern match against subproject paths
        // If pattern is e.g. "src/**", "*.rs", or "storage/*" (does not start with "projects", "shared", "docs")
        // and target path is inside a project like "projects/alpha/src/storage/db.rs",
        // check if any sub-path of target matches the pattern.
        let is_rooted = matches!(pattern_parts.first().copied(), Some("projects" | "shared" | "docs"));
        if !is_rooted && target_parts.len() > pattern_parts.len() {
            for i in 1..target_parts.len() {
                if Self::match_segments(&pattern_parts, &target_parts[i..]) {
                    return true;
                }
            }
        }

        // 3. Rooted pattern against subproject-relative target
        // If pattern is "projects/alpha/src/**" and target is "src/db.rs"
        if is_rooted && pattern_parts.len() > 2 {
            if Self::match_segments(&pattern_parts[2..], &target_parts) {
                return true;
            }
        }

        false
    }

    fn match_segments(pattern: &[&str], target: &[&str]) -> bool {
        if pattern.is_empty() {
            return target.is_empty();
        }

        if pattern[0] == "**" {
            if pattern.len() == 1 {
                return true;
            }
            for i in 0..=target.len() {
                if Self::match_segments(&pattern[1..], &target[i..]) {
                    return true;
                }
            }
            return false;
        }

        if target.is_empty() {
            return false;
        }

        if Self::match_single_segment(pattern[0], target[0]) {
            return Self::match_segments(&pattern[1..], &target[1..]);
        }

        false
    }

    fn match_single_segment(pattern: &str, segment: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if !pattern.contains('*') {
            return pattern == segment;
        }

        if let Some(prefix) = pattern.strip_suffix('*') {
            return segment.starts_with(prefix);
        }
        if let Some(suffix) = pattern.strip_prefix('*') {
            return segment.ends_with(suffix);
        }

        pattern == segment
    }

    pub fn check_paths(
        open_risks: &[(Document, Vec<String>, Vec<String>, Vec<String>)],
        target_paths: &[String],
        target_version: Option<&str>,
        target_env: Option<&str>,
    ) -> RiskCheck {
        let mut matches = Vec::new();

        for (doc, risk_paths, versions, environments) in open_risks {
            let mut matched_paths = Vec::new();
            for target in target_paths {
                let norm_target = Self::normalize_path(target, None::<&Path>);
                for pattern in risk_paths {
                    if Self::matches_path(pattern, &norm_target) {
                        matched_paths.push(target.clone());
                        break;
                    }
                }
            }

            if !matched_paths.is_empty() {
                let version_matches = target_version.map_or(true, |tv| {
                    versions.is_empty() || versions.iter().any(|v| v == tv)
                });
                let env_matches = target_env.map_or(true, |te| {
                    environments.is_empty() || environments.iter().any(|e| e == te)
                });

                let applicability = if version_matches && env_matches {
                    RiskApplicability::Applies
                } else {
                    RiskApplicability::NotApplicable
                };

                let is_acknowledged = doc.status == crate::domain::DocumentStatus::Acknowledged
                    || doc.declared_status.as_deref() == Some("acknowledged");

                matches.push(RiskMatch {
                    document: doc.clone(),
                    matched_paths,
                    reason: format!("Matches risk path patterns declared in {}", doc.path),
                    applicability,
                    acknowledged: is_acknowledged,
                    acknowledgement: None,
                    external_issue_freshness: None,
                    suppressed: false,
                    suppression_reason: None,
                });
            }
        }

        let has_applicable = matches
            .iter()
            .any(|m| m.applicability == RiskApplicability::Applies);
        let message = if has_applicable {
            format!(
                "WARNING: {} relevant open risk(s) found before editing.",
                matches.len()
            )
        } else {
            "No matching open risks found for selected paths.".to_string()
        };

        RiskCheck {
            matches,
            applicable_directives: Vec::new(),
            hygiene_warnings: Vec::new(),
            historical_candidates: Vec::new(),
            checked_paths: target_paths.to_vec(),
            coverage_complete: true,
            applicability_known: true,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_path_matching() {
        assert!(RiskEngine::matches_path(
            "src/**/*.rs",
            "src/domain/actor.rs"
        ));
        assert!(RiskEngine::matches_path("src/*.rs", "src/main.rs"));
        assert!(!RiskEngine::matches_path("src/*.rs", "src/domain/actor.rs"));
        assert!(RiskEngine::matches_path("auth/tokens.go", "auth/tokens.go"));
        assert!(RiskEngine::matches_path("auth/**", "auth/tokens/jwt.go"));

        // Leading dot and backslashes
        assert!(RiskEngine::matches_path("src/storage/**", "./src/storage/db.rs"));
        assert!(RiskEngine::matches_path("src/storage/**", "src\\storage\\db.rs"));

        // Generic unrooted pattern matching inside a subproject
        assert!(RiskEngine::matches_path("src/**", "projects/adaptive-tuning/src/main.rs"));
        assert!(RiskEngine::matches_path("*.rs", "projects/adaptive-tuning/src/main.rs"));

        // Rooted pattern matching against subproject-relative target
        assert!(RiskEngine::matches_path("projects/adaptive-tuning/src/**", "src/main.rs"));
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            RiskEngine::normalize_path("./src/storage/./queries.rs", None::<&Path>),
            "src/storage/queries.rs"
        );
        assert_eq!(
            RiskEngine::normalize_path("src\\domain\\document.rs", None::<&Path>),
            "src/domain/document.rs"
        );
        let root = Path::new("/var/app/myrepo");
        assert_eq!(
            RiskEngine::normalize_path("/var/app/myrepo/src/main.rs", Some(root)),
            "src/main.rs"
        );
    }
}

