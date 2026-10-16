//! Definition replacement uses the creator's authority, independently of session replication.
use crate::persistence::ActiveDatabase;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS orchid_logical_repositories(id TEXT PRIMARY KEY,label TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS orchid_definitions (
 kind TEXT NOT NULL,id TEXT NOT NULL,master_device_id TEXT NOT NULL,
 revision INTEGER NOT NULL,content_json TEXT NOT NULL,PRIMARY KEY(kind,id)
);
CREATE TABLE IF NOT EXISTS orchid_skill_content (
 id TEXT PRIMARY KEY,content_json TEXT NOT NULL
);
"#;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Definition {
    pub kind: String,
    pub id: String,
    pub master_device_id: String,
    pub revision: u64,
    pub content: Value,
}

#[derive(Clone)]
pub struct DefinitionStore {
    pub(crate) database: Arc<ActiveDatabase>,
}
impl DefinitionStore {
    pub fn new(database: Arc<ActiveDatabase>) -> Self {
        Self { database }
    }
    pub fn list(&self) -> Result<Vec<Definition>, String> {
        self.database
            .read("list synchronized definitions", |db| read_all(db))
            .map_err(|e| e.to_string())
    }
    pub fn get(&self, kind: &str, id: &str) -> Result<Option<Definition>, String> {
        Ok(self
            .list()?
            .into_iter()
            .find(|d| d.kind == kind && d.id == id))
    }
    pub fn proposal(
        &self,
        kind: &str,
        id: &str,
        mut content: Value,
    ) -> Result<Option<Definition>, String> {
        let local = super::repository::ReplicaStore::new(self.database.clone()).device_id()?;
        let Some(mut current) = self.get(kind, id)? else {
            return Ok(None);
        };
        if current.master_device_id == local {
            return Ok(None);
        }
        portable_routes(&mut content, &local, false);
        current.content = content;
        validate(&current)?;
        Ok(Some(current))
    }
    /// Capture only master-owned current data. Non-master offline edits are never replayed.
    pub fn capture_owned(&self) -> Result<Vec<Definition>, String> {
        let local = super::repository::ReplicaStore::new(self.database.clone()).device_id()?;
        self.database
            .write(
                "capture master definitions",
                |tx| -> Result<Vec<Definition>, String> {
                    let mut content =
                        crate::workflows::authoring_repository::export_definitions(tx)?;
                    content.extend(crate::execution_configuration::definitions::export(tx)?);
                    content.extend(crate::identities::definitions::export(tx)?);
                    content.extend(crate::repository_catalog::definitions::export(tx)?);
                    let skills = tx
                        .prepare("SELECT id,content_json FROM orchid_skill_content")
                        .map_err(|e| e.to_string())?
                        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                        .map_err(|e| e.to_string())?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| e.to_string())?;
                    for (id, json) in skills {
                        content.push((
                            "skill".into(),
                            id,
                            serde_json::from_str(&json).map_err(|e| e.to_string())?,
                        ));
                    }
                    let existing = read_all(tx)?;
                    let present: std::collections::BTreeSet<_> = content
                        .iter()
                        .map(|(kind, id, _)| (kind.clone(), id.clone()))
                        .collect();
                    let mut result = Vec::new();
                    for (kind, id, mut content) in content {
                        let previous = existing.iter().find(|d| d.kind == kind && d.id == id);
                        if previous.is_some_and(|d| d.master_device_id != local) {
                            continue;
                        }
                        portable_routes(&mut content, &local, false);
                        let revision =
                            previous.map_or(1, |d| d.revision + u64::from(d.content != content));
                        let definition = Definition {
                            kind,
                            id,
                            content,
                            revision,
                            master_device_id: local.clone(),
                        };
                        save(tx, &definition)?;
                        result.push(definition);
                    }
                    for previous in existing.into_iter().filter(|d| {
                        d.master_device_id == local
                            && !present.contains(&(d.kind.clone(), d.id.clone()))
                    }) {
                        let deleted = Definition {
                            revision: previous.revision + u64::from(!previous.content.is_null()),
                            content: Value::Null,
                            ..previous
                        };
                        save(tx, &deleted)?;
                        result.push(deleted);
                    }
                    Ok(result)
                },
            )
            .map_err(|e| e.to_string())
    }
    /// Server storage has no feature side effects. Only its recorded master can publish.
    pub fn accept_master(&self, source: &str, definition: &Definition) -> Result<bool, String> {
        validate(definition)?;
        if source != definition.master_device_id {
            return Err("Definition publication requires its master".into());
        }
        self.database
            .write("accept master definition", |tx| -> Result<bool, String> {
                if let Some(existing) = read_all(tx)?
                    .into_iter()
                    .find(|d| d.kind == definition.kind && d.id == definition.id)
                {
                    if existing.master_device_id != source {
                        return Err("Definition belongs to another master".into());
                    }
                    if definition.revision < existing.revision {
                        return Ok(false);
                    }
                    if definition.revision == existing.revision {
                        if existing.content != definition.content {
                            return Err("Master revision has conflicting content".into());
                        }
                        return Ok(false);
                    }
                }
                save(tx, definition)?;
                Ok(true)
            })
            .map_err(|e| e.to_string())
    }
    /// Every received canonical copy replaces local non-master edits, including an equal revision.
    pub fn receive(&self, definition: &Definition) -> Result<(), String> {
        validate(definition)?;
        let local = super::repository::ReplicaStore::new(self.database.clone()).device_id()?;
        if definition.master_device_id == local {
            return Ok(());
        }
        self.database
            .write(
                "replace non-master definition",
                |tx| -> Result<(), String> {
                    if let Some(existing) = read_all(tx)?
                        .into_iter()
                        .find(|d| d.kind == definition.kind && d.id == definition.id)
                    {
                        if existing.master_device_id != definition.master_device_id {
                            return Err("Definition master changed".into());
                        }
                        if existing.revision > definition.revision {
                            return Ok(());
                        }
                    }
                    let mut local_copy = definition.clone();
                    portable_routes(&mut local_copy.content, &local, true);
                    apply(tx, &local_copy)?;
                    save(tx, definition)
                },
            )
            .map_err(|e| e.to_string())
    }
    /// Called by the online router on the master; a stale edit is rejected, never queued.
    pub fn edit_on_master(&self, proposal: &Definition) -> Result<Definition, String> {
        validate(proposal)?;
        let local = super::repository::ReplicaStore::new(self.database.clone()).device_id()?;
        self.capture_owned()?;
        self.database
            .write(
                "accept online definition edit",
                |tx| -> Result<Definition, String> {
                    let existing = read_all(tx)?
                        .into_iter()
                        .find(|d| d.kind == proposal.kind && d.id == proposal.id)
                        .ok_or("Definition not found on master")?;
                    if existing.master_device_id != local || proposal.master_device_id != local {
                        return Err("This device is not the definition master".into());
                    }
                    if existing.revision != proposal.revision {
                        return Err(
                            "Definition changed on master; fetch its synchronized copy".into()
                        );
                    }
                    let accepted = Definition {
                        revision: existing.revision + 1,
                        ..proposal.clone()
                    };
                    let mut local_copy = accepted.clone();
                    portable_routes(&mut local_copy.content, &local, true);
                    apply(tx, &local_copy)?;
                    save(tx, &accepted)?;
                    Ok(accepted)
                },
            )
            .map_err(|e| e.to_string())
    }
    pub fn save_skill(&self, skill: &crate::skills::SkillPackage) -> Result<(), String> {
        skill.validate()?;
        self.database.write("save Orchid skill copy",|tx|tx.execute("INSERT INTO orchid_skill_content VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET content_json=excluded.content_json",params![skill.id,serde_json::to_string(skill).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn skills(&self) -> Result<Vec<crate::skills::SkillPackage>, String> {
        self.database
            .read(
                "read Orchid skill copies",
                |db| -> Result<Vec<crate::skills::SkillPackage>, String> {
                    let rows = db
                        .prepare("SELECT content_json FROM orchid_skill_content ORDER BY id")
                        .map_err(|e| e.to_string())?
                        .query_map([], |r| r.get::<_, String>(0))
                        .map_err(|e| e.to_string())?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| e.to_string())?;
                    rows.into_iter()
                        .map(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
                        .collect()
                },
            )
            .map_err(|e| e.to_string())
    }
}
fn read_all(db: &Connection) -> Result<Vec<Definition>, String> {
    db.prepare("SELECT kind,id,master_device_id,revision,content_json FROM orchid_definitions ORDER BY kind,id").map_err(|e|e.to_string())?.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get::<_,String>(4)?))).map_err(|e|e.to_string())?.map(|row|{let (kind,id,master_device_id,revision,json)=row.map_err(|e|e.to_string())?;Ok(Definition{kind,id,master_device_id,revision,content:serde_json::from_str(&json).map_err(|e|e.to_string())?})}).collect()
}
fn save(db: &Connection, d: &Definition) -> Result<(), String> {
    db.execute("INSERT INTO orchid_definitions VALUES(?1,?2,?3,?4,?5) ON CONFLICT(kind,id) DO UPDATE SET revision=excluded.revision,content_json=excluded.content_json",params![d.kind,d.id,d.master_device_id,d.revision,d.content.to_string()]).map(|_|()).map_err(|e|e.to_string())
}
fn validate(d: &Definition) -> Result<(), String> {
    if d.id.is_empty() || d.master_device_id.is_empty() || d.revision == 0 {
        return Err("Definition identity is incomplete".into());
    }
    if d.content.is_null()
        && ["workflow", "capability_profile", "identity", "skill"].contains(&d.kind.as_str())
    {
        return Ok(());
    }
    match d.kind.as_str() {
        "repository" => crate::repository_catalog::definitions::validate(&d.id, &d.content),
        "workflow" => {
            crate::workflows::authoring_repository::validate_definition(&d.id, &d.content)
        }
        "capability_profile" => {
            crate::execution_configuration::definitions::validate(&d.id, &d.content).map(|_| ())
        }
        "identity" => crate::identities::definitions::validate(&d.id, &d.content).map(|_| ()),
        "skill" => {
            let skill: crate::skills::SkillPackage =
                serde_json::from_value(d.content.clone()).map_err(|e| e.to_string())?;
            if skill.id != d.id {
                return Err("Skill identity mismatch".into());
            }
            skill.validate()
        }
        _ => Err("Unsupported definition kind".into()),
    }
}
fn apply(db: &Connection, d: &Definition) -> Result<(), String> {
    if d.content.is_null() {
        let sql = match d.kind.as_str() {
            "workflow" => "DELETE FROM workflow_recipe_authoring WHERE recipe_id=?1",
            "capability_profile" => {
                "DELETE FROM execution_capability_profiles WHERE capability_profile_id=?1"
            }
            "identity" => "DELETE FROM identity_definitions WHERE identity_id=?1",
            "skill" => "DELETE FROM orchid_skill_content WHERE id=?1",
            _ => return Err("Unsupported definition kind".into()),
        };
        return db
            .execute(sql, [&d.id])
            .map(|_| ())
            .map_err(|e| e.to_string());
    }
    match d.kind.as_str(){
    "repository"=>crate::repository_catalog::definitions::import(db,&d.id,&d.content),
    "workflow"=>crate::workflows::authoring_repository::import_definition(db,&d.content),
    "capability_profile"=>crate::execution_configuration::definitions::import(db,&d.id,&d.content),
    "identity"=>crate::identities::definitions::import(db,&d.id,&d.content),
    "skill"=>db.execute("INSERT INTO orchid_skill_content VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET content_json=excluded.content_json",params![d.id,d.content.to_string()]).map(|_|()).map_err(|e|e.to_string()),
    _=>Err("Unsupported definition kind".into()),
}
}

pub(crate) fn portable_routes(value: &mut Value, device: &str, receiving_master: bool) {
    match value {
        Value::Object(object) => {
            if object.contains_key("connection") && object.contains_key("configurationRef") {
                let source = if receiving_master { device } else { "local" };
                if object.get("deviceId").and_then(Value::as_str) == Some(source) {
                    object.insert(
                        "deviceId".into(),
                        serde_json::json!(if receiving_master { "local" } else { device }),
                    );
                    object.insert(
                        "connection".into(),
                        serde_json::json!({"kind":if receiving_master{"local"}else{"device"}}),
                    );
                }
            }
            for child in object.values_mut() {
                portable_routes(child, device, receiving_master);
            }
        }
        Value::Array(array) => {
            for child in array {
                portable_routes(child, device, receiving_master);
            }
        }
        _ => {}
    }
}
