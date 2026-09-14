//! Bounded supervised RPC operations outside turn execution.
use super::connection::{unavailable, Connection};
use crate::{
    agent_sessions::{
        domain::{AgentInvocationId, AgentSessionId},
        ports::RuntimePortError,
    },
    runtime::processes::*,
};
use serde_json::json;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

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

pub(super) fn with_connection<T>(
    program: &str,
    home: PathBuf,
    cwd: Option<PathBuf>,
    read: impl FnOnce(&Connection) -> Result<T, RuntimePortError>,
) -> Result<T, RuntimePortError> {
    let program = super::super::resolve_program(program.to_owned()).map_err(unavailable)?;
    let sink = Arc::new(ReadSink::default());
    let supervisor = Arc::new(ProcessSupervisor::new(
        Arc::new(DuplexProcessFactory),
        sink.clone(),
    ));
    let id = uuid::Uuid::new_v4().to_string();
    let invocation = AgentInvocationId::new(id.clone()).map_err(|e| unavailable(e.to_string()))?;
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
        read(&connection)
    })();
    let _ = supervisor.close_input(&invocation);
    let cleanup = supervisor
        .shutdown_with_grace_period(Duration::from_secs(2))
        .map_err(|e| unavailable(e.to_string()));
    *sink.0.lock().expect("discovery sink") = None;
    result.and_then(|environment| cleanup.map(|_| environment))
}
