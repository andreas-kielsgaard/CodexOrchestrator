//! Read native capabilities through the same executable, protocol and process owner as execution.
use super::connection::{unavailable, Connection};
use crate::contracts::ports::RuntimePortError;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Clone)]
pub struct CodexEnvironmentReader {
    program: String,
    skill_roots: Vec<String>,
}

pub struct CodexEnvironment {
    pub models: Value,
    pub skills: Value,
    pub config: Value,
    pub requirements: Value,
}
pub trait CodexEnvironmentSource: Send + Sync {
    fn read(
        &self,
        home: PathBuf,
        cwd: Option<PathBuf>,
    ) -> Result<CodexEnvironment, RuntimePortError>;
    fn discover_skills(
        &self,
        home: PathBuf,
        cwd: Option<PathBuf>,
    ) -> Result<super::skills::CodexSkillCatalogue, RuntimePortError> {
        self.read(home, cwd)
            .map(|environment| super::skills::project(&environment.skills))
    }
    fn inventory(
        &self,
        _home: PathBuf,
        _cwd: Option<PathBuf>,
    ) -> Result<crate::configuration::NativeCapabilityInventory, RuntimePortError> {
        Err(unavailable("Native inventory discovery is unavailable"))
    }
}

impl CodexEnvironmentReader {
    pub fn with_skill_roots(mut self, roots: Vec<String>) -> Self {
        self.skill_roots = roots;
        self
    }
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            skill_roots: Vec::new(),
        }
    }
}
impl CodexEnvironmentSource for CodexEnvironmentReader {
    fn read(
        &self,
        home: PathBuf,
        cwd: Option<PathBuf>,
    ) -> Result<CodexEnvironment, RuntimePortError> {
        let context = cwd.clone().unwrap_or_else(|| home.clone());
        self.with_connection(home, cwd.clone(), |connection| {
            let config = connection.call("config/read", json!({"cwd":cwd,"includeLayers":true}))?;
            let requirements = connection.call("configRequirements/read", json!({}))?;
            let mut models = Vec::new();
            let mut cursor = Value::Null;
            loop {
                let page = connection
                    // Capability Profiles need the complete account-visible model catalogue, not
                    // merely the subset Codex happens to surface in its compact picker.
                    .call("model/list", json!({"cursor":cursor,"includeHidden":true}))?;
                if let Some(data) = page["data"].as_array() {
                    models.extend(data.iter().cloned());
                }
                cursor = page["nextCursor"].clone();
                if cursor.is_null() {
                    break;
                }
            }
            let skills = super::skills::read_response(connection, &context, false)?;
            Ok(CodexEnvironment {
                models: Value::Array(models),
                skills,
                config,
                requirements,
            })
        })
    }

    fn discover_skills(
        &self,
        home: PathBuf,
        cwd: Option<PathBuf>,
    ) -> Result<super::skills::CodexSkillCatalogue, RuntimePortError> {
        let context = cwd.clone().unwrap_or_else(|| home.clone());
        self.with_connection(home, cwd, |connection| {
            super::skills::read_forced(connection, &context)
        })
    }

    fn inventory(
        &self,
        home: PathBuf,
        cwd: Option<PathBuf>,
    ) -> Result<crate::configuration::NativeCapabilityInventory, RuntimePortError> {
        let context = cwd.clone().unwrap_or_else(|| home.clone());
        self.with_connection(home, cwd, |connection| {
            Ok(super::inventory::read(
                connection,
                &context,
                &self.skill_roots,
            ))
        })
    }
}

impl CodexEnvironmentReader {
    fn with_connection<T>(
        &self,
        home: PathBuf,
        cwd: Option<PathBuf>,
        read: impl FnOnce(&Connection) -> Result<T, RuntimePortError>,
    ) -> Result<T, RuntimePortError> {
        super::client::with_connection(&self.program, home, cwd, read)
    }
}
