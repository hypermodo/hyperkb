use rusqlite::{Connection, OpenFlags, Result};
use std::fs::OpenOptions;
use std::path::Path;

const SCHEMA_SQL: &str = include_str!("schema.sql");
pub const CURRENT_SCHEMA_VERSION: i32 = 4;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Initializes a brand new HyperKB SQLite database.
    pub fn create<P: AsRef<Path>>(path: P, collection_id: &str, profile_id: &str) -> Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            return Err(rusqlite::Error::ExecuteReturnedResults);
        }

        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        let mut db = Self { conn };
        db.apply_pragmas()?;
        db.init_schema(collection_id, profile_id)?;
        Ok(db)
    }

    /// Opens an existing HyperKB database, verifying collection and profile identities.
    pub fn open<P: AsRef<Path>>(path: P, collection_id: &str, profile_id: &str) -> Result<Self> {
        let conn = Connection::open_with_flags(
            path.as_ref(),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        let db = Self { conn };
        db.apply_pragmas()?;
        db.verify_identities(collection_id, profile_id)?;
        Ok(db)
    }

    /// Opens an existing HyperKB database or creates a new one if it does not exist.
    pub fn open_or_create<P: AsRef<Path>>(path: P, collection_id: &str, profile_id: &str) -> Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            Self::open(path, collection_id, profile_id)
        } else {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            Self::create(path, collection_id, profile_id)
        }
    }

    /// Opens a disposable in-memory database initialized with the schema (for tests and demo).
    pub fn open_in_memory(collection_id: &str, profile_id: &str) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let mut db = Self { conn };
        db.apply_pragmas()?;
        db.init_schema(collection_id, profile_id)?;
        Ok(db)
    }

    fn apply_pragmas(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;",
        )?;
        Ok(())
    }

    fn init_schema(&mut self, collection_id: &str, profile_id: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(SCHEMA_SQL)?;

        tx.execute(
            "INSERT INTO collections (id) VALUES (?1) ON CONFLICT(id) DO NOTHING;",
            [collection_id],
        )?;
        tx.execute(
            "INSERT INTO profiles (id) VALUES (?1) ON CONFLICT(id) DO NOTHING;",
            [profile_id],
        )?;
        tx.execute(
            &format!("PRAGMA user_version = {};", CURRENT_SCHEMA_VERSION),
            [],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn verify_identities(&self, collection_id: &str, profile_id: &str) -> Result<()> {
        let coll_exists: bool = self.conn.query_row(
            "SELECT count(*) > 0 FROM collections WHERE id = ?1;",
            [collection_id],
            |row| row.get(0),
        )?;
        if !coll_exists {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        let prof_exists: bool = self.conn.query_row(
            "SELECT count(*) > 0 FROM profiles WHERE id = ?1;",
            [profile_id],
            |row| row.get(0),
        )?;
        if !prof_exists {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_init_in_memory() -> Result<()> {
        let db = Database::open_in_memory("coll_test_123", "prof_test_456")?;
        let count: i64 = db
            .conn()
            .query_row("SELECT count(*) FROM collections;", [], |r| r.get(0))?;
        assert_eq!(count, 1);

        let user_ver: i32 = db
            .conn()
            .query_row("PRAGMA user_version;", [], |r| r.get(0))?;
        assert_eq!(user_ver, CURRENT_SCHEMA_VERSION);
        Ok(())
    }
}

