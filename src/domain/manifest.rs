use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaxonomyCategory {
    pub id: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaxonomyConfig {
    #[serde(default = "default_taxonomy_categories")]
    pub categories: Vec<TaxonomyCategory>,
}

fn default_taxonomy_categories() -> Vec<TaxonomyCategory> {
    vec![
        TaxonomyCategory {
            id: "architecture".to_string(),
            label: "Architecture & Contracts".to_string(),
            description: "System boundaries, protocols, database schemas, and parity guarantees".to_string(),
        },
        TaxonomyCategory {
            id: "behavior".to_string(),
            label: "Code & Agent Behavior".to_string(),
            description: "Comment hygiene, naming conventions, and testing requirements".to_string(),
        },
        TaxonomyCategory {
            id: "deployment".to_string(),
            label: "Deployment & Safety".to_string(),
            description: "Environment configs, migrations, rollback safety, and runtime constraints".to_string(),
        },
        TaxonomyCategory {
            id: "security".to_string(),
            label: "Security & Authority".to_string(),
            description: "Authentication, secrets, external API calls, and autonomy bounds".to_string(),
        },
    ]
}

impl Default for TaxonomyConfig {
    fn default() -> Self {
        Self {
            categories: default_taxonomy_categories(),
        }
    }
}

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

    #[serde(
        default = "default_knowledge_roots",
        deserialize_with = "deserialize_knowledge_roots",
        alias = "docs_root"
    )]
    pub knowledge_roots: Vec<String>,

    #[serde(default = "default_decisions_path")]
    pub decisions_path: String,

    #[serde(default = "default_risks_path")]
    pub risks_path: String,

    #[serde(default = "default_directives_path")]
    pub directives_path: String,

    #[serde(default = "default_memory_path")]
    pub memory_path: String,

    #[serde(default)]
    pub taxonomy: TaxonomyConfig,

    #[serde(default)]
    pub harnesses: crate::domain::HarnessConfig,

    #[serde(default)]
    pub settings: KbSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KbSettings {
    #[serde(default = "default_max_briefing_directives")]
    pub max_briefing_directives: usize,

    #[serde(default = "default_stale_days_threshold")]
    pub stale_days_threshold: i64,

    #[serde(default = "default_audit_max_lines")]
    pub audit_max_lines: usize,

    #[serde(default = "default_audit_max_depth")]
    pub audit_max_depth: usize,

    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_mouse_enabled")]
    pub mouse_enabled: bool,
}

fn default_max_briefing_directives() -> usize {
    5
}

fn default_stale_days_threshold() -> i64 {
    90
}

fn default_audit_max_lines() -> usize {
    250
}

fn default_audit_max_depth() -> usize {
    3
}

fn default_theme() -> String {
    "cyberpunk".to_string()
}

fn default_mouse_enabled() -> bool {
    true
}

impl Default for KbSettings {
    fn default() -> Self {
        Self {
            max_briefing_directives: default_max_briefing_directives(),
            stale_days_threshold: default_stale_days_threshold(),
            audit_max_lines: default_audit_max_lines(),
            audit_max_depth: default_audit_max_depth(),
            theme: default_theme(),
            mouse_enabled: default_mouse_enabled(),
        }
    }
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

fn default_knowledge_roots() -> Vec<String> {
    vec![
        "projects".to_string(),
        "shared".to_string(),
        "docs".to_string(),
    ]
}

fn deserialize_knowledge_roots<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }

    match Option::<StringOrVec>::deserialize(deserializer)? {
        Some(StringOrVec::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(default_knowledge_roots())
            } else {
                Ok(vec![trimmed.to_string()])
            }
        }
        Some(StringOrVec::Vec(v)) => {
            if v.is_empty() {
                Ok(default_knowledge_roots())
            } else {
                Ok(v)
            }
        }
        None => Ok(default_knowledge_roots()),
    }
}

fn default_decisions_path() -> String {
    "decisions".to_string()
}

fn default_risks_path() -> String {
    "risks".to_string()
}

fn default_directives_path() -> String {
    "directives".to_string()
}

fn default_memory_path() -> String {
    "memory".to_string()
}

impl Default for RepoManifest {
    fn default() -> Self {
        Self {
            collection_id: default_collection_id(),
            name: default_name(),
            version: default_version(),
            default_profile: default_profile(),
            knowledge_roots: default_knowledge_roots(),
            decisions_path: default_decisions_path(),
            risks_path: default_risks_path(),
            directives_path: default_directives_path(),
            memory_path: default_memory_path(),
            taxonomy: TaxonomyConfig::default(),
            harnesses: crate::domain::HarnessConfig::default(),
            settings: KbSettings::default(),
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

        // Create standard documentation & governance directories
        fs::create_dir_all(root.join(&manifest.decisions_path))
            .map_err(|e| format!("failed to create decisions path: {}", e))?;
        fs::create_dir_all(root.join(&manifest.risks_path))
            .map_err(|e| format!("failed to create risks path: {}", e))?;
        fs::create_dir_all(root.join(&manifest.directives_path))
            .map_err(|e| format!("failed to create directives path: {}", e))?;
        fs::create_dir_all(root.join(&manifest.memory_path))
            .map_err(|e| format!("failed to create memory path: {}", e))?;
        for k_root in &manifest.knowledge_roots {
            fs::create_dir_all(root.join(k_root))
                .map_err(|e| format!("failed to create knowledge root '{}': {}", k_root, e))?;
        }

        fs::write(&manifest_path, json.as_bytes())
            .map_err(|e| format!("failed to write hyperkb.json: {}", e))?;

        Ok((manifest, manifest_path))
    }

    /// Returns the primary docs or knowledge root for display / backwards compatibility.
    pub fn docs_root(&self) -> &str {
        self.knowledge_roots
            .first()
            .map(|s| s.as_str())
            .unwrap_or("docs")
    }

    pub fn save<P: AsRef<Path>>(&self, root: P) -> Result<(), String> {
        let manifest_path = root.as_ref().join(Self::FILE_NAME);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize manifest: {}", e))?;
        fs::write(&manifest_path, json.as_bytes())
            .map_err(|e| format!("failed to write hyperkb.json: {}", e))?;
        Ok(())
    }

    pub fn is_valid_category(&self, cat: &str) -> bool {
        self.taxonomy.categories.iter().any(|c| c.id.eq_ignore_ascii_case(cat))
    }

    pub fn add_category(
        &mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<(), String> {
        let id_str = id.into();
        if self.is_valid_category(&id_str) {
            return Err(format!("Category '{}' already exists in taxonomy", id_str));
        }
        self.taxonomy.categories.push(TaxonomyCategory {
            id: id_str,
            label: label.into(),
            description: description.into(),
        });
        Ok(())
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
        assert_eq!(m.decisions_path, "decisions");
        assert_eq!(m.risks_path, "risks");
        assert_eq!(m.directives_path, "directives");
        assert_eq!(m.memory_path, "memory");
        assert_eq!(m.knowledge_roots, vec!["projects", "shared", "docs"]);
        assert_eq!(m.docs_root(), "projects");
        assert!(m.is_valid_category("architecture"));
        assert!(m.is_valid_category("BEHAVIOR"));
        assert!(!m.is_valid_category("unknown_category"));
    }

    #[test]
    fn test_manifest_backward_compatibility_docs_root() {
        // Test that an older manifest JSON containing "docs_root": "my_docs" deserializes properly into knowledge_roots
        let json = r#"{
            "collection_id": "legacy_repo",
            "name": "Legacy Repo",
            "docs_root": "legacy_docs"
        }"#;

        let m: RepoManifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.collection_id, "legacy_repo");
        assert_eq!(m.knowledge_roots, vec!["legacy_docs".to_string()]);
        assert_eq!(m.docs_root(), "legacy_docs");
    }

    #[test]
    fn test_manifest_init_and_load() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-manifest-test-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);

        let (manifest, path) = RepoManifest::init(&temp_dir, Some("My Project"), Some("my-proj")).unwrap();
        assert_eq!(manifest.name, "My Project");
        assert_eq!(manifest.collection_id, "my-proj");
        assert!(path.exists());
        assert!(temp_dir.join("decisions").exists());
        assert!(temp_dir.join("risks").exists());
        assert!(temp_dir.join("directives").exists());
        assert!(temp_dir.join("projects").exists());
        assert!(temp_dir.join("shared").exists());
        assert!(temp_dir.join("docs").exists());

        let loaded = RepoManifest::load_or_default(&temp_dir);
        assert_eq!(loaded, manifest);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_taxonomy_validation_and_addition() {
        let mut m = RepoManifest::default();
        assert!(m.is_valid_category("security"));
        assert!(!m.is_valid_category("database"));

        m.add_category("database", "Database & Storage", "Schema migrations and queries").unwrap();
        assert!(m.is_valid_category("database"));

        let duplicate_err = m.add_category("database", "Dup", "Dup");
        assert!(duplicate_err.is_err());
    }

    #[test]
    fn test_kb_settings_defaults_and_roundtrip() {
        let settings = KbSettings::default();
        assert_eq!(settings.max_briefing_directives, 5);
        assert_eq!(settings.stale_days_threshold, 90);
        assert_eq!(settings.audit_max_lines, 250);
        assert_eq!(settings.audit_max_depth, 3);
        assert_eq!(settings.theme, "cyberpunk");
        assert!(settings.mouse_enabled);

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: KbSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, settings);

        // Test backward compatibility: empty json should deserialize with defaults
        let empty_json = "{}";
        let from_empty: KbSettings = serde_json::from_str(empty_json).unwrap();
        assert_eq!(from_empty, settings);
    }
}
