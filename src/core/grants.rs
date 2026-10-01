use crate::domain::{ActionKind, Actor, AuthorityGrant, GrantConstraints};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct GrantStore;

impl GrantStore {
    fn grants_dir<P: AsRef<Path>>(root: P) -> PathBuf {
        root.as_ref().join(".hyperkb").join("grants")
    }

    fn grant_file<P: AsRef<Path>>(root: P, grant_id: Uuid) -> PathBuf {
        Self::grants_dir(root).join(format!("{}.json", grant_id))
    }

    /// Saves an AuthorityGrant to the local `.hyperkb/grants` store.
    pub fn save_grant<P: AsRef<Path>>(root: P, grant: &AuthorityGrant) -> Result<(), String> {
        let dir = Self::grants_dir(root.as_ref());
        fs::create_dir_all(&dir)
            .map_err(|e| format!("failed to create grants directory: {}", e))?;

        let path = Self::grant_file(root, grant.grant_id);
        let json_str = serde_json::to_string_pretty(grant)
            .map_err(|e| format!("failed to serialize grant: {}", e))?;

        fs::write(&path, json_str.as_bytes())
            .map_err(|e| format!("failed to write grant file: {}", e))?;

        Ok(())
    }

    /// Loads an AuthorityGrant by UUID from the local `.hyperkb/grants` store.
    pub fn load_grant<P: AsRef<Path>>(root: P, grant_id: Uuid) -> Result<AuthorityGrant, String> {
        let path = Self::grant_file(root, grant_id);
        if !path.exists() {
            return Err(format!("authority grant '{}' not found", grant_id));
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read grant file: {}", e))?;

        let grant: AuthorityGrant = serde_json::from_str(&content)
            .map_err(|e| format!("invalid grant JSON: {}", e))?;

        Ok(grant)
    }

    /// Lists all AuthorityGrants stored in `.hyperkb/grants`.
    pub fn list_grants<P: AsRef<Path>>(root: P) -> Result<Vec<AuthorityGrant>, String> {
        let dir = Self::grants_dir(root);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut grants = Vec::new();
        let entries = fs::read_dir(dir)
            .map_err(|e| format!("failed to read grants directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(grant) = serde_json::from_str::<AuthorityGrant>(&content) {
                        grants.push(grant);
                    }
                }
            }
        }

        grants.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(grants)
    }

    /// Revokes/deletes an AuthorityGrant.
    pub fn revoke_grant<P: AsRef<Path>>(root: P, grant_id: Uuid) -> Result<bool, String> {
        let path = Self::grant_file(root, grant_id);
        if !path.exists() {
            return Ok(false);
        }

        fs::remove_file(path)
            .map_err(|e| format!("failed to remove grant file: {}", e))?;

        Ok(true)
    }

    /// Issues and persists a new AuthorityGrant for an agent delegate.
    pub fn issue_grant<P: AsRef<Path>>(
        root: P,
        grantee: &str,
        granted_by: &str,
        allowed_actions: Vec<ActionKind>,
        allowed_scope_patterns: Vec<String>,
        constraints: GrantConstraints,
    ) -> Result<AuthorityGrant, String> {
        let grantee = grantee.trim();
        let granted_by = granted_by.trim();

        if grantee.is_empty() {
            return Err("grantee (agent ID) is required".to_string());
        }
        if granted_by.is_empty() {
            return Err("granted_by (human authorizer/principal) is required".to_string());
        }
        if allowed_actions.is_empty() {
            return Err("grant requires at least one allowed action".to_string());
        }
        if allowed_scope_patterns.is_empty() {
            return Err("grant requires at least one allowed scope pattern".to_string());
        }

        let grant = AuthorityGrant {
            grant_id: Uuid::now_v7(),
            grantee: grantee.to_string(),
            granted_by: granted_by.to_string(),
            allowed_actions,
            allowed_scope_patterns,
            constraints,
            created_at: Utc::now(),
            signature: None,
        };

        Self::save_grant(root, &grant)?;
        Ok(grant)
    }

    /// Resolves an Actor from either a human identity or an agent citing a stored grant.
    pub fn resolve_actor<P: AsRef<Path>>(
        root: P,
        agent_id: Option<&str>,
        grant_id: Option<Uuid>,
        human_username: Option<&str>,
    ) -> Result<Actor, String> {
        match grant_id {
            Some(gid) => {
                let grant = Self::load_grant(root, gid)?;

                // Enforce expiration
                if let Some(exp) = grant.constraints.expires_at {
                    if Utc::now() > exp {
                        return Err(format!(
                            "authority grant '{}' expired at {}",
                            gid,
                            exp.to_rfc3339()
                        ));
                    }
                }

                // Verify grantee matches if agent_id is explicitly supplied
                let effective_agent_id = match agent_id {
                    Some(aid) if !aid.trim().is_empty() => {
                        let aid = aid.trim();
                        if !grant.grantee.is_empty() && grant.grantee != "*" && grant.grantee != aid {
                            return Err(format!(
                                "agent identity '{}' does not match grant grantee '{}'",
                                aid, grant.grantee
                            ));
                        }
                        aid.to_string()
                    }
                    _ => grant.grantee.clone(),
                };

                Ok(Actor::Agent {
                    agent_id: effective_agent_id,
                    model: "delegated-agent".to_string(),
                    grant,
                })
            }
            None => {
                let username = human_username
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Developer")
                    .to_string();
                Ok(Actor::Human { username })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grant_issue_load_list_revoke() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-grants-test-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);

        let grant = GrantStore::issue_grant(
            &temp_dir,
            "claude-3-7-sonnet",
            "wiqar",
            vec![ActionKind::AcceptDecision, ActionKind::AcknowledgeRisk],
            vec!["docs/decisions/rfc/**".to_string()],
            GrantConstraints::default(),
        )
        .unwrap();

        assert_eq!(grant.grantee, "claude-3-7-sonnet");
        assert_eq!(grant.granted_by, "wiqar");

        // Load grant
        let loaded = GrantStore::load_grant(&temp_dir, grant.grant_id).unwrap();
        assert_eq!(loaded.grant_id, grant.grant_id);
        assert_eq!(loaded.granted_by, "wiqar");

        // List grants
        let list = GrantStore::list_grants(&temp_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].grant_id, grant.grant_id);

        // Resolve actor
        let actor = GrantStore::resolve_actor(&temp_dir, Some("claude-3-7-sonnet"), Some(grant.grant_id), None)
            .unwrap();
        assert_eq!(actor.name(), "claude-3-7-sonnet");
        assert_eq!(actor.responsible_owner(), "wiqar");
        assert!(actor.can_perform(&ActionKind::AcceptDecision, "docs/decisions/rfc/001.md"));
        assert!(!actor.can_perform(&ActionKind::AcceptDecision, "src/core/security.rs"));

        // Revoke grant
        let revoked = GrantStore::revoke_grant(&temp_dir, grant.grant_id).unwrap();
        assert!(revoked);

        assert!(GrantStore::load_grant(&temp_dir, grant.grant_id).is_err());
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
