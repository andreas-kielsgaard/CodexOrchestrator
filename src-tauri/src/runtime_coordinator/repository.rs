use crate::persistence::ActiveDatabase;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Binding {
    pub coordinator_id: String,
    pub device_id: String,
    pub execution_device_id: String,
}
pub(crate) struct Bindings(pub Arc<ActiveDatabase>);
impl Bindings {
    pub fn all(&self) -> Result<Vec<Binding>, String> {
        self.0.read("read coordinator device bindings", |db| {
            let mut statement = db.prepare("SELECT coordinator_id,device_id,execution_device_id FROM runtime_coordinator_device_bindings").map_err(|e| e.to_string())?;
            let rows = statement.query_map([], |row| Ok(Binding {
                coordinator_id: row.get(0)?,
                device_id: row.get(1)?,
                execution_device_id: row.get(2)?,
            })).map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        }).map_err(|e| e.into_string())
    }
    pub fn associate(&self, b: &Binding) -> Result<(), String> {
        self.0.write("associate coordinator device",|db|{let present=db.query_row("SELECT COUNT(*) FROM execution_devices WHERE device_id=?1",[&b.execution_device_id],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string())?;if present==0||b.execution_device_id=="local"{return Err("Select an existing remote execution device".into())}db.execute("INSERT INTO runtime_coordinator_device_bindings VALUES(?1,?2,?3) ON CONFLICT(coordinator_id,device_id) DO UPDATE SET execution_device_id=excluded.execution_device_id",params![b.coordinator_id,b.device_id,b.execution_device_id]).map(|_|()).map_err(|e|e.to_string())}).map_err(|e|e.into_string())
    }
    pub fn active(&self, coordinator: &str, device: &str) -> Result<u64, String> {
        self.0.read("check coordinated device activity",|db|{db.query_row("SELECT COUNT(*) FROM execution_device_activity_leases WHERE device_id=(SELECT execution_device_id FROM runtime_coordinator_device_bindings WHERE coordinator_id=?1 AND device_id=?2)",params![coordinator,device],|r|r.get(0)).map_err(|e|e.to_string())}).map_err(|e|e.into_string())
    }
}
pub(crate) fn managed(database: &ActiveDatabase, id: &str) -> Result<bool, String> {
    database
        .read("check lifecycle ownership", |db| {
            db.query_row(
                "SELECT 1 FROM runtime_coordinator_device_bindings WHERE execution_device_id=?1",
                [id],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map(|r| r.is_some())
            .map_err(|e| e.to_string())
        })
        .map_err(|e| e.into_string())
}
