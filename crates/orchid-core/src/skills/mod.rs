//! Portable Orchid-owned skill packages; origin folders remain externally owned.
mod files;
#[cfg(test)]
mod tests;
use crate::synchronization::definitions::DefinitionStore;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillPackage {
    pub id: String,
    pub name: String,
    pub description: String,
    pub group_id: String,
    pub origin_device: String,
    pub origin_path: String,
    pub origin_fingerprint: String,
    pub files: BTreeMap<String, SkillFile>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillFile {
    pub data_base64: String,
    pub executable: bool,
}
impl SkillPackage {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.len() != 64
            || !self.id.bytes().all(|b| b.is_ascii_hexdigit())
            || self.name.trim().is_empty()
        {
            return Err("Invalid skill identity".into());
        }
        if !self.files.contains_key("SKILL.md") {
            return Err("Skill package is missing SKILL.md".into());
        }
        let mut names = std::collections::HashSet::new();
        let mut bytes = 0;
        for (name, file) in &self.files {
            files::validate_relative(name)?;
            if !names.insert(name.to_lowercase()) {
                return Err("Skill package contains conflicting filenames".into());
            }
            bytes += base64::engine::general_purpose::STANDARD
                .decode(&file.data_base64)
                .map_err(|e| e.to_string())?
                .len();
            if bytes > 32 * 1024 * 1024 {
                return Err("Skill package exceeds 32 MiB".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct SkillCatalogue {
    pub store: DefinitionStore,
    pub root: PathBuf,
    pub device: String,
}
impl SkillCatalogue {
    pub fn refresh_origins(store: &DefinitionStore) -> Result<(), String> {
        let local = crate::synchronization::repository::ReplicaStore::new(store.database.clone())
            .device_id()?;
        for mut skill in store
            .skills()?
            .into_iter()
            .filter(|s| s.origin_device == local)
        {
            let path = Path::new(&skill.origin_path);
            if !path.is_file() {
                continue;
            }
            let Some(parent) = path.parent() else {
                continue;
            };
            let Ok(content) = files::read_package(parent) else {
                continue;
            };
            let fingerprint = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&content).map_err(|e| e.to_string())?)
            );
            if fingerprint != skill.origin_fingerprint {
                skill.files = content;
                skill.origin_fingerprint = fingerprint;
                store.save_skill(&skill)?;
            }
        }
        Ok(())
    }
    pub fn new(
        database: std::sync::Arc<crate::persistence::ActiveDatabase>,
        root: PathBuf,
    ) -> Result<Self, String> {
        let device =
            crate::synchronization::repository::ReplicaStore::new(database.clone()).device_id()?;
        Ok(Self {
            store: DefinitionStore::new(database),
            root,
            device,
        })
    }
    pub fn import(
        &self,
        path: &Path,
        name: &str,
        description: &str,
        group: &str,
    ) -> Result<SkillPackage, String> {
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        if path.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
            return Err("Skill origin must name SKILL.md".into());
        }
        // An already owned materialization keeps its portable identity.
        for skill in self.store.skills()? {
            if path
                == self
                    .root
                    .join(&skill.id)
                    .join("SKILL.md")
                    .canonicalize()
                    .unwrap_or_default()
            {
                return Ok(skill);
            }
        }
        let origin_path = path.to_string_lossy().into_owned();
        let id = format!(
            "{:x}",
            Sha256::digest(format!("{}\0{origin_path}", self.device))
        );
        let content = files::read_package(path.parent().ok_or("Skill has no parent")?)?;
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&content).map_err(|e| e.to_string())?)
        );
        if let Some(existing) = self
            .store
            .skills()?
            .into_iter()
            .find(|s| s.id == id && s.origin_fingerprint == fingerprint)
        {
            return Ok(existing);
        }
        let skill = SkillPackage {
            id,
            name: name.into(),
            description: description.into(),
            group_id: group.into(),
            origin_device: self.device.clone(),
            origin_path,
            origin_fingerprint: fingerprint,
            files: content,
        };
        self.store.save_skill(&skill)?;
        Ok(skill)
    }
    pub fn materialize(&self, skill: &SkillPackage) -> Result<PathBuf, String> {
        skill.validate()?;
        files::materialize(&self.root, skill)
    }
    pub fn resolve(
        &self,
        input: &crate::agent_sessions::ports::RuntimeSkillInput,
    ) -> Result<crate::agent_sessions::ports::RuntimeSkillInput, String> {
        Self::refresh_origins(&self.store)?;
        let skill = if let Some(id) = input.id.strip_prefix("orchid-skill:") {
            self.store
                .skills()?
                .into_iter()
                .find(|s| s.id == id)
                .ok_or_else(|| {
                    format!("Skill {} has not synchronized to this device", input.name)
                })?
        } else if let Some(skill) = self
            .store
            .skills()?
            .into_iter()
            .find(|s| s.origin_path == input.path && s.name == input.name)
        {
            skill
        } else {
            self.import(
                Path::new(&input.path),
                &input.name,
                &input.description,
                crate::execution_configuration::NATIVE_SKILL_GROUP,
            )?
        };
        let path = self.materialize(&skill)?;
        Ok(crate::agent_sessions::ports::RuntimeSkillInput {
            id: format!("orchid-skill:{}", skill.id),
            name: skill.name,
            description: skill.description,
            path: path.to_string_lossy().into_owned(),
            content_sha256: format!(
                "{:x}",
                Sha256::digest(std::fs::read(&path).map_err(|e| e.to_string())?)
            ),
        })
    }
}
