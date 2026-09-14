//! Read native capabilities through the same executable, protocol and process owner as execution.
use super::connection::{unavailable, Connection};
use crate::agent_sessions::ports::RuntimePortError;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct CodexEnvironmentReader {
    program: String,
    skill_roots: Vec<String>,
}

pub(crate) struct CodexEnvironment {
    pub(crate) models: Value,
    pub(crate) skills: Value,
    pub(crate) config: Value,
    pub(crate) requirements: Value,
}
pub(crate) trait CodexEnvironmentSource: Send + Sync {
    fn read(
        &self,
        home: PathBuf,
        cwd: Option<PathBuf>,
    ) -> Result<CodexEnvironment, RuntimePortError>;
    fn inventory(
        &self,
        _home: PathBuf,
        _cwd: Option<PathBuf>,
    ) -> Result<crate::execution_configuration::NativeCapabilityInventory, RuntimePortError> {
        Err(unavailable("Native inventory discovery is unavailable"))
    }
}

impl CodexEnvironmentReader {
    pub(crate) fn with_skill_roots(mut self, roots: Vec<String>) -> Self {
        self.skill_roots = roots;
        self
    }
    pub(crate) fn new(program: impl Into<String>) -> Self {
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
        self.with_connection(home, cwd.clone(), |connection| {
            let config = connection.call("config/read", json!({"cwd":cwd,"includeLayers":true}))?;
            let requirements = connection.call("configRequirements/read", json!({}))?;
            let mut models = Vec::new();
            let mut cursor = Value::Null;
            loop {
                let page = connection
                    .call("model/list", json!({"cursor":cursor,"includeHidden":false}))?;
                if let Some(data) = page["data"].as_array() {
                    models.extend(data.iter().cloned());
                }
                cursor = page["nextCursor"].clone();
                if cursor.is_null() {
                    break;
                }
            }
            let skills = connection.call(
                "skills/list",
                json!({"cwds":cwd.into_iter().collect::<Vec<_>>(),"forceReload":true}),
            )?;
            Ok(CodexEnvironment {
                models: Value::Array(models),
                skills,
                config,
                requirements,
            })
        })
    }

    fn inventory(
        &self,
        home: PathBuf,
        cwd: Option<PathBuf>,
    ) -> Result<crate::execution_configuration::NativeCapabilityInventory, RuntimePortError> {
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
        super::client::with_connection(&self.program, home, cwd, |connection| {
            super::capability_roots::apply_skill_roots(connection, &self.skill_roots)?;
            read(connection)
        })
    }
}
