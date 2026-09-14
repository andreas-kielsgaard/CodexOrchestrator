use crate::persistence::ActiveDatabase;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub(crate) const DEVICE_LOCATION_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS repository_device_locations (repository_id TEXT NOT NULL REFERENCES registered_repositories(repository_id) ON DELETE CASCADE, device_id TEXT NOT NULL, repository_root TEXT NOT NULL, PRIMARY KEY(repository_id,device_id));";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryDeviceLocation {
    pub(crate) repository_id: String,
    pub(crate) device_id: String,
    pub(crate) repository_root: String,
}

pub(crate) struct RepositoryDeviceLocations(pub(crate) Arc<ActiveDatabase>);
impl RepositoryDeviceLocations {
    pub(crate) fn list(&self) -> Result<Vec<RepositoryDeviceLocation>, String> {
        self.0.read("list repository device locations", |connection| {
            let mut query = connection.prepare("SELECT repository_id,device_id,repository_root FROM repository_device_locations ORDER BY repository_id,device_id").map_err(|e| e.to_string())?;
            let rows = query.query_map([], |row| Ok(RepositoryDeviceLocation { repository_id: row.get(0)?, device_id: row.get(1)?, repository_root: row.get(2)? })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
            Ok(rows)
        }).map_err(|e| e.into_string())
    }
    pub(crate) fn save(&self, location: RepositoryDeviceLocation) -> Result<(), String> {
        if location.repository_root.trim().is_empty() || location.device_id.trim().is_empty() {
            return Err("Select a device and enter its repository path".into());
        }
        self.0.write("save repository device location", |transaction| {
            transaction.execute("INSERT INTO repository_device_locations(repository_id,device_id,repository_root) VALUES (?1,?2,?3) ON CONFLICT(repository_id,device_id) DO UPDATE SET repository_root=excluded.repository_root", params![location.repository_id, location.device_id, location.repository_root]).map_err(|e| e.to_string())?;
            Ok(())
        }).map_err(|e| e.into_string())
    }
}
