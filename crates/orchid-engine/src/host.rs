//! Destination-side execution ownership. Product history stays with the desktop application.
pub mod providers;
pub use providers::{HostProvider, HostProviderConfiguration};
use crate::{
    contracts::*,
    protocol::*,
    repository_context::{RepositoryContext, WorktreeLocation},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostConfiguration {
    pub device_id: String,
    pub device_name: String,
    pub configurations: Vec<HostProviderConfiguration>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionBinding {
    session_id: AgentSessionId,
    configuration_ref: String,
    working_directory: String,
    external_context_id: Option<ExternalRuntimeContextId>,
}

pub struct Host {
    configuration: HostConfiguration,
    providers: HashMap<String, Arc<dyn HostProvider>>,
    sessions_directory: PathBuf,
    runtimes: HashMap<String, Arc<dyn AgentRuntime>>,
    invocations: Mutex<HashMap<AgentInvocationId, String>>,
    active_sessions: Arc<Mutex<HashMap<AgentSessionId, AgentInvocationId>>>,
}

fn unavailable(error: impl std::fmt::Display) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::Unavailable, error.to_string())
}

pub fn list_worktrees(
    repository_root: &str,
    branch_ref: Option<&str>,
) -> Result<Vec<WorktreeInstance>, RuntimePortError> {
    let repository = RepositoryContext::discover().map_err(unavailable)?;
    let root = Path::new(repository_root);
    let identity = repository.identities().inspect(root).map_err(unavailable)?;
    let observations = repository
        .worktrees()
        .list(&identity.id, root)
        .map_err(unavailable)?;
    Ok(observations
        .into_iter()
        .filter(|item| {
            branch_ref.is_none()
                || item.head_ref.as_ref().map(|branch| branch.as_str()) == branch_ref
        })
        .filter_map(|item| {
            let WorktreeLocation::Available(location) = item.location else {
                return None;
            };
            let status = repository.status().status(location.path()).ok()?;
            let committed_at = repository
                .commits()
                .facts(location.path(), &item.head)
                .ok()?
                .committed_at;
            Some(WorktreeInstance {
                handle: item.id.as_str().into(),
                path: location.path().to_string_lossy().into_owned(),
                branch_ref: item.head_ref.map(|branch| branch.as_str().into()),
                head: Some(item.head.as_str().into()),
                dirty: status.staged_paths > 0
                    || status.unstaged_paths > 0
                    || status.untracked_paths > 0,
                head_committed_at: Some(committed_at),
            })
        })
        .collect())
}

impl Host {
    pub fn new(
        configuration: HostConfiguration,
        sessions_directory: PathBuf,
    ) -> Result<Self, RuntimePortError> {
        fs::create_dir_all(&sessions_directory).map_err(unavailable)?;
        let providers = providers::registered();
        let runtimes = configuration
            .configurations
            .iter()
            .map(|config| {
                let provider = providers.get(&config.provider).ok_or_else(|| {
                    unavailable(format!(
                        "Agent provider `{}` is not registered on this Orchid host",
                        config.provider
                    ))
                })?;
                Ok((config.id.clone(), provider.runtime(config)))
            })
            .collect::<Result<HashMap<_, _>, RuntimePortError>>()?;
        Ok(Self {
            configuration,
            providers,
            sessions_directory,
            runtimes,
            invocations: Mutex::new(HashMap::new()),
            active_sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn configuration(&self, id: &str) -> Result<&HostProviderConfiguration, RuntimePortError> {
        self.configuration
            .configurations
            .iter()
            .find(|config| config.id == id)
            .ok_or_else(|| unavailable(format!("Unknown host execution configuration: {id}")))
    }

    fn provider_configuration(
        &self,
        provider: &str,
        id: &str,
    ) -> Result<(&HostProviderConfiguration, &dyn HostProvider), RuntimePortError> {
        let configuration = self.configuration(id)?;
        if configuration.provider != provider {
            return Err(unavailable(format!(
                "Execution configuration `{id}` does not belong to agent provider `{provider}`"
            )));
        }
        Ok((configuration, self.host_provider(configuration)?))
    }

    fn host_provider(
        &self,
        configuration: &HostProviderConfiguration,
    ) -> Result<&dyn HostProvider, RuntimePortError> {
        self.providers
            .get(&configuration.provider)
            .map(|provider| provider.as_ref())
            .ok_or_else(|| {
                unavailable(format!(
                    "Agent provider `{}` is not registered on this Orchid host",
                    configuration.provider
                ))
            })
    }

    fn runtime(&self, id: &str) -> Result<Arc<dyn AgentRuntime>, RuntimePortError> {
        self.runtimes
            .get(id)
            .cloned()
            .ok_or_else(|| unavailable(format!("Unknown host execution configuration: {id}")))
    }

    fn invocation_runtime(
        &self,
        id: &AgentInvocationId,
    ) -> Result<Arc<dyn AgentRuntime>, RuntimePortError> {
        let config = self
            .invocations
            .lock()
            .map_err(unavailable)?
            .get(id)
            .cloned()
            .ok_or_else(|| {
                RuntimePortError::new(
                    RuntimePortErrorKind::NotActive,
                    "Invocation has no connection on this host",
                )
            })?;
        self.runtime(&config)
    }

    pub fn execute(
        &self,
        command: HostCommand,
        output: Arc<dyn FrameOutput>,
    ) -> Result<Value, RuntimePortError> {
        match command {
            HostCommand::PublishedCommit {
                repository_root,
                branch_ref,
            } => Ok(
                serde_json::json!({"commit":crate::workspaces::published_commit(&repository_root, &branch_ref)?}),
            ),
            HostCommand::MaterializeWorktree {
                repository_root,
                branch_ref,
                commit,
                instance_id,
            } => serde_json::to_value(crate::workspaces::materialize_worktree(
                &repository_root,
                &branch_ref,
                &commit,
                &instance_id,
            )?)
            .map_err(unavailable),
            HostCommand::InspectWorktree {
                worktree_root,
                compare_to_head,
            } => serde_json::to_value(crate::workspaces::inspect_worktree(
                &worktree_root,
                compare_to_head.as_deref(),
            )?)
            .map_err(unavailable),
            HostCommand::CaptureWorktreeSnapshot {
                worktree_root,
                destination_head,
                snapshot_id,
            } => serde_json::to_value(crate::workspaces::capture_worktree_snapshot(
                &worktree_root,
                destination_head.as_deref(),
                &snapshot_id,
            )?)
            .map_err(unavailable),
            HostCommand::ApplyWorktreeSnapshot {
                worktree_root,
                snapshot,
            } => serde_json::to_value(crate::workspaces::apply_worktree_snapshot(
                &worktree_root,
                &snapshot,
            )?)
            .map_err(unavailable),
            HostCommand::AuxiliaryWorkspace { session_id } => serde_json::to_value(
                crate::workspaces::auxiliary_workspace(&self.sessions_directory, &session_id)?,
            )
            .map_err(unavailable),
            HostCommand::ExportContinuation {
                provider,
                configuration_ref,
                external_context_id,
            } => {
                let (config, host_provider) =
                    self.provider_configuration(&provider, &configuration_ref)?;
                self.assert_native_idle(&configuration_ref, &external_context_id)?;
                serde_json::to_value(host_provider.export_continuation(config, &external_context_id)?)
                    .map_err(unavailable)
            }
            HostCommand::InstallContinuation {
                provider,
                configuration_ref,
                continuation,
            } => {
                let (config, host_provider) =
                    self.provider_configuration(&provider, &configuration_ref)?;
                let id = host_provider.continuation_context(&continuation)?;
                self.assert_native_idle(&configuration_ref, &id)?;
                host_provider.install_continuation(config, &continuation)?;
                Ok(Value::Null)
            }
            HostCommand::PrepareInvocation {
                provider,
                configuration_ref,
                request,
                external_context_id,
            } => {
                let _ = self.provider_configuration(&provider, &configuration_ref)?;
                self.launch_invocation(configuration_ref, request, external_context_id, output, true)
            }
            HostCommand::DeliverPreparedInvocation { invocation_id } => {
                self.invocation_runtime(&invocation_id)?
                    .deliver_prepared_invocation(&invocation_id)?;
                Ok(Value::Null)
            }
            HostCommand::Describe => serde_json::to_value(HostDescription {
                contract_version: HOST_PROTOCOL_VERSION,
                device_id: self.configuration.device_id.clone(),
                device_name: self.configuration.device_name.clone(),
                configurations: self
                    .configuration
                    .configurations
                    .iter()
                    .map(|config| HostConfigurationDescription {
                        id: config.id.clone(),
                        provider_kind: config.provider.clone(),
                    })
                    .collect(),
            })
            .map_err(unavailable),
            HostCommand::ListWorktrees {
                repository_root,
                branch_ref,
            } => serde_json::to_value(list_worktrees(&repository_root, branch_ref.as_deref())?)
                .map_err(unavailable),
            HostCommand::Capabilities {
                provider,
                configuration_ref,
                working_directory,
            } => {
                let (config, host_provider) =
                    self.provider_configuration(&provider, &configuration_ref)?;
                let capabilities = host_provider.capabilities(
                    config,
                    ProviderConfigurationRef::new(provider, configuration_ref.clone()),
                    working_directory.map(PathBuf::from),
                )?;
                serde_json::to_value(capabilities).map_err(unavailable)
            }
            HostCommand::Preflight {
                provider,
                configuration_ref,
                mode,
                options,
            } => {
                let _ = self.provider_configuration(&provider, &configuration_ref)?;
                serde_json::to_value(self.runtime(&configuration_ref)?
                    .preflight_invocation(mode, &options)?,
                ).map_err(unavailable)
            },
            HostCommand::Invoke {
                provider,
                configuration_ref,
                request,
                external_context_id,
            } => {
                let _ = self.provider_configuration(&provider, &configuration_ref)?;
                self.launch_invocation(configuration_ref, request, external_context_id, output, false)
            }
            HostCommand::Respond {
                invocation_id,
                request_id,
                response,
            } => {
                self.invocation_runtime(&invocation_id)?.respond(
                    &invocation_id,
                    &request_id,
                    response,
                )?;
                Ok(Value::Null)
            }
            HostCommand::Cancel { invocation_id } => {
                self.invocation_runtime(&invocation_id)?
                    .cancel_invocation(&invocation_id)?;
                Ok(Value::Null)
            }
            HostCommand::ActiveTurn { invocation_id } => serde_json::to_value(
                self.invocation_runtime(&invocation_id)?
                    .active_turn(&invocation_id)?,
            )
            .map_err(unavailable),
            HostCommand::Steer {
                invocation_id,
                target,
                input_id,
                text,
            } => {
                self.invocation_runtime(&invocation_id)?.steer(
                    &invocation_id,
                    &target,
                    &input_id,
                    &text,
                )?;
                Ok(Value::Null)
            }
        }
    }

    fn launch_invocation(
        &self,
        configuration_ref: String,
        mut request: RuntimeInvocationRequest,
        external_context_id: Option<ExternalRuntimeContextId>,
        output: Arc<dyn FrameOutput>,
        prepare: bool,
    ) -> Result<Value, RuntimePortError> {
        let config = self.configuration(&configuration_ref)?;
        let cwd = request
            .working_directory
            .as_deref()
            .ok_or_else(|| unavailable("Remote execution requires an existing worktree"))?;
        if !Path::new(cwd).is_absolute() || !Path::new(cwd).is_dir() {
            return Err(unavailable("Remote worktree directory is unavailable"));
        }
        let path = self.binding_path(&request.session_id);
        let binding = SessionBinding {
            session_id: request.session_id.clone(),
            configuration_ref: configuration_ref.clone(),
            working_directory: cwd.into(),
            external_context_id: external_context_id.clone(),
        };
        if path.exists() && !prepare {
            let previous: SessionBinding =
                serde_json::from_slice(&fs::read(&path).map_err(unavailable)?)
                    .map_err(unavailable)?;
            if previous.configuration_ref != binding.configuration_ref
                || previous.working_directory != binding.working_directory
                || previous
                    .external_context_id
                    .as_ref()
                    .is_some_and(|id| Some(id) != external_context_id.as_ref())
            {
                return Err(unavailable("Session is already bound to another host configuration, directory, or provider thread; prepare an idle rebind first"));
            }
        }
        let extension = request
            .launch_extension
            .get_or_insert_with(Default::default);
        if !extension.managed_mcp_servers.is_empty() {
            return Err(unavailable(
                "Remote workflow tools are outside this prototype",
            ));
        }
        for (key, value) in self.host_provider(config)?.launch_environment(config) {
            extension.environment.retain(|(existing, _)| existing != &key);
            extension.environment.push((key, value));
        }
        let runtime = self.runtime(&configuration_ref)?;
        let id = request.invocation_id.clone();
        let session = request.session_id.clone();
        {
            let mut active = self.active_sessions.lock().map_err(unavailable)?;
            if active.contains_key(&session) {
                return Err(unavailable(
                    "Session already has an active or preparing invocation",
                ));
            }
            active.insert(session.clone(), id.clone());
        }
        self.invocations
            .lock()
            .map_err(unavailable)?
            .insert(id.clone(), configuration_ref);
        let sink = Arc::new(InvocationOutput {
            output,
            binding: Mutex::new(binding.clone()),
            path: path.clone(),
            active_sessions: self.active_sessions.clone(),
            persist_binding: !prepare,
        });
        let result = if prepare {
            runtime
                .prepare_invocation(request, external_context_id, sink)
                .and_then(|ready| {
                    let ready_binding = SessionBinding {
                        external_context_id: Some(ready.external_context_id.clone()),
                        working_directory: ready.working_directory.clone(),
                        ..binding
                    };
                    fs::write(
                        &path,
                        serde_json::to_vec(&ready_binding).map_err(unavailable)?,
                    )
                    .map_err(unavailable)?;
                    serde_json::to_value(ready).map_err(unavailable)
                })
        } else {
            fs::write(&path, serde_json::to_vec(&binding).map_err(unavailable)?)
                .map_err(unavailable)?;
            let started = if let Some(external) = external_context_id {
                runtime.resume_invocation(request, external, sink)
            } else {
                runtime.start_invocation(request, sink)
            };
            started.map(|_| Value::Null)
        };
        if result.is_err() {
            self.active_sessions
                .lock()
                .map_err(unavailable)?
                .remove(&session);
            let _ = runtime.cancel_invocation(&id);
        }
        result
    }

    fn assert_native_idle(
        &self,
        configuration_ref: &str,
        external: &ExternalRuntimeContextId,
    ) -> Result<(), RuntimePortError> {
        let active = self.active_sessions.lock().map_err(unavailable)?;
        for session in active.keys() {
            let path = self.binding_path(session);
            if let Ok(bytes) = fs::read(path) {
                let binding: SessionBinding =
                    serde_json::from_slice(&bytes).map_err(unavailable)?;
                if binding.configuration_ref == configuration_ref
                    && binding.external_context_id.as_ref() == Some(external)
                {
                    return Err(unavailable(
                        "Native conversation has an active or preparing invocation",
                    ));
                }
            }
        }
        Ok(())
    }

    fn binding_path(&self, id: &AgentSessionId) -> PathBuf {
        use sha2::{Digest, Sha256};
        self.sessions_directory
            .join(format!("{:x}.json", Sha256::digest(id.as_str().as_bytes())))
    }

    pub fn shutdown(&self) -> Result<(), RuntimePortError> {
        for runtime in self.runtimes.values() {
            runtime.shutdown()?;
        }
        Ok(())
    }
}

pub trait FrameOutput: Send + Sync {
    fn send(&self, frame: HostFrame) -> Result<(), RuntimePortError>;
}

struct InvocationOutput {
    output: Arc<dyn FrameOutput>,
    binding: Mutex<SessionBinding>,
    path: PathBuf,
    active_sessions: Arc<Mutex<HashMap<AgentSessionId, AgentInvocationId>>>,
    persist_binding: bool,
}
impl AgentRuntimeUpdateSink for InvocationOutput {
    fn emit_update(
        &self,
        invocation_id: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        if matches!(&update, RuntimeUpdate::Finished(_)) {
            let session = self.binding.lock().map_err(unavailable)?.session_id.clone();
            let mut active = self.active_sessions.lock().map_err(unavailable)?;
            if active.get(&session) == Some(invocation_id) {
                active.remove(&session);
            }
        }
        if let RuntimeUpdate::Event(event) = &update {
            if let Some(external) = event
                .normalized
                .as_ref()
                .and_then(|event| event.external_context_id.as_ref())
            {
                let mut binding = self.binding.lock().map_err(unavailable)?;
                binding.external_context_id = Some(external.clone());
                if self.persist_binding {
                    fs::write(
                        &self.path,
                        serde_json::to_vec(&*binding).map_err(unavailable)?,
                    )
                    .map_err(unavailable)?;
                }
            }
        }
        self.output.send(HostFrame::Update {
            invocation_id: invocation_id.clone(),
            update,
        })
    }
    fn report_delivery_failure(
        &self,
        invocation_id: &AgentInvocationId,
        failure: RuntimeUpdateDeliveryFailure,
    ) {
        eprintln!(
            "Unable to deliver host update for {invocation_id}: {}",
            failure.error
        );
    }
}

pub struct JsonLineOutput<W: Write + Send>(pub Mutex<W>);
impl<W: Write + Send> FrameOutput for JsonLineOutput<W> {
    fn send(&self, frame: HostFrame) -> Result<(), RuntimePortError> {
        let mut output = self.0.lock().map_err(unavailable)?;
        serde_json::to_writer(&mut *output, &frame).map_err(unavailable)?;
        output.write_all(b"\n").map_err(unavailable)?;
        output.flush().map_err(unavailable)
    }
}

/// Read requests independently of provider event emission; interactive replies remain usable.
pub fn connect(
    host: Arc<Host>,
    input: impl BufRead,
    output: Arc<dyn FrameOutput>,
) -> Result<(), RuntimePortError> {
    let mut preparations = Vec::new();
    for line in input.lines() {
        let line = line.map_err(unavailable)?;
        if line.trim().is_empty() {
            continue;
        }
        let request: HostRequest = serde_json::from_str(&line).map_err(unavailable)?;
        if matches!(&request.command, HostCommand::PrepareInvocation { .. }) {
            let host = host.clone();
            let output = output.clone();
            preparations.push(std::thread::spawn(move || {
                let result = host.execute(request.command, output.clone());
                let (result, error) = match result {
                    Ok(value) => (Some(value), None),
                    Err(error) => (None, Some(error)),
                };
                output.send(HostFrame::Response {
                    id: request.id,
                    result,
                    error,
                })
            }));
            continue;
        }
        let result = host.execute(request.command, output.clone());
        let (result, error) = match result {
            Ok(value) => (Some(value), None),
            Err(error) => (None, Some(error)),
        };
        output.send(HostFrame::Response {
            id: request.id,
            result,
            error,
        })?;
    }
    let shutdown = host.shutdown();
    for preparation in preparations {
        preparation
            .join()
            .map_err(|_| unavailable("Native preparation worker panicked"))??;
    }
    shutdown
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[derive(Default)]
    struct Capture(Mutex<Vec<HostFrame>>);
    impl FrameOutput for Capture {
        fn send(&self, frame: HostFrame) -> Result<(), RuntimePortError> {
            self.0.lock().unwrap().push(frame);
            Ok(())
        }
    }

    #[derive(Default)]
    struct InteractiveRuntime {
        active: Mutex<Option<(RuntimeInvocationRequest, Arc<dyn AgentRuntimeUpdateSink>)>>,
        launches: Mutex<Vec<RuntimeInvocationRequest>>,
    }
    impl AgentRuntime for InteractiveRuntime {
        fn prepare_invocation(
            &self,
            request: RuntimeInvocationRequest,
            external: Option<ExternalRuntimeContextId>,
            sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<RuntimeInvocationReady, RuntimePortError> {
            let cwd = request.working_directory.clone().unwrap();
            self.start_invocation(request, sink)?;
            Ok(RuntimeInvocationReady {
                external_context_id: external
                    .unwrap_or_else(|| ExternalRuntimeContextId::new("native-thread").unwrap()),
                working_directory: cwd,
            })
        }
        fn deliver_prepared_invocation(
            &self,
            _: &AgentInvocationId,
        ) -> Result<(), RuntimePortError> {
            Ok(())
        }
        fn preflight_invocation(
            &self,
            _: RuntimeInvocationMode,
            options: &AgentRuntimeOptions,
        ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
            Ok(RuntimeInvocationPreflight {
                effective_options: options.clone(),
            })
        }
        fn start_invocation(
            &self,
            request: RuntimeInvocationRequest,
            sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.launches.lock().unwrap().push(request.clone());
            sink.emit_update(
                &request.invocation_id,
                RuntimeUpdate::Event(RuntimeEventDraft {
                    source: AgentRuntimeEventSource::Runtime,
                    raw_payload: json!({"kind":"runtime_request","requestId":"approval-1"}),
                    normalized: Some(NormalizedRuntimeEvent {
                        kind: NormalizedRuntimeEventKind::RuntimeContextEstablished,
                        text: None,
                        external_context_id: Some(
                            ExternalRuntimeContextId::new("native-thread").unwrap(),
                        ),
                        usage: None,
                        details: None,
                        tool_activity: None,
                    }),
                }),
            )?;
            *self.active.lock().unwrap() = Some((request, sink));
            Ok(())
        }
        fn resume_invocation(
            &self,
            request: RuntimeInvocationRequest,
            external: ExternalRuntimeContextId,
            sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            assert_eq!(external.as_str(), "native-thread");
            self.start_invocation(request, sink)
        }
        fn respond(
            &self,
            id: &AgentInvocationId,
            request: &str,
            response: RuntimeInteractionResponse,
        ) -> Result<(), RuntimePortError> {
            assert_eq!(request, "approval-1");
            assert_eq!(response, RuntimeInteractionResponse::Choose { choice_id: "accept".into() });
            let active = self.active.lock().unwrap();
            let (invocation, sink) = active.as_ref().unwrap();
            assert_eq!(&invocation.invocation_id, id);
            sink.emit_update(
                id,
                RuntimeUpdate::Event(RuntimeEventDraft {
                    source: AgentRuntimeEventSource::Runtime,
                    raw_payload: json!({"kind":"approval_received"}),
                    normalized: None,
                }),
            )
        }
        fn cancel_invocation(&self, id: &AgentInvocationId) -> Result<(), RuntimePortError> {
            let (invocation, sink) = self.active.lock().unwrap().take().unwrap();
            assert_eq!(&invocation.invocation_id, id);
            sink.emit_update(
                id,
                RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                    status: AgentInvocationTerminalStatus::Canceled,
                    exit_code: None,
                    signal: None,
                    runtime_error: None,
                }),
            )
        }
    }

    #[test]
    fn connected_requests_reach_running_invocation_and_resume_preserves_host_binding() {
        let directory = tempfile::tempdir().unwrap();
        let native_home = directory.path().join("native-home");
        fs::create_dir(&native_home).unwrap();
        let runtime = Arc::new(InteractiveRuntime::default());
        let mut host = Host::new(
            HostConfiguration {
                device_id: "server".into(),
                device_name: "Server".into(),
                configurations: vec![HostProviderConfiguration {
                    provider: "codex".into(),
                    id: "codex-default".into(),
                    executable: "fake-codex".into(),
                    home: native_home.clone(),
                }],
            },
            directory.path().join("bindings"),
        )
        .unwrap();
        host.runtimes
            .insert("codex-default".into(), runtime.clone());
        let host = Arc::new(host);
        let capture = Arc::new(Capture::default());
        let request = RuntimeInvocationRequest {
            session_id: AgentSessionId::new("session-1").unwrap(),
            invocation_id: AgentInvocationId::new("invocation-1").unwrap(),
            submitted_text: "work here".into(),
            working_directory: Some(directory.path().to_string_lossy().into_owned()),
            options: Default::default(),
            launch_extension: Some(RuntimeLaunchExtension {
                environment: vec![("CODEX_HOME".into(), "laptop-home".into())],
                ..Default::default()
            }),
        };
        let commands = [
            HostCommand::Invoke {
                provider: "codex".into(),
                configuration_ref: "codex-default".into(),
                request: request.clone(),
                external_context_id: None,
            },
            HostCommand::Respond {
                invocation_id: request.invocation_id.clone(),
                request_id: "approval-1".into(),
                response: RuntimeInteractionResponse::Choose { choice_id: "accept".into() },
            },
            HostCommand::Cancel {
                invocation_id: request.invocation_id.clone(),
            },
        ];
        let input = commands
            .into_iter()
            .enumerate()
            .map(|(id, command)| {
                serde_json::to_string(&HostRequest {
                    id: id.to_string(),
                    command,
                })
                .unwrap()
            })
            .collect::<Vec<_>>()
            .join("\n");
        connect(host.clone(), input.as_bytes(), capture.clone()).unwrap();
        let frames = capture.0.lock().unwrap();
        assert_eq!(
            frames
                .iter()
                .filter(|frame| matches!(frame, HostFrame::Response { error: None, .. }))
                .count(),
            3
        );
        assert!(frames.iter().any(|frame| matches!(frame, HostFrame::Update { invocation_id, update: RuntimeUpdate::Finished(outcome) } if invocation_id == &request.invocation_id && outcome.status == AgentInvocationTerminalStatus::Canceled)));
        drop(frames);
        let binding: SessionBinding =
            serde_json::from_slice(&fs::read(host.binding_path(&request.session_id)).unwrap())
                .unwrap();
        assert_eq!(
            binding.external_context_id.as_ref().unwrap().as_str(),
            "native-thread"
        );
        let mut continuation = request.clone();
        continuation.invocation_id = AgentInvocationId::new("invocation-2").unwrap();
        host.execute(
            HostCommand::Invoke {
                provider: "codex".into(),
                configuration_ref: "codex-default".into(),
                request: continuation.clone(),
                external_context_id: binding.external_context_id.clone(),
            },
            capture.clone(),
        )
        .unwrap();
        let launched = runtime.launches.lock().unwrap();
        let environment = &launched[1].launch_extension.as_ref().unwrap().environment;
        assert_eq!(
            environment
                .iter()
                .filter(|(key, _)| key == "CODEX_HOME")
                .count(),
            1
        );
        assert_eq!(environment[0].1, native_home.to_string_lossy());
        drop(launched);
        continuation.working_directory = Some(native_home.to_string_lossy().into_owned());
        assert!(host
            .execute(
                HostCommand::Invoke {
                    provider: "codex".into(),
                    configuration_ref: "codex-default".into(),
                    request: continuation,
                    external_context_id: binding.external_context_id
                },
                capture
            )
            .is_err());
    }
    #[test]
    fn discovery_replies_keep_request_identity_and_errors_do_not_end_connection() {
        let directory = tempfile::tempdir().unwrap();
        let host = Arc::new(
            Host::new(
                HostConfiguration {
                    device_id: "remote".into(),
                    device_name: "Server".into(),
                    configurations: vec![],
                },
                directory.path().into(),
            )
            .unwrap(),
        );
        let capture = Arc::new(Capture::default());
        let requests = b"{\"id\":\"1\",\"method\":\"cancel\",\"params\":{\"invocationId\":\"missing\"}}\n{\"id\":\"2\",\"method\":\"describe\"}\n";
        connect(host, &requests[..], capture.clone()).unwrap();
        let frames = capture.0.lock().unwrap();
        assert!(matches!(&frames[0], HostFrame::Response { id, error: Some(_), .. } if id == "1"));
        assert!(
            matches!(&frames[1], HostFrame::Response { id, result: Some(value), error: None } if id == "2" && value["deviceId"] == "remote")
        );
    }

    #[test]
    fn host_dispatches_typed_worktree_inspection() {
        use std::process::Command;

        fn git(root: &Path, args: &[&str]) {
            let output = Command::new("git")
                .current_dir(root)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let directory = tempfile::tempdir().unwrap();
        git(directory.path(), &["init", "-b", "main"]);
        git(directory.path(), &["config", "user.name", "Fixture"]);
        git(
            directory.path(),
            &["config", "user.email", "fixture@example.invalid"],
        );
        git(directory.path(), &["config", "core.autocrlf", "false"]);
        fs::write(directory.path().join("README.md"), "base\n").unwrap();
        git(directory.path(), &["add", "."]);
        git(directory.path(), &["commit", "-m", "base"]);
        fs::write(directory.path().join("README.md"), "staged\n").unwrap();
        git(directory.path(), &["add", "."]);

        let host = Host::new(
            HostConfiguration {
                device_id: "remote".into(),
                device_name: "Remote".into(),
                configurations: vec![],
            },
            directory.path().join("bindings"),
        )
        .unwrap();
        let value = host
            .execute(
                HostCommand::InspectWorktree {
                    worktree_root: directory.path().to_string_lossy().into_owned(),
                    compare_to_head: None,
                },
                Arc::new(Capture::default()),
            )
            .unwrap();
        let inspection: WorktreeInspection = serde_json::from_value(value).unwrap();
        assert_eq!(inspection.branch_ref.as_deref(), Some("refs/heads/main"));
        assert_eq!(inspection.staged.files, 1);
        assert!(!inspection.detached);
    }
    #[test]
    fn explicit_preparation_rebinds_idle_sessions_and_preserves_active_destination() {
        let folder = tempfile::tempdir().unwrap();
        let old = folder.path().join("old");
        let new = folder.path().join("new");
        fs::create_dir(&old).unwrap();
        fs::create_dir(&new).unwrap();
        let configuration = HostConfiguration {
            device_id: "remote".into(),
            device_name: "Remote".into(),
            configurations: vec![HostProviderConfiguration {
                provider: "codex".into(),
                id: "codex".into(),
                executable: "fake".into(),
                home: folder.path().into(),
            }],
        };
        let mut host = Host::new(configuration, folder.path().join("bindings")).unwrap();
        host.runtimes
            .insert("codex".into(), Arc::new(InteractiveRuntime::default()));
        let id = AgentSessionId::new("session").unwrap();
        let external = ExternalRuntimeContextId::new("native-thread").unwrap();
        let binding = SessionBinding {
            session_id: id.clone(),
            configuration_ref: "codex".into(),
            working_directory: old.to_string_lossy().into_owned(),
            external_context_id: Some(external.clone()),
        };
        fs::write(
            host.binding_path(&id),
            serde_json::to_vec(&binding).unwrap(),
        )
        .unwrap();
        let request = RuntimeInvocationRequest {
            session_id: id.clone(),
            invocation_id: AgentInvocationId::new("prepared").unwrap(),
            submitted_text: "continue".into(),
            working_directory: Some(new.to_string_lossy().into_owned()),
            options: Default::default(),
            launch_extension: None,
        };
        let capture = Arc::new(Capture::default());
        host.execute(
            HostCommand::PrepareInvocation {
                provider: "codex".into(),
                configuration_ref: "codex".into(),
                request: request.clone(),
                external_context_id: Some(external.clone()),
            },
            capture.clone(),
        )
        .unwrap();
        let actual: SessionBinding =
            serde_json::from_slice(&fs::read(host.binding_path(&id)).unwrap()).unwrap();
        assert_eq!(actual.working_directory, new.to_string_lossy());
        assert_eq!(actual.external_context_id, Some(external.clone()));
        let mut blocked = request.clone();
        blocked.invocation_id = AgentInvocationId::new("other").unwrap();
        blocked.working_directory = Some(old.to_string_lossy().into_owned());
        assert!(host
            .execute(
                HostCommand::PrepareInvocation {
                    provider: "codex".into(),
                    configuration_ref: "codex".into(),
                    request: blocked,
                    external_context_id: Some(external)
                },
                capture.clone()
            )
            .is_err());
        let actual: SessionBinding =
            serde_json::from_slice(&fs::read(host.binding_path(&id)).unwrap()).unwrap();
        assert_eq!(actual.working_directory, new.to_string_lossy());
        host.execute(
            HostCommand::Cancel {
                invocation_id: request.invocation_id,
            },
            capture,
        )
        .unwrap();
    }
}
