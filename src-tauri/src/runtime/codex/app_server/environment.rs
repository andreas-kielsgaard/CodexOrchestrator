//! Read native capabilities through the same executable, protocol and process owner as execution.
use super::connection::{unavailable, Connection};
use crate::{
    agent_sessions::{
        domain::{AgentInvocationId, AgentSessionId},
        ports::RuntimePortError,
    },
    runtime::processes::*,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

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

#[derive(Default)]
struct ReadSink(Mutex<Option<Arc<Connection>>>);
impl ProcessEventSink for ReadSink {
    fn on_output(&self, _: &AgentInvocationId, output: ProcessOutput) {
        if output.stream != ProcessOutputStream::Stdout {
            return;
        }
        let connection = self.0.lock().ok().and_then(|slot| slot.clone());
        if let Some(connection) = connection {
            match connection.read(&output.bytes) {
                Ok(messages) => {
                    for message in messages {
                        if message.get("id").is_some() {
                            let _ = connection.write(json!({"id":message["id"],"error":{"code":-32601,"message":"Interactive requests are unavailable during capability discovery"}}));
                        }
                    }
                }
                Err(_) => connection.disconnected(),
            }
        }
    }
    fn on_terminal(&self, _: &AgentInvocationId, _: ProcessTerminalOutcome) {
        if let Some(connection) = self.0.lock().ok().and_then(|slot| slot.clone()) {
            connection.disconnected();
        }
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
        let program = super::super::resolve_program(self.program.clone()).map_err(unavailable)?;
        let sink = Arc::new(ReadSink::default());
        let supervisor = Arc::new(ProcessSupervisor::new(
            Arc::new(DuplexProcessFactory),
            sink.clone(),
        ));
        let id = uuid::Uuid::new_v4().to_string();
        let invocation =
            AgentInvocationId::new(id.clone()).map_err(|e| unavailable(e.to_string()))?;
        let connection = Arc::new(Connection::new(invocation.clone(), supervisor.clone()));
        *sink.0.lock().expect("new mutex") = Some(connection.clone());
        supervisor
            .start(
                AgentSessionId::new(id).map_err(|e| unavailable(e.to_string()))?,
                invocation.clone(),
                ProcessLaunchSpec {
                    remove_environment: super::process_context::parent_session_variables(),
                    program,
                    args: vec!["app-server".into()],
                    working_directory: Some(cwd.clone().unwrap_or_else(|| home.clone())),
                    environment: vec![("CODEX_HOME".into(), home.to_string_lossy().into_owned())],
                },
            )
            .map_err(|e| unavailable(e.to_string()))?;
        let result = (|| {
            connection.call("initialize", json!({"clientInfo":{"name":"codex_orchestrator_discovery","version":"1"},"capabilities":{"experimentalApi":true}}))?;
            connection.write(json!({"method":"initialized","params":{}}))?;
            super::capability_roots::apply_skill_roots(&connection, &self.skill_roots)?;
            read(&connection)
        })();
        let _ = supervisor.close_input(&invocation);
        let cleanup = supervisor
            .shutdown_with_grace_period(Duration::from_secs(2))
            .map_err(|e| unavailable(e.to_string()));
        *sink.0.lock().expect("discovery sink") = None;
        result.and_then(|environment| cleanup.map(|_| environment))
    }
}
