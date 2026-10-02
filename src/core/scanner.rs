use crate::core::metadata::MetadataParser;
use crate::domain::{IndexReport, RecordMeta, RepoManifest};
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

    pub fn is_tombstone(rel_str: &str, meta: &RecordMeta, raw_content: &str) -> bool {
        let lower_path = rel_str.to_ascii_lowercase();
        if lower_path.contains("/_archive/")
            || lower_path.starts_with("_archive/")
            || lower_path.contains("/_draft/")
            || lower_path.starts_with("_draft/")
            || lower_path.contains("/_archived/")
            || lower_path.starts_with("_archived/")
            || lower_path.contains("/refuted/")
            || lower_path.starts_with("refuted/")
            || lower_path.contains("/_trash/")
            || lower_path.starts_with("_trash/")
        {
            return true;
        }

        let lower_status = meta.status.to_ascii_lowercase();
        if lower_status == "archived"
            || lower_status == "refuted"
            || lower_status == "tombstoned"
            || lower_status == "tombstone"
        {
            return true;
        }

        if meta.kind.eq_ignore_ascii_case("plan")
            && (lower_status == "completed"
                || lower_status == "done"
                || lower_status == "abandoned")
        {
            return true;
        }

        // Check frontmatter for explicit tombstone: true or archived: true
        if raw_content.starts_with("---") {
            if let Some(end) = raw_content[3..].find("---") {
                let fm = &raw_content[3..3 + end];
                if fm.contains("tombstone: true") || fm.contains("archived: true") {
                    return true;
                }
            }
        }

        false
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

    /// Discovers markdown documents across all configured knowledge roots and governance paths,
    /// strictly excluding directives which are indexed into their own dedicated table.
    pub fn discover_workspace_markdown(
        workspace_root: &Path,
        manifest: &RepoManifest,
    ) -> Vec<PathBuf> {
        let directives_dir = workspace_root.join(&manifest.directives_path);
        let mut candidate_dirs: Vec<PathBuf> = Vec::new();

        for k_root in &manifest.knowledge_roots {
            let p = workspace_root.join(k_root);
            if p.exists() && !candidate_dirs.iter().any(|existing| p.starts_with(existing)) {
                candidate_dirs.push(p);
            }
        }

        let dec_path = workspace_root.join(&manifest.decisions_path);
        if dec_path.exists() && !candidate_dirs.iter().any(|existing| dec_path.starts_with(existing)) {
            candidate_dirs.push(dec_path);
        }

        let risk_path = workspace_root.join(&manifest.risks_path);
        if risk_path.exists() && !candidate_dirs.iter().any(|existing| risk_path.starts_with(existing)) {
            candidate_dirs.push(risk_path);
        }

        // If none of the specialized roots exist on disk, fall back to scanning workspace root
        if candidate_dirs.is_empty() {
            candidate_dirs.push(workspace_root.to_path_buf());
        }

        let mut results = Vec::new();
        for dir in candidate_dirs {
            Self::walk_workspace_dir(&dir, workspace_root, &directives_dir, &mut results);
        }

        results.sort();
        results.dedup();
        results
    }

    fn walk_workspace_dir(
        current: &Path,
        workspace_root: &Path,
        directives_dir: &Path,
        results: &mut Vec<PathBuf>,
    ) {
        if directives_dir.exists() && current.starts_with(directives_dir) {
            return;
        }

        let entries = match fs::read_dir(current) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if file_name.starts_with('.')
                || file_name == "node_modules"
                || file_name == "target"
                || file_name == "dist"
                || file_name == "build"
                || file_name == ".venv"
                || file_name == "venv"
            {
                continue;
            }

            if directives_dir.exists() && path.starts_with(directives_dir) {
                continue;
            }

            if path.is_dir() {
                Self::walk_workspace_dir(&path, workspace_root, directives_dir, results);
            } else if path.extension().map_or(false, |ext| ext == "md") {
                if let Ok(rel) = path.strip_prefix(workspace_root) {
                    results.push(rel.to_path_buf());
                }
            }
        }
    }

    /// Indexes all knowledge documents across the workspace (multi-root) and separates
    /// policy directives into the directives table.
    pub fn index_workspace<P: AsRef<Path>>(
        conn: &Connection,
        workspace_root: P,
        manifest: &RepoManifest,
    ) -> Result<IndexReport, rusqlite::Error> {
        let root = workspace_root.as_ref();
        let collection_id = &manifest.collection_id;
        let paths = Self::discover_workspace_markdown(root, manifest);

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

            let existing_doc: Result<Option<(String, bool)>, _> = conn
                .query_row(
                    "SELECT checksum, is_tombstone FROM documents WHERE collection_id = ?1 AND path = ?2;",
                    [collection_id, &rel_str],
                    |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
                )
                .map(Some)
                .or_else(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    err => Err(err),
                });

            match existing_doc {
                Ok(Some((ref old_sum, old_tomb))) if old_sum == &checksum => {
                    let path_tomb = rel_str.contains("/_archive/")
                        || rel_str.contains("/_draft/")
                        || rel_str.contains("/refuted/")
                        || rel_str.contains("/_trash/")
                        || rel_str.starts_with("_archive/")
                        || rel_str.starts_with("_draft/");
                    if path_tomb != old_tomb {
                        let _ = conn.execute(
                            "UPDATE documents SET is_tombstone = ?1 WHERE collection_id = ?2 AND path = ?3;",
                            rusqlite::params![if path_tomb { 1 } else { 0 }, collection_id, &rel_str],
                        );
                    }
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

            // Directives MUST NEVER be indexed into documents table
            if let Some(ref m) = parsed.meta {
                if m.kind.eq_ignore_ascii_case("directive") {
                    if let Ok(dir) = crate::domain::Directive::parse_markdown(&content, collection_id) {
                        let _ = Queries::upsert_directive(conn, &dir);
                    }
                    continue;
                }
            } else if let Ok(dir) = crate::domain::Directive::parse_markdown(&content, collection_id) {
                let _ = Queries::upsert_directive(conn, &dir);
                continue;
            }

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
            let is_tomb = Self::is_tombstone(&rel_str, &meta, &content);

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
                is_tomb,
            )?;
        }

        // Also index dedicated directives directory into directives table
        let directives_dir = root.join(&manifest.directives_path);
        if directives_dir.exists() {
            let mut dir_paths = Vec::new();
            Self::walk_dir(&directives_dir, &directives_dir, &mut dir_paths);
            for rel in dir_paths {
                let full = directives_dir.join(rel);
                if let Ok(content) = fs::read_to_string(&full) {
                    if let Ok(dir) = crate::domain::Directive::parse_markdown(&content, collection_id) {
                        let _ = Queries::upsert_directive(conn, &dir);
                    }
                }
            }
        }

        Ok(report)
    }

    pub fn index_directory<P: AsRef<Path>>(
        conn: &Connection,
        root: P,
        collection_id: &str,
    ) -> Result<IndexReport, rusqlite::Error> {
        let root = root.as_ref();
        if root.join(RepoManifest::FILE_NAME).exists() {
            let manifest = RepoManifest::load_or_default(root);
            return Self::index_workspace(conn, root, &manifest);
        }

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

            let existing_doc: Result<Option<(String, bool)>, _> = conn
                .query_row(
                    "SELECT checksum, is_tombstone FROM documents WHERE collection_id = ?1 AND path = ?2;",
                    [collection_id, &rel_str],
                    |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
                )
                .map(Some)
                .or_else(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    err => Err(err),
                });

            match existing_doc {
                Ok(Some((ref old_sum, old_tomb))) if old_sum == &checksum => {
                    let path_tomb = rel_str.contains("/_archive/")
                        || rel_str.contains("/_draft/")
                        || rel_str.contains("/refuted/")
                        || rel_str.contains("/_trash/")
                        || rel_str.starts_with("_archive/")
                        || rel_str.starts_with("_draft/");
                    if path_tomb != old_tomb {
                        let _ = conn.execute(
                            "UPDATE documents SET is_tombstone = ?1 WHERE collection_id = ?2 AND path = ?3;",
                            rusqlite::params![if path_tomb { 1 } else { 0 }, collection_id, &rel_str],
                        );
                    }
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

            // Directives MUST NEVER be indexed into documents table
            if let Some(ref m) = parsed.meta {
                if m.kind.eq_ignore_ascii_case("directive") {
                    if let Ok(dir) = crate::domain::Directive::parse_markdown(&content, collection_id) {
                        let _ = Queries::upsert_directive(conn, &dir);
                    }
                    continue;
                }
            } else if let Ok(dir) = crate::domain::Directive::parse_markdown(&content, collection_id) {
                let _ = Queries::upsert_directive(conn, &dir);
                continue;
            }

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
            let is_tomb = Self::is_tombstone(&rel_str, &meta, &content);

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
                is_tomb,
            )?;
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

    #[test]
    fn test_multi_root_workspace_indexing() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb_test_multi_{}", uuid::Uuid::now_v7()));
        let (manifest, _) = RepoManifest::init(&temp_dir, Some("MultiRoot Repo"), Some("multi_coll"))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // 1. Create project doc
        let proj_dir = temp_dir.join("projects/engine");
        fs::create_dir_all(&proj_dir)?;
        fs::write(
            proj_dir.join("spec.md"),
            "---hyperkb\n{\"id\":\"spec_engine_001\",\"kind\":\"spec\",\"status\":\"active\",\"owner\":\"alice\",\"paths\":[],\"versions\":[],\"environments\":[],\"supersedes\":null,\"delegation\":null}\n---\n# Engine Spec\nArchitecture details.",
        )?;

        // 2. Create shared doc
        let shared_dir = temp_dir.join("shared/protocols");
        fs::create_dir_all(&shared_dir)?;
        fs::write(
            shared_dir.join("auth.md"),
            "---hyperkb\n{\"id\":\"shared_auth_001\",\"kind\":\"document\",\"status\":\"active\",\"owner\":\"alice\",\"paths\":[],\"versions\":[],\"environments\":[],\"supersedes\":null,\"delegation\":null}\n---\n# Auth Protocol\nToken auth.",
        )?;

        // 3. Create top-level decision
        let dec_dir = temp_dir.join("decisions");
        fs::create_dir_all(&dec_dir)?;
        fs::write(
            dec_dir.join("001-storage.md"),
            "---hyperkb\n{\"id\":\"dec_storage_001\",\"kind\":\"decision\",\"status\":\"accepted\",\"owner\":\"alice\",\"paths\":[],\"versions\":[],\"environments\":[],\"supersedes\":null,\"delegation\":null}\n---\n# Decision 001\nWe use SQLite.",
        )?;

        // 4. Create top-level risk
        let risk_dir = temp_dir.join("risks");
        fs::create_dir_all(&risk_dir)?;
        fs::write(
            risk_dir.join("001-lock.md"),
            "---hyperkb\n{\"id\":\"risk_storage_001\",\"kind\":\"risk\",\"status\":\"open\",\"owner\":\"alice\",\"paths\":[\"src/storage/**\"],\"versions\":[],\"environments\":[],\"supersedes\":null,\"delegation\":null}\n---\n# Concurrency Hazard\nBe careful of DB locking.",
        )?;

        // 5. Create directive in directives/
        let dir_dir = temp_dir.join("directives");
        fs::create_dir_all(&dir_dir)?;
        let directive = crate::domain::Directive::new(
            "DIR-ARCH-001",
            "multi_coll",
            "Storage Parity",
            "architecture",
            "alice",
            vec!["src/**".to_string()],
            "strict",
            None,
            "# Storage Parity\nDirectives must never pollute documents table.",
        );
        fs::write(dir_dir.join("DIR-ARCH-001.md"), directive.to_markdown())?;

        let db = Database::open_in_memory("multi_coll", "default_profile")?;

        let report = Scanner::index_workspace(db.conn(), &temp_dir, &manifest)?;
        assert_eq!(report.scanned, 4); // spec, auth, decision, risk (directive excluded from docs scan)
        assert_eq!(report.added, 4);

        // Verify documents table does NOT contain the directive
        let doc_count: i64 = db.conn().query_row(
            "SELECT count(*) FROM documents WHERE collection_id = 'multi_coll' AND kind = 'directive';",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(doc_count, 0, "Directives must NEVER be in documents table");

        // Verify directive IS present in directives table
        let dir_count: i64 = db.conn().query_row(
            "SELECT count(*) FROM directives WHERE collection_id = 'multi_coll' AND id = 'DIR-ARCH-001';",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(dir_count, 1, "Directive must be indexed in directives table");

        // Verify browsing projects
        let opts_proj = crate::domain::BrowseOptions {
            project: Some("engine".to_string()),
            ..Default::default()
        };
        let (proj_docs, _) = Queries::browse(db.conn(), &["multi_coll".to_string()], &opts_proj)?;
        assert_eq!(proj_docs.len(), 1);
        assert_eq!(proj_docs[0].path, "projects/engine/spec.md");

        // Verify browsing decisions
        let opts_dec = crate::domain::BrowseOptions {
            category: "decisions".to_string(),
            ..Default::default()
        };
        let (dec_docs, _) = Queries::browse(db.conn(), &["multi_coll".to_string()], &opts_dec)?;
        assert_eq!(dec_docs.len(), 1);
        assert_eq!(dec_docs[0].path, "decisions/001-storage.md");

        // Verify browsing risks
        let opts_risk = crate::domain::BrowseOptions {
            category: "risks".to_string(),
            ..Default::default()
        };
        let (risk_docs, _) = Queries::browse(db.conn(), &["multi_coll".to_string()], &opts_risk)?;
        assert_eq!(risk_docs.len(), 1);
        assert_eq!(risk_docs[0].path, "risks/001-lock.md");

        // Verify check_work detects the open risk
        let check = Queries::check_work(db.conn(), "multi_coll", &["src/storage/db.rs".to_string()], None, None)?;
        assert_eq!(check.matches.len(), 1);
        assert_eq!(check.matches[0].document.id, "risk_storage_001");
        assert_eq!(check.applicable_directives.len(), 1);
        assert_eq!(check.applicable_directives[0].id, "DIR-ARCH-001");

        fs::remove_dir_all(&temp_dir)?;
        Ok(())
    }
}

