use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;

use crate::crdt::{Anchor, Operation};

pub struct Database {
    pub conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(forge_path: &Path) -> Result<Self> {
        let db_path = forge_path.join("forge.db");
        let conn = Connection::open(db_path)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open(forge_path: &str) -> Result<Self> {
        Self::new(Path::new(forge_path))
    }

    pub fn initialize(&self) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS operations (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                op_type TEXT NOT NULL,
                op_data BLOB NOT NULL,
                parent_ops TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS anchors (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                stable_id TEXT NOT NULL UNIQUE,
                position BLOB NOT NULL,
                created_at TEXT NOT NULL,
                message TEXT,
                tags TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS annotations (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                anchor_id TEXT,
                line INTEGER NOT NULL,
                content TEXT NOT NULL,
                author TEXT NOT NULL,
                created_at TEXT NOT NULL,
                is_ai BOOLEAN NOT NULL,
                FOREIGN KEY(anchor_id) REFERENCES anchors(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ops_file_time
             ON operations(file_path, timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_anchors_file
             ON anchors(file_path)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_annotations_file
             ON annotations(file_path, line)",
            [],
        )?;

        Ok(())
    }

    pub fn store_operation(&self, op: &Operation) -> Result<bool> {
        let conn = self.conn.lock();
        let op_data = bincode::serialize(&op.op_type)?;
        let parent_ops = serde_json::to_string(&op.parent_ops)?;

        conn.execute(
            "INSERT OR IGNORE INTO operations (id, timestamp, actor_id, file_path, op_type, op_data, parent_ops)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                op.id.to_string(),
                op.timestamp.to_rfc3339(),
                op.actor_id,
                op.file_path,
                format!("{:?}", op.op_type).split('{').next().unwrap(),
                op_data,
                parent_ops,
            ],
        )
        .map(|changes| changes > 0)
        .map_err(Into::into)
    }

    pub fn get_operations(&self, file: Option<&Path>, limit: usize) -> Result<Vec<Operation>> {
        let conn = self.conn.lock();

        let query = if let Some(f) = file {
            format!(
                "SELECT id, timestamp, actor_id, file_path, op_data, parent_ops
                 FROM operations
                 WHERE file_path = '{}'
                 ORDER BY timestamp DESC
                 LIMIT {}",
                f.display(),
                limit
            )
        } else {
            format!(
                "SELECT id, timestamp, actor_id, file_path, op_data, parent_ops
                 FROM operations
                 ORDER BY timestamp DESC
                 LIMIT {}",
                limit
            )
        };

        let mut stmt = conn.prepare(&query)?;
        let ops = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let timestamp: String = row.get(1)?;
            let actor_id: String = row.get(2)?;
            let file_path: String = row.get(3)?;
            let op_data: Vec<u8> = row.get(4)?;
            let parent_ops: String = row.get(5)?;

            let op_type = bincode::deserialize(&op_data).unwrap();
            let parents: Vec<uuid::Uuid> = serde_json::from_str(&parent_ops).unwrap();

            Ok(Operation {
                id: uuid::Uuid::parse_str(&id).unwrap(),
                timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp)
                    .unwrap()
                    .into(),
                actor_id,
                file_path,
                op_type,
                parent_ops: parents,
            })
        })?;

        Ok(ops.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn store_anchor(&self, anchor: &Anchor) -> Result<()> {
        let conn = self.conn.lock();
        let position = bincode::serialize(&anchor.position)?;
        let tags = serde_json::to_string(&anchor.tags)?;

        conn.execute(
            "INSERT INTO anchors (id, file_path, stable_id, position, created_at, message, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                anchor.id.to_string(),
                anchor.file_path,
                anchor.stable_id,
                position,
                anchor.created_at.to_rfc3339(),
                anchor.message,
                tags,
            ],
        )?;

        Ok(())
    }
}
