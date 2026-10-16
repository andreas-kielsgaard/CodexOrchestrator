use super::repository::IdentityCatalogEntry;
use rusqlite::{params, Connection};
use serde_json::Value;
pub fn export(db: &Connection) -> Result<Vec<(String, String, Value)>, String> {
    db.prepare("SELECT identity_id,display_name,color,shape,created_at,updated_at FROM identity_definitions").map_err(|e|e.to_string())?.query_map([],|r|Ok(serde_json::json!({"id":r.get::<_,String>(0)?,"displayName":r.get::<_,String>(1)?,"color":r.get::<_,String>(2)?,"shape":r.get::<_,String>(3)?,"createdAt":r.get::<_,String>(4)?,"updatedAt":r.get::<_,String>(5)?}))).map_err(|e|e.to_string())?.map(|r|{let value=r.map_err(|e|e.to_string())?;Ok(("identity".into(),value["id"].as_str().ok_or("Missing identity")?.into(),value))}).collect()
}
pub fn validate(id: &str, value: &Value) -> Result<IdentityCatalogEntry, String> {
    let entry: IdentityCatalogEntry =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    entry.definition.validate().map_err(|e| e.to_string())?;
    if entry.definition.id.as_str() != id {
        return Err("Identity mismatch".into());
    }
    Ok(entry)
}
pub fn import(db: &Connection, id: &str, value: &Value) -> Result<(), String> {
    let entry = validate(id, value)?;
    db.execute("INSERT INTO identity_definitions VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(identity_id) DO UPDATE SET display_name=excluded.display_name,color=excluded.color,shape=excluded.shape,updated_at=excluded.updated_at",params![id,entry.definition.display_name,entry.definition.color,value["shape"].as_str().ok_or("Missing shape")?,entry.created_at.to_rfc3339(),entry.updated_at.to_rfc3339()]).map(|_|()).map_err(|e|e.to_string())
}
