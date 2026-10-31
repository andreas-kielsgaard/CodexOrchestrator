use crate::protocol::*;
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

pub struct Repository {
    connection: Mutex<Connection>,
}
impl Repository {
    pub fn open(path: &Path) -> Result<Self, String> {
        let db = Connection::open(path).map_err(|e| e.to_string())?;
        db.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS identity (id TEXT NOT NULL); CREATE TABLE IF NOT EXISTS devices (id TEXT PRIMARY KEY, credential_id TEXT NOT NULL, server_id INTEGER NOT NULL, device_json TEXT NOT NULL, UNIQUE(credential_id,server_id)); CREATE TABLE IF NOT EXISTS operations (id TEXT PRIMARY KEY, device_id TEXT NOT NULL, operation_json TEXT NOT NULL);").map_err(|e|e.to_string())?;
        if db
            .query_row("SELECT id FROM identity LIMIT 1", [], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map_err(|e| e.to_string())?
            .is_none()
        {
            db.execute(
                "INSERT INTO identity VALUES(?1)",
                [uuid::Uuid::new_v4().to_string()],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(Self {
            connection: Mutex::new(db),
        })
    }
    pub fn identity(&self) -> Result<String, String> {
        self.connection
            .lock()
            .unwrap()
            .query_row("SELECT id FROM identity LIMIT 1", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }
    pub fn list(&self) -> Result<Vec<Device>, String> {
        let db = self.connection.lock().unwrap();
        let mut s=db.prepare("SELECT device_json FROM devices ORDER BY json_extract(device_json,'$.displayName')").map_err(|e|e.to_string())?;
        let rows = s
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.map(|r| {
            serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
        })
        .collect()
    }
    pub fn device(&self, id: &str) -> Result<(Device, String), String> {
        let db = self.connection.lock().unwrap();
        let (json, cred) = db
            .query_row(
                "SELECT device_json,credential_id FROM devices WHERE id=?1",
                [id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .map_err(|_| "Device was not found".to_string())?;
        Ok((
            serde_json::from_str(&json).map_err(|e| e.to_string())?,
            cred,
        ))
    }
    pub fn save_device(&self, d: &Device, credential: &str) -> Result<(), String> {
        self.connection.lock().unwrap().execute("INSERT INTO devices VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET device_json=excluded.device_json",params![d.id,credential,d.server.id,serde_json::to_string(d).map_err(|e|e.to_string())?]).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn operation(&self, id: &str) -> Result<Option<Operation>, String> {
        let json = self
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT operation_json FROM operations WHERE id=?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        json.map(|j| serde_json::from_str(&j).map_err(|e| e.to_string()))
            .transpose()
    }
    pub fn save_state(&self, d: &Device, credential: &str) -> Result<(), String> {
        let mut db = self.connection.lock().unwrap();
        let tx = db.transaction().map_err(|e| e.to_string())?;
        if let Some(o) = &d.operation {
            tx.execute("INSERT INTO operations VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET operation_json=excluded.operation_json",params![o.id,o.device_id,serde_json::to_string(o).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
        }
        tx.execute(
            "UPDATE devices SET device_json=?1 WHERE id=?2 AND credential_id=?3",
            params![
                serde_json::to_string(d).map_err(|e| e.to_string())?,
                d.id,
                credential
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
}
