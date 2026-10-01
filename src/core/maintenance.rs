use crate::storage::Database;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub collection_id: String,
    pub profile_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupReport {
    pub path: String,
    pub kept: usize,
    pub removed: usize,
}

pub struct MaintenanceManager;

impl MaintenanceManager {
    /// Creates a self-contained, verified atomic snapshot and rotates managed snapshots according to the retention policy.
    pub fn create_backup<P: AsRef<Path>>(
        root: P,
        db: &Database,
        collection_id: &str,
        profile_id: &str,
        keep: usize,
    ) -> Result<BackupReport, String> {
        if !(1..=1000).contains(&keep) {
            return Err("backup retention must be between 1 and 1000".to_string());
        }

        let root_path = root.as_ref();
        let backups_dir = root_path.join(".hyperkb").join("backups");
        fs::create_dir_all(&backups_dir)
            .map_err(|e| format!("failed to create backups directory: {}", e))?;

        // 1. Stage in a temporary incoming folder
        let incoming_name = format!(".incoming-{}", Uuid::now_v7());
        let incoming_dir = backups_dir.join(&incoming_name);
        fs::create_dir_all(&incoming_dir)
            .map_err(|e| format!("failed to create incoming staging directory: {}", e))?;

        let snapshot_db = incoming_dir.join("hyperkb.db");
        if let Err(e) = db.snapshot(&snapshot_db) {
            let _ = fs::remove_dir_all(&incoming_dir);
            return Err(format!("failed to snapshot database: {}", e));
        }

        // 2. Write backup manifest
        let manifest = BackupManifest {
            collection_id: collection_id.to_string(),
            profile_id: profile_id.to_string(),
            created_at: Utc::now().to_rfc3339(),
        };
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("failed to serialize manifest: {}", e))?;

        let manifest_path = incoming_dir.join("manifest.json");
        if let Err(e) = fs::write(&manifest_path, manifest_json.as_bytes()) {
            let _ = fs::remove_dir_all(&incoming_dir);
            return Err(format!("failed to write manifest: {}", e));
        }

        // 3. Verify snapshot integrity without mutating it
        {
            let snapshot_db_res = Database::open(&snapshot_db, collection_id, profile_id);
            match snapshot_db_res {
                Ok(snap) => {
                    if let Err(e) = snap.integrity_check() {
                        let _ = fs::remove_dir_all(&incoming_dir);
                        return Err(format!("snapshot failed integrity check: {}", e));
                    }
                }
                Err(e) => {
                    let _ = fs::remove_dir_all(&incoming_dir);
                    return Err(format!("failed to verify snapshot database: {}", e));
                }
            }
        } // snap dropped here

        // 4. Atomically promote incoming snapshot to final timestamped name
        let timestamp = Utc::now().format("%Y%m%dT%H%M%S%6fZ").to_string();
        let rand_suffix = Uuid::now_v7().to_string();
        let snapshot_name = format!("snapshot-{}-{}", timestamp, &rand_suffix[rand_suffix.len() - 8..]);
        let final_dir = backups_dir.join(&snapshot_name);

        if let Err(e) = fs::rename(&incoming_dir, &final_dir) {
            let _ = fs::remove_dir_all(&incoming_dir);
            return Err(format!("failed to promote backup snapshot: {}", e));
        }

        // 5. Enforce retention policy (keep newest `keep` snapshots)
        let (kept, removed) = Self::enforce_retention(&backups_dir, keep)?;

        Ok(BackupReport {
            path: final_dir.to_string_lossy().to_string(),
            kept,
            removed,
        })
    }

    /// Enforces retention policy on managed snapshots. Unmanaged directories are never touched.
    fn enforce_retention(backups_dir: &Path, keep: usize) -> Result<(usize, usize), String> {
        let entries = fs::read_dir(backups_dir)
            .map_err(|e| format!("failed to read backups directory: {}", e))?;

        let mut managed: Vec<PathBuf> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy().to_string();
            // Managed snapshots start with 'snapshot-' and contain manifest.json
            if file_name.starts_with("snapshot-") && path.join("manifest.json").exists() {
                managed.push(path);
            }
        }

        // Sort ascending by path name (timestamp formatted names will naturally sort chronologically)
        managed.sort();

        let total = managed.len();
        if total <= keep {
            return Ok((total, 0));
        }

        let to_remove_count = total - keep;
        let mut removed = 0;

        for path in managed.iter().take(to_remove_count) {
            if fs::remove_dir_all(path).is_ok() {
                removed += 1;
            }
        }

        Ok((total - removed, removed))
    }

    /// Lists all managed snapshots.
    pub fn list_backups<P: AsRef<Path>>(root: P) -> Result<Vec<String>, String> {
        let backups_dir = root.as_ref().join(".hyperkb").join("backups");
        if !backups_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&backups_dir)
            .map_err(|e| format!("failed to read backups directory: {}", e))?;

        let mut snapshots = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() && name.starts_with("snapshot-") && path.join("manifest.json").exists() {
                snapshots.push(name);
            }
        }

        snapshots.sort();
        snapshots.reverse();
        Ok(snapshots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_and_retention_rotation() {
        let temp_dir = std::env::temp_dir().join(format!("hyperkb-maint-test-{}", Uuid::now_v7()));
        let _ = fs::create_dir_all(&temp_dir);
        let root = &temp_dir;

        let db_path = root.join(".hyperkb").join("hyperkb.db");
        let db = Database::create(&db_path, "coll_1", "prof_1").unwrap();

        // 1. Create first backup (keep = 2)
        let rep1 = MaintenanceManager::create_backup(root, &db, "coll_1", "prof_1", 2).unwrap();
        assert_eq!(rep1.kept, 1);
        assert_eq!(rep1.removed, 0);

        // 2. Create second backup
        let rep2 = MaintenanceManager::create_backup(root, &db, "coll_1", "prof_1", 2).unwrap();
        assert_eq!(rep2.kept, 2);
        assert_eq!(rep2.removed, 0);

        // 3. Create third backup - should rotate and remove the oldest one!
        let rep3 = MaintenanceManager::create_backup(root, &db, "coll_1", "prof_1", 2).unwrap();
        assert_eq!(rep3.kept, 2);
        assert_eq!(rep3.removed, 1);

        // 4. Verify list_backups returns exactly 2 snapshots
        let list = MaintenanceManager::list_backups(root).unwrap();
        assert_eq!(list.len(), 2);
    }
}
