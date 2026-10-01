use crate::domain::{Document, RiskApplicability, RiskCheck, RiskMatch};

pub struct RiskEngine;

impl RiskEngine {
    pub fn matches_path(pattern: &str, target_path: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let target_parts: Vec<&str> = target_path.split('/').filter(|s| !s.is_empty()).collect();

        Self::match_segments(&pattern_parts, &target_parts)
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
                for pattern in risk_paths {
                    if Self::matches_path(pattern, target) {
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
    }
}

