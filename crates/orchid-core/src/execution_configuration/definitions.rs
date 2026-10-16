use super::CapabilityProfile;
use rusqlite::{params, Connection};
use serde_json::Value;
pub fn export(db: &Connection) -> Result<Vec<(String, String, Value)>, String> {
    let rows = db
        .prepare("SELECT capability_profile_id,profile_json FROM execution_capability_profiles")
        .map_err(|e| e.to_string())?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|(id, json)| {
            let mut value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            super::repository::hydrate_profile_routes(db, &mut value).map_err(|e| e.to_string())?;
            Ok(("capability_profile".into(), id, value))
        })
        .collect()
}
pub fn validate(id: &str, value: &Value) -> Result<CapabilityProfile, String> {
    let profile: CapabilityProfile =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    profile.validate()?;
    if profile.capability_profile_id != id {
        return Err("Capability Profile identity mismatch".into());
    }
    Ok(profile)
}
pub fn import(db: &Connection, id: &str, value: &Value) -> Result<(), String> {
    let profile = validate(id, value)?;
    for binding in std::iter::once(&profile.execution)
        .chain(profile.route_policies.iter().map(|route| &route.execution))
    {
        db.execute("INSERT INTO execution_devices(device_id,display_name,connection_json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(device_id) DO NOTHING",params![binding.device_id,binding.device_name,serde_json::to_string(&binding.connection).map_err(|e|e.to_string())?,chrono::Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
    }
    db.execute("INSERT INTO execution_capability_profiles VALUES(?1,?2,?3) ON CONFLICT(capability_profile_id) DO UPDATE SET revision=excluded.revision,profile_json=excluded.profile_json",params![id,profile.revision,value.to_string()]).map(|_|()).map_err(|e|e.to_string())
}
