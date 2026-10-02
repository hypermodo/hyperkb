use std::fs;
use std::path::Path;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct FrontmatterSplicer;

pub struct SpliceResult<'a> {
    pub raw_frontmatter: Option<&'a str>,
    pub body: &'a str,
    pub is_hyperkb_header: bool,
}

impl FrontmatterSplicer {
    const YAML_DELIM: &'static str = "---";
    const HYPERKB_HEADER: &'static str = "---hyperkb";

    /// Extracts frontmatter slice and body slice without modifying bytes.
    pub fn split_frontmatter(content: &str) -> SpliceResult<'_> {
        let trimmed_start = content.trim_start();
        if trimmed_start.starts_with(Self::HYPERKB_HEADER) {
            let offset = content.len() - trimmed_start.len();
            let after_header = &content[offset + Self::HYPERKB_HEADER.len()..];
            let after_header = after_header.strip_prefix("\r\n").or_else(|| after_header.strip_prefix('\n')).unwrap_or(after_header);

            if let Some(end_idx) = after_header.find("\n---") {
                let fm = &after_header[..end_idx];
                let rest = &after_header[end_idx + 4..];
                let body = rest.strip_prefix("\r\n").or_else(|| rest.strip_prefix('\n')).unwrap_or(rest);
                return SpliceResult {
                    raw_frontmatter: Some(fm),
                    body,
                    is_hyperkb_header: true,
                };
            }
        } else if trimmed_start.starts_with(Self::YAML_DELIM) {
            let offset = content.len() - trimmed_start.len();
            let after_delim = &content[offset + Self::YAML_DELIM.len()..];
            let after_delim = after_delim.strip_prefix("\r\n").or_else(|| after_delim.strip_prefix('\n')).unwrap_or(after_delim);

            if let Some(end_idx) = after_delim.find("\n---") {
                let fm = &after_delim[..end_idx];
                let rest = &after_delim[end_idx + 4..];
                let body = rest.strip_prefix("\r\n").or_else(|| rest.strip_prefix('\n')).unwrap_or(rest);
                return SpliceResult {
                    raw_frontmatter: Some(fm),
                    body,
                    is_hyperkb_header: false,
                };
            }
        }

        SpliceResult {
            raw_frontmatter: None,
            body: content,
            is_hyperkb_header: false,
        }
    }

    /// Deserializes typed frontmatter from markdown content if present.
    pub fn parse<T: DeserializeOwned>(content: &str) -> Result<(Option<T>, &str), String> {
        let parts = Self::split_frontmatter(content);
        if let Some(raw_fm) = parts.raw_frontmatter {
            if parts.is_hyperkb_header {
                let val: T = serde_json::from_str(raw_fm)
                    .map_err(|e| format!("Failed to parse hyperkb JSON frontmatter: {}", e))?;
                Ok((Some(val), parts.body))
            } else {
                let val: T = serde_yaml::from_str(raw_fm)
                    .map_err(|e| format!("Failed to parse YAML frontmatter: {}", e))?;
                Ok((Some(val), parts.body))
            }
        } else {
            Ok((None, parts.body))
        }
    }

    /// Mutates YAML frontmatter using a closure that manipulates a `serde_yaml::Value`,
    /// reassembling the document while preserving the body verbatim.
    pub fn mutate_yaml<F>(content: &str, mutator: F) -> Result<String, String>
    where
        F: FnOnce(&mut serde_yaml::Value) -> Result<(), String>,
    {
        let parts = Self::split_frontmatter(content);
        let mut yaml_val: serde_yaml::Value = match parts.raw_frontmatter {
            Some(raw) => {
                if parts.is_hyperkb_header {
                    let json_val: serde_json::Value = serde_json::from_str(raw)
                        .map_err(|e| format!("Failed to parse existing JSON frontmatter: {}", e))?;
                    serde_yaml::to_value(json_val)
                        .map_err(|e| format!("Failed to convert JSON frontmatter to YAML: {}", e))?
                } else {
                    serde_yaml::from_str(raw)
                        .map_err(|e| format!("Failed to parse YAML frontmatter: {}", e))?
                }
            }
            None => serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        };

        mutator(&mut yaml_val)?;

        let new_yaml_str = serde_yaml::to_string(&yaml_val)
            .map_err(|e| format!("Failed to serialize mutated YAML frontmatter: {}", e))?;
        let clean_yaml = new_yaml_str.trim();

        let mut output = String::with_capacity(clean_yaml.len() + parts.body.len() + 16);
        output.push_str("---\n");
        output.push_str(clean_yaml);
        output.push_str("\n---\n");
        output.push_str(parts.body);

        Ok(output)
    }

    /// Mutates typed frontmatter by deserializing `T`, mutating it, and reserializing.
    pub fn mutate_typed<T, F>(content: &str, mutator: F) -> Result<String, String>
    where
        T: DeserializeOwned + Serialize + Default,
        F: FnOnce(&mut T) -> Result<(), String>,
    {
        let parts = Self::split_frontmatter(content);
        let mut typed: T = match parts.raw_frontmatter {
            Some(raw) => {
                if parts.is_hyperkb_header {
                    serde_json::from_str(raw)
                        .map_err(|e| format!("Failed to parse JSON frontmatter: {}", e))?
                } else {
                    serde_yaml::from_str(raw)
                        .map_err(|e| format!("Failed to parse YAML frontmatter: {}", e))?
                }
            }
            None => T::default(),
        };

        mutator(&mut typed)?;

        let new_yaml_str = serde_yaml::to_string(&typed)
            .map_err(|e| format!("Failed to serialize mutated frontmatter: {}", e))?;
        let clean_yaml = new_yaml_str.trim();

        let mut output = String::with_capacity(clean_yaml.len() + parts.body.len() + 16);
        output.push_str("---\n");
        output.push_str(clean_yaml);
        output.push_str("\n---\n");
        output.push_str(parts.body);

        Ok(output)
    }

    /// Reads a markdown file, applies the mutation atomically, and writes it back to disk.
    pub fn splice_file<F>(file_path: &Path, mutator: F) -> Result<(), String>
    where
        F: FnOnce(&mut serde_yaml::Value) -> Result<(), String>,
    {
        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Cannot read file '{}': {}", file_path.display(), e))?;

        let modified = Self::mutate_yaml(&content, mutator)?;

        let parent = file_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_file = parent.join(format!(".tmp_{}", uuid::Uuid::now_v7()));

        fs::write(&temp_file, &modified)
            .map_err(|e| format!("Failed to write temp file '{}': {}", temp_file.display(), e))?;

        fs::rename(&temp_file, file_path)
            .map_err(|e| {
                let _ = fs::remove_file(&temp_file);
                format!("Failed to atomically replace '{}': {}", file_path.display(), e)
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_and_preserve_body() {
        let input = "---\nstatus: active\nkind: task\n---\n# Build Parser\n\n| Item | Value |\n|---|---|\n| A | 1 |\n";
        let res = FrontmatterSplicer::split_frontmatter(input);
        assert_eq!(res.raw_frontmatter, Some("status: active\nkind: task"));
        assert_eq!(res.body, "# Build Parser\n\n| Item | Value |\n|---|---|\n| A | 1 |\n");
        assert!(!res.is_hyperkb_header);
    }

    #[test]
    fn test_mutate_yaml_preserves_body_byte_identical() {
        let original_body = "# Project Title\n\nThis is a complex markdown body with tables and ascii art.\n| # | Status |\n|---|---|\n| 1 | ✔ |\n";
        let input = format!("---\nstatus: pending\nhealth: healthy\n---\n{}", original_body);

        let mutated = FrontmatterSplicer::mutate_yaml(&input, |val| {
            if let Some(map) = val.as_mapping_mut() {
                map.insert(
                    serde_yaml::Value::String("status".to_string()),
                    serde_yaml::Value::String("completed".to_string()),
                );
                map.insert(
                    serde_yaml::Value::String("reason".to_string()),
                    serde_yaml::Value::String("verification passed".to_string()),
                );
            }
            Ok(())
        }).expect("mutation should succeed");

        assert!(mutated.starts_with("---\n"));
        assert!(mutated.contains("status: completed"));
        assert!(mutated.contains("reason: verification passed"));
        assert!(mutated.ends_with(original_body));
    }

    #[test]
    fn test_inject_frontmatter_into_plain_markdown() {
        let input = "# Pure Markdown Doc\n\nNo frontmatter was present originally.";
        let mutated = FrontmatterSplicer::mutate_yaml(input, |val| {
            if let Some(map) = val.as_mapping_mut() {
                map.insert(
                    serde_yaml::Value::String("status".to_string()),
                    serde_yaml::Value::String("active".to_string()),
                );
            }
            Ok(())
        }).expect("should inject frontmatter cleanly");

        assert!(mutated.contains("status: active"));
        assert!(mutated.ends_with("# Pure Markdown Doc\n\nNo frontmatter was present originally."));
    }
}
