use crate::core::metadata::MetadataParser;
use crate::domain::{IndexReport, RecordMeta};
use crate::storage::queries::Queries;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Scanner;

impl Scanner {
    const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024;

    pub fn checksum(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    pub fn discover_markdown<P: AsRef<Path>>(root: P) -> Vec<PathBuf> {
        let mut results = Vec::new();
        Self::walk_dir(root.as_ref(), root.as_ref(), &mut results);
        results.sort();
        results
    }

    fn walk_dir(current: &Path, root: &Path, results: &mut Vec<PathBuf>) {
        let entries = match fs::read_dir(current) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if file_name.starts_with('.') || file_name == "node_modules" || file_name == "target" {
                continue;
            }

            if path.is_dir() {
                Self::walk_dir(&path, root, results);
            } else if path.extension().map_or(false, |ext| ext == "md") {
                if let Ok(rel) = path.strip_prefix(root) {
                    results.push(rel.to_path_buf());
                }
            }
        }
    }

    pub fn index_directory<P: AsRef<Path>>(
        conn: &Connection,
        root: P,
        collection_id: &str,
    ) -> Result<IndexReport, rusqlite::Error> {
        let root = root.as_ref();
        let paths = Self::discover_markdown(root);

        let mut report = IndexReport {
            scanned: paths.len(),
            added: 0,
            updated: 0,
            unchanged: 0,
            removed: 0,
            errors: 0,
            prune_refused: false,
            coverage_complete: true,
            pending_moves: Vec::new(),
            unavailable_roots: Vec::new(),
        };

        for rel_path in paths {
            let full_path = root.join(&rel_path);
            let metadata = match fs::metadata(&full_path) {
                Ok(m) => m,
                Err(_) => {
                    report.errors += 1;
                    continue;
                }
            };

            if metadata.len() > Self::MAX_FILE_SIZE {
                report.errors += 1;
                continue;
            }

            let content = match fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => {
                    report.errors += 1;
                    continue;
                }
            };

            let checksum = Self::checksum(content.as_bytes());
            let rel_str = rel_path.to_string_lossy().replace('\\', "/");

            let existing_checksum: Result<Option<String>, _> = conn
                .query_row(
                    "SELECT checksum FROM documents WHERE collection_id = ?1 AND path = ?2;",
                    [collection_id, &rel_str],
                    |row| row.get(0),
                )
                .map(Some)
                .or_else(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    err => Err(err),
                });

            match existing_checksum {
                Ok(Some(old_sum)) if old_sum == checksum => {
                    report.unchanged += 1;
                    continue;
                }
                Ok(Some(_)) => {
                    report.updated += 1;
                }
                Ok(None) => {
                    report.added += 1;
                }
                Err(_) => {
                    report.errors += 1;
                    continue;
                }
            }

            let parsed = match MetadataParser::parse(&content) {
                Ok(p) => p,
                Err(_) => {
                    report.errors += 1;
                    continue;
                }
            };

            let meta = parsed.meta.unwrap_or_else(|| RecordMeta {
                id: format!("doc_{}", &checksum[..16]),
                kind: "document".into(),
                status: "unknown".into(),
                owner: "".into(),
                issue: None,
                paths: Vec::new(),
                versions: Vec::new(),
                environments: Vec::new(),
                supersedes: None,
                delegation: None,
            });

            let topic = rel_path
                .parent()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "General".into());

            let search_text = format!("{} {}", parsed.title, parsed.body);

            Queries::upsert_document(
                conn,
                &meta.id,
                collection_id,
                &rel_str,
                &topic,
                &parsed.title,
                &content,
                &search_text,
                &meta,
                &checksum,
            )?;

            if let Ok(dir) = crate::domain::Directive::parse_markdown(&content, collection_id) {
                let _ = Queries::upsert_directive(conn, &dir);
            }
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_incremental_indexing() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb_test_{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&temp_dir)?;

        let doc1 = temp_dir.join("README.md");
        let mut f1 = File::create(&doc1)?;
        writeln!(f1, "# Main Project README\nWelcome to our codebase.")?;

        let db = Database::open_in_memory("coll_inc", "prof_inc")?;

        let report1 = Scanner::index_directory(db.conn(), &temp_dir, "coll_inc")?;
        assert_eq!(report1.scanned, 1);
        assert_eq!(report1.added, 1);
        assert_eq!(report1.unchanged, 0);

        let report2 = Scanner::index_directory(db.conn(), &temp_dir, "coll_inc")?;
        assert_eq!(report2.scanned, 1);
        assert_eq!(report2.added, 0);
        assert_eq!(report2.unchanged, 1);

        fs::remove_dir_all(&temp_dir)?;
        Ok(())
    }
}

