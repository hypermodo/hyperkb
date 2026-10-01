use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoManifest {
    #[serde(default = "default_collection_id")]
    pub collection_id: String,

    #[serde(default = "default_name")]
    pub name: String,

    #[serde(default = "default_version")]
    pub version: String,

    #[serde(default = "default_profile")]
    pub default_profile: String,

    #[serde(default = "default_docs_root")]
    pub docs_root: String,

    #[serde(default = "default_decisions_path")]
    pub decisions_path: String,

    #[serde(default = "default_risks_path")]
    pub risks_path: String,

    #[serde(default = "default_memory_path")]
    pub memory_path: String,
}

fn default_collection_id() -> String {
    "local_collection".to_string()
}

fn default_name() -> String {
    "Repository Knowledge Base".to_string()
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_profile() -> String {
    "default_profile".to_string()
}

fn default_docs_root() -> String {
    "docs".to_string()
}

fn default_decisions_path() -> String {
    "docs/decisions".to_string()
}

fn default_risks_path() -> String {
    "docs/risks".to_string()
}

fn default_memory_path() -> String {
    "docs/memory".to_string()
}

impl Default for RepoManifest {
    fn default() -> Self {
        Self {
            collection_id: default_collection_id(),
            name: default_name(),
            version: default_version(),
            default_profile: default_profile(),
            docs_root: default_docs_root(),
            decisions_path: default_decisions_path(),
            risks_path: default_risks_path(),
            memory_path: default_memory_path(),
        }
    }
}

impl RepoManifest {
    pub const FILE_NAME: &'static str = "hyperkb.json";

    /// Loads `hyperkb.json` from the repository root, falling back to default configuration if not present.
    pub fn load_or_default<P: AsRef<Path>>(root: P) -> Self {
        let manifest_path = root.as_ref().join(Self::FILE_NAME);
        if manifest_path.exists() {
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<RepoManifest>(&content) {
                    return manifest;
                }
            }
        }
        Self::default()
    }

    /// Initializes a new `hyperkb.json` and standard directories in the given root path.
    pub fn init<P: AsRef<Path>>(
        root: P,
        name: Option<&str>,
        collection_id: Option<&str>,
    ) -> Result<(Self, PathBuf), String> {
        let root = root.as_ref();
        let manifest_path = root.join(Self::FILE_NAME);

        let default_coll = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "local_collection".to_string());

        let coll_id = collection_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or(default_coll);

        let repo_name = name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| coll_id.clone());

        let manifest = RepoManifest {
            collection_id: coll_id,
            name: repo_name,
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("failed to serialize manifest: {}", e))?;

        // Create standard documentation directories
        fs::create_dir_all(root.join(&manifest.decisions_path))
            .map_err(|e| format!("failed to create decisions path: {}", e))?;
        fs::create_dir_all(root.join(&manifest.risks_path))
            .map_err(|e| format!("failed to create risks path: {}", e))?;

        fs::write(&manifest_path, json.as_bytes())
            .map_err(|e| format!("failed to write hyperkb.json: {}", e))?;

        Ok((manifest, manifest_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_manifest_defaults() {
        let m = RepoManifest::default();
        assert_eq!(m.collection_id, "local_collection");
        assert_eq!(m.decisions_path, "docs/decisions");
        assert_eq!(m.risks_path, "docs/risks");
    }

    #[test]
    fn test_manifest_init_and_load() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-manifest-test-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);

        let (manifest, path) = RepoManifest::init(&temp_dir, Some("My Project"), Some("my-proj")).unwrap();
        assert_eq!(manifest.name, "My Project");
        assert_eq!(manifest.collection_id, "my-proj");
        assert!(path.exists());
        assert!(temp_dir.join("docs/decisions").exists());
        assert!(temp_dir.join("docs/risks").exists());

        let loaded = RepoManifest::load_or_default(&temp_dir);
        assert_eq!(loaded, manifest);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
