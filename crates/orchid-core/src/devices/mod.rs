//! Stored device observations and explicitly addressed machine operations.
pub mod operations;
use crate::{persistence::ActiveDatabase, synchronization::DeviceDescription};
use rusqlite::params;
use std::sync::Arc;
pub const SCHEMA:&str="CREATE TABLE IF NOT EXISTS orchid_device_descriptions(id TEXT PRIMARY KEY,payload_json TEXT NOT NULL,observed_at TEXT NOT NULL);";
#[derive(Clone)]
pub struct DeviceDirectory(pub Arc<ActiveDatabase>);
impl DeviceDirectory {
    pub fn record(&self, description: &DeviceDescription) -> Result<(), String> {
        if description.id.is_empty() {
            return Err("Device identity is required".into());
        }
        self.0.write("record device observation",|db|db.execute("INSERT INTO orchid_device_descriptions VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET payload_json=excluded.payload_json,observed_at=excluded.observed_at",params![description.id,serde_json::to_string(description).map_err(|e|e.to_string())?,chrono::Utc::now().to_rfc3339()]).map_err(|e|e.to_string())).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn list(&self) -> Result<Vec<DeviceDescription>, String> {
        self.0
            .read(
                "read device observations",
                |db| -> Result<Vec<DeviceDescription>, String> {
                    let rows = db
                        .prepare("SELECT payload_json FROM orchid_device_descriptions ORDER BY id")
                        .map_err(|e| e.to_string())?
                        .query_map([], |r| r.get::<_, String>(0))
                        .map_err(|e| e.to_string())?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| e.to_string())?;
                    rows.into_iter()
                        .map(|j| serde_json::from_str(&j).map_err(|e| e.to_string()))
                        .collect()
                },
            )
            .map_err(|e| e.to_string())
    }
}
