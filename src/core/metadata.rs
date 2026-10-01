use crate::domain::RecordMeta;

pub struct ParsedMetadata<'a> {
    pub meta: Option<RecordMeta>,
    pub body: &'a str,
    pub title: String,
}

pub struct MetadataParser;

impl MetadataParser {
    const HEADER_PREFIX: &'static str = "---hyperkb\n";
    const HEADER_SUFFIX: &'static str = "\n---\n";
    const MAX_HEADER_LEN: usize = 16 * 1024;

    pub fn parse(content: &str) -> Result<ParsedMetadata<'_>, String> {
        let (meta, body) = if content.starts_with(Self::HEADER_PREFIX) {
            let rest = &content[Self::HEADER_PREFIX.len()..];
            let end_idx = rest
                .find(Self::HEADER_SUFFIX)
                .ok_or_else(|| "missing closing '---' for HyperKB metadata block".to_string())?;

            if end_idx > Self::MAX_HEADER_LEN {
                return Err("HyperKB metadata block exceeds 16 KiB limit".to_string());
            }

            let json_str = &rest[..end_idx];
            let meta: RecordMeta = serde_json::from_str(json_str)
                .map_err(|e| format!("invalid HyperKB metadata JSON: {}", e))?;

            Self::validate_meta(&meta)?;

            let remaining_body = &rest[end_idx + Self::HEADER_SUFFIX.len()..];
            (Some(meta), remaining_body)
        } else {
            (None, content)
        };

        let title = Self::extract_title(body);

        Ok(ParsedMetadata { meta, body, title })
    }

    pub fn extract_title(body: &str) -> String {
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") {
                let title = trimmed[2..].trim();
                if !title.is_empty() {
                    return title.to_string();
                }
            }
        }
        "Untitled Document".to_string()
    }

    fn validate_meta(m: &RecordMeta) -> Result<(), String> {
        if m.id.len() < 8 || m.id.len() > 128 {
            return Err("metadata id must be 8–128 characters long".to_string());
        }
        if m.owner.trim().is_empty() {
            return Err("metadata owner is required".to_string());
        }
        match m.kind.as_str() {
            "decision" => match m.status.as_str() {
                "proposed" | "accepted" | "superseded" => Ok(()),
                _ => Err("decision status must be proposed, accepted or superseded".to_string()),
            },
            "risk" => {
                match m.status.as_str() {
                    "open" | "fixed" | "superseded" => {}
                    _ => return Err("risk status must be open, fixed or superseded".to_string()),
                }
                if m.paths.is_empty() {
                    return Err("risk requires at least one affected path".to_string());
                }
                Ok(())
            }
            "spec" | "plan" | "document" => Ok(()),
            other => Err(format!("unsupported record kind '{}'", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_decision_metadata() {
        let content = r#"---hyperkb
{
  "id": "dec_2026_001",
  "kind": "decision",
  "status": "accepted",
  "owner": "wiqar"
}
---
# Move to Rust Architecture

We are replacing Go with Rust for extreme resource efficiency.
"#;
        let parsed = MetadataParser::parse(content).expect("should parse successfully");
        assert_eq!(parsed.title, "Move to Rust Architecture");
        let meta = parsed.meta.expect("should have metadata");
        assert_eq!(meta.id, "dec_2026_001");
        assert_eq!(meta.kind, "decision");
        assert_eq!(meta.status, "accepted");
        assert!(parsed.body.contains("We are replacing Go with Rust"));
    }

    #[test]
    fn test_parse_plain_markdown_candidate() {
        let content = "# Getting Started Guide\n\nWelcome to HyperKB.";
        let parsed = MetadataParser::parse(content).expect("should parse raw markdown");
        assert_eq!(parsed.title, "Getting Started Guide");
        assert!(parsed.meta.is_none());
        assert_eq!(parsed.body, content);
    }
}

