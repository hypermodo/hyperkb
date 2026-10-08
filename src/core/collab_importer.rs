use crate::domain::{KnowledgeKind, KnowledgeRecord, TaskRecord, TaskState};
use crate::storage::Queries;
use chrono::Utc;
use rusqlite::{params, Connection, OpenFlags};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollabImportReport {
    pub tasks_imported: usize,
    pub knowledge_imported: usize,
    pub source_sha256: String,
}

pub struct CollabImporter;

impl CollabImporter {
    pub fn compute_sha256<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        Ok(hex::encode(hasher.finalize()))
    }

    pub fn normalize_project(project_path: &str, default_project: Option<&str>) -> String {
        if let Some(dp) = default_project {
            let trimmed = dp.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        if let Some(idx) = project_path.find("/projects/") {
            let rest = &project_path[idx + "/projects/".len()..];
            let name = rest.split('/').next().unwrap_or("");
            if !name.is_empty() {
                return name.to_string();
            }
        }

        let trimmed = project_path.trim_end_matches('/');
        if let Some(last) = trimmed.rsplit('/').next() {
            if !last.is_empty() {
                return last.to_lowercase();
            }
        }

        "global".to_string()
    }

    pub fn import<P: AsRef<Path>>(
        source_path: P,
        target_conn: &Connection,
        collection_id: &str,
        default_project: Option<&str>,
    ) -> Result<CollabImportReport, Box<dyn std::error::Error>> {
        let path_ref = source_path.as_ref();
        let sha256_hex = Self::compute_sha256(path_ref)?;

        let source_conn = Connection::open_with_flags(
            path_ref,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        let mut tasks_imported = 0;
        let has_tasks_table: bool = source_conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='tasks';",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if has_tasks_table {
            let mut stmt = source_conn.prepare(
                "SELECT id, session_id, project_path, title, description, status, priority, created_at, updated_at, completed_at FROM tasks;"
            )?;

            let rows = stmt.query_map([], |row| {
                let id: String = row.get(0)?;
                let session_id: Option<String> = row.get(1)?;
                let project_path: String = row.get(2)?;
                let title: String = row.get(3)?;
                let description: Option<String> = row.get(4)?;
                let status_str: String = row.get(5)?;
                let priority: i32 = row.get(6)?;
                let created_at: String = row.get(7)?;
                let updated_at: String = row.get(8)?;
                let completed_at: Option<String> = row.get(9)?;

                Ok((
                    id,
                    session_id,
                    project_path,
                    title,
                    description.unwrap_or_default(),
                    status_str,
                    priority,
                    created_at,
                    updated_at,
                    completed_at,
                ))
            })?;

            for r in rows {
                let (
                    id,
                    session_id,
                    project_path,
                    title,
                    description,
                    status_str,
                    priority,
                    created_at,
                    updated_at,
                    completed_at,
                ) = r?;

                let project = Self::normalize_project(&project_path, default_project);
                let status = TaskState::from_str_loose(&status_str);

                let record = TaskRecord {
                    id,
                    collection_id: collection_id.to_string(),
                    project,
                    session_id,
                    title,
                    description,
                    status,
                    priority,
                    created_at,
                    updated_at,
                    completed_at,
                    metadata_json: "{}".to_string(),
                };

                Queries::upsert_task(target_conn, &record)?;
                tasks_imported += 1;
            }
        }

        let mut knowledge_imported = 0;
        let has_knowledge_table: bool = source_conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='knowledge';",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if has_knowledge_table {
            let mut stmt = source_conn.prepare(
                "SELECT id, project_path, title, content, type, tags, created_at, updated_at FROM knowledge;"
            )?;

            let rows = stmt.query_map([], |row| {
                let id: String = row.get(0)?;
                let project_path: String = row.get(1)?;
                let title: String = row.get(2)?;
                let content: String = row.get(3)?;
                let type_str: String = row.get(4)?;
                let tags: Option<String> = row.get(5)?;
                let created_at: String = row.get(6)?;
                let updated_at: String = row.get(7)?;

                Ok((
                    id,
                    project_path,
                    title,
                    content,
                    type_str,
                    tags.unwrap_or_default(),
                    created_at,
                    updated_at,
                ))
            })?;

            for r in rows {
                let (
                    id,
                    project_path,
                    title,
                    content,
                    type_str,
                    tags,
                    created_at,
                    updated_at,
                ) = r?;

                let project = Self::normalize_project(&project_path, default_project);
                let kind = KnowledgeKind::from_str_loose(&type_str);

                let record = KnowledgeRecord {
                    id,
                    collection_id: collection_id.to_string(),
                    project,
                    title,
                    content,
                    kind,
                    tags,
                    created_at,
                    updated_at,
                    metadata_json: "{}".to_string(),
                };

                Queries::upsert_knowledge(target_conn, &record)?;
                knowledge_imported += 1;
            }
        }

        let mut import_stmt = target_conn.prepare(
            "INSERT OR REPLACE INTO legacy_imports (snapshot_sha256, archive_path, selected_paths, imported_rows, imported_docs, imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);"
        )?;
        import_stmt.execute(params![
            sha256_hex,
            path_ref.to_string_lossy(),
            format!("tasks:{},knowledge:{}", tasks_imported, knowledge_imported),
            (tasks_imported + knowledge_imported) as i64,
            knowledge_imported as i64,
            Utc::now().to_rfc3339(),
        ])?;

        Ok(CollabImportReport {
            tasks_imported,
            knowledge_imported,
            source_sha256: sha256_hex,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;
    use std::fs;

    #[test]
    fn test_normalize_project_path() {
        assert_eq!(
            CollabImporter::normalize_project("/Volumes/ExtSSD/Workspace/ZDP/ZDP-SYSTEM-KB/projects/zdp-monitoring/STATUS.md", None),
            "zdp-monitoring"
        );
        assert_eq!(
            CollabImporter::normalize_project("/Volumes/ExtSSD/Workspace/ZDP", None),
            "zdp"
        );
        assert_eq!(
            CollabImporter::normalize_project("/Volumes/ExtSSD/Workspace/ZDP", Some("custom-target")),
            "custom-target"
        );
    }

    #[test]
    fn test_collab_importer_flow() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = std::env::temp_dir().join(format!("collab-import-test-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&temp_dir)?;
        let legacy_db_path = temp_dir.join("legacy-claude-collab.sqlite");

        {
            let conn = Connection::open(&legacy_db_path)?;
            conn.execute_batch(
                "CREATE TABLE tasks (
                    id TEXT PRIMARY KEY,
                    session_id TEXT,
                    project_path TEXT NOT NULL,
                    title TEXT NOT NULL,
                    description TEXT,
                    status TEXT DEFAULT 'pending',
                    priority INTEGER DEFAULT 0,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    completed_at DATETIME
                );
                CREATE TABLE knowledge (
                    id TEXT PRIMARY KEY,
                    project_path TEXT NOT NULL,
                    title TEXT NOT NULL,
                    content TEXT NOT NULL,
                    type TEXT DEFAULT 'note',
                    tags TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                INSERT INTO tasks (id, session_id, project_path, title, description, status, priority)
                VALUES ('t-01', 's-01', '/Workspace/ZDP/projects/mdm-sync', 'Fix MDM composite FK', 'duplicate FK in prod', 'in_progress', 8);
                INSERT INTO knowledge (id, project_path, title, content, type, tags)
                VALUES ('k-01', '/Workspace/ZDP/projects/mdm-sync', 'Dynamic FK trap', 'stagingTableDDL uses dynamic FK', 'warning', 'mdm,fk,staging');"
            )?;
        }

        let target_db = Database::open_in_memory("coll_main", "prof_1")?;
        let report = CollabImporter::import(&legacy_db_path, target_db.conn(), "coll_main", None)?;

        assert_eq!(report.tasks_imported, 1);
        assert_eq!(report.knowledge_imported, 1);
        assert!(!report.source_sha256.is_empty());

        let tasks = Queries::list_tasks(target_db.conn(), "coll_main", Some("mdm-sync"), None, 10)?;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Fix MDM composite FK");
        assert_eq!(tasks[0].status, TaskState::InProgress);
        assert_eq!(tasks[0].priority, 8);

        let warnings = Queries::list_knowledge(target_db.conn(), "coll_main", Some(KnowledgeKind::Warning), None, 10)?;
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].title, "Dynamic FK trap");
        assert_eq!(warnings[0].kind, KnowledgeKind::Warning);

        let hits = Queries::search_knowledge(target_db.conn(), "coll_main", "stagingTableDDL", None, 10)?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "k-01");

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }
}
