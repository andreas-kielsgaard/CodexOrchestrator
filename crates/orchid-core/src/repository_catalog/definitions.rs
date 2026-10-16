//! Shared repository identity and label; physical registrations remain device-local.
use rusqlite::{params, Connection};
use serde_json::Value;
pub fn export(db: &Connection) -> Result<Vec<(String, String, Value)>, String> {
    db.prepare("SELECT repository_id,label FROM registered_repositories UNION SELECT id,label FROM orchid_logical_repositories WHERE id NOT IN(SELECT repository_id FROM registered_repositories)").map_err(|e|e.to_string())?
        .query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?
        .map(|row|{let (id,label)=row.map_err(|e|e.to_string())?;Ok(("repository".into(),id.clone(),serde_json::json!({"id":id,"label":label})))}).collect()
}
pub fn validate(id: &str, value: &Value) -> Result<(), String> {
    if value["id"].as_str() != Some(id)
        || value["label"].as_str().is_none_or(|s| s.trim().is_empty())
    {
        return Err("Invalid logical repository definition".into());
    }
    Ok(())
}
pub fn import(db: &Connection, id: &str, value: &Value) -> Result<(), String> {
    validate(id, value)?;
    let label = value["label"].as_str().ok_or("Missing repository label")?;
    db.execute("INSERT INTO orchid_logical_repositories VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET label=excluded.label",params![id,label]).map_err(|e|e.to_string())?;
    db.execute(
        "UPDATE registered_repositories SET label=?2 WHERE repository_id=?1",
        params![id, label],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
