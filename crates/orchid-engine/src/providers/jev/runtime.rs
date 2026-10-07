use super::{
    client::{JevClient, ENDPOINT},
    contracts::{validate_model, JevEvaluation},
    DEFAULT_MODEL,
};
use crate::contracts::{domain::*, runtime::*};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

type CredentialReader = Arc<dyn Fn() -> Result<String, String> + Send + Sync>;

#[derive(Default)]
struct RuntimeState {
    active: HashMap<AgentInvocationId, CancellationToken>,
    stopping: bool,
}

pub struct JevRuntime {
    state: Arc<Mutex<RuntimeState>>,
    endpoint: String,
    credential: CredentialReader,
    timeout: Duration,
}

impl JevRuntime {
    pub fn system() -> Self {
        Self::new(
            ENDPOINT.into(),
            Arc::new(|| {
                std::env::var("TYPESAFE_API_KEY")
                    .ok()
                    .filter(|key| !key.trim().is_empty())
                    .ok_or_else(|| "Set TYPESAFE_API_KEY in Orchid's environment to run Jev".into())
            }),
            Duration::from_secs(30),
        )
    }

    pub fn new(endpoint: String, credential: CredentialReader, timeout: Duration) -> Self {
        Self {
            state: Default::default(),
            endpoint,
            credential,
            timeout,
        }
    }
}

fn unavailable(message: impl Into<String>) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::Unavailable, message)
}

fn unsupported(message: impl Into<String>) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::UnsupportedOptions, message)
}

fn emit(sink: &dyn AgentRuntimeUpdateSink, id: &AgentInvocationId, update: RuntimeUpdate) {
    if let Err(error) = sink.emit_update(id, update.clone()) {
        sink.report_delivery_failure(id, RuntimeUpdateDeliveryFailure { update, error });
    }
}

fn event(
    kind: NormalizedRuntimeEventKind,
    raw_payload: Value,
    text: Option<String>,
    details: Option<Value>,
    usage: Option<AgentRuntimeUsage>,
) -> RuntimeUpdate {
    RuntimeUpdate::Event(RuntimeEventDraft {
        source: AgentRuntimeEventSource::Runtime,
        raw_payload,
        normalized: Some(NormalizedRuntimeEvent {
            kind,
            text,
            details,
            usage,
            external_context_id: None,
            tool_activity: None,
        }),
    })
}

impl AgentRuntime for JevRuntime {
    fn preflight_invocation(
        &self,
        mode: RuntimeInvocationMode,
        options: &AgentRuntimeOptions,
    ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
        if mode != RuntimeInvocationMode::Start || options.sandbox.is_some() {
            return Err(unsupported("Jev evaluates independent requests; native resume and sandbox options are unavailable"));
        }
        let model = options.model.as_deref().unwrap_or(DEFAULT_MODEL);
        validate_model(model).map_err(unsupported)?;
        (self.credential)().map_err(unavailable)?;
        Ok(RuntimeInvocationPreflight {
            effective_options: AgentRuntimeOptions {
                model: Some(model.into()),
                sandbox: None,
            },
        })
    }

    fn start_invocation(
        &self,
        request: RuntimeInvocationRequest,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        if request.kind != RuntimeInvocationKind::Turn
            || !request.attachments.is_empty()
            || !request.content.context.is_empty()
        {
            return Err(unsupported("Jev requires a typed evaluation with explicit state and questions, without attachments or conversational context"));
        }
        if let Some(extension) = &request.launch_extension {
            if !extension.managed_mcp_servers.is_empty()
                || !extension.skill_inputs.is_empty()
                || !extension.invoked_skill_ids.is_empty()
                || extension.reasoning_mode.is_some()
                || extension.provider_options.is_some()
                || extension.native_mcp_enabled == Some(true)
                || extension.initial_prompt_prefix.is_some()
                || extension.executable.is_some()
                || !extension.environment.is_empty()
                || extension.ignore_user_rules
                || extension.sandbox_network_access
            {
                return Err(unsupported(
                    "Jev does not support agent tools, skills, or native launch settings",
                ));
            }
        }
        let preflight =
            self.preflight_invocation(RuntimeInvocationMode::Start, &request.options)?;
        let input: JevEvaluation = serde_json::from_str(&request.content.primary_query)
            .map_err(|_| unsupported("Jev requires a typed evaluation with state and questions"))?;
        input.validate().map_err(unsupported)?;
        let model = preflight.effective_options.model.unwrap();
        let client = JevClient::new(
            self.endpoint.clone(),
            (self.credential)().map_err(unavailable)?,
            self.timeout,
        )
        .map_err(unavailable)?;
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| unavailable("Unable to initialize Jev request executor"))?;
        let id = request.invocation_id;
        let cancel = CancellationToken::new();
        let mut state = self
            .state
            .lock()
            .map_err(|_| unavailable("Jev runtime unavailable"))?;
        if state.stopping {
            return Err(unavailable("Jev runtime is stopping"));
        }
        if state.active.contains_key(&id) {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::AlreadyActive,
                "Jev invocation is already active",
            ));
        }
        state.active.insert(id.clone(), cancel.clone());
        let worker_state = self.state.clone();
        let worker_id = id.clone();
        let started = std::thread::Builder::new()
            .name("jev-api-invocation".into())
            .spawn(move || {
                let mut status = AgentInvocationTerminalStatus::Failed;
                let mut failure = None;
                let result = executor.block_on(async {
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => None,
                        result = client.evaluate(&model, &input) => Some(result),
                    }
                });
                match result {
                    None => status = AgentInvocationTerminalStatus::Canceled,
                    Some(Ok((result, raw))) => {
                        emit(
                            sink.as_ref(),
                            &worker_id,
                            event(
                                NormalizedRuntimeEventKind::AgentMessage,
                                raw.clone(),
                                Some(raw.to_string()),
                                Some(json!({"role":"final", "structuredOutput":result})),
                                None,
                            ),
                        );
                        emit(
                            sink.as_ref(),
                            &worker_id,
                            event(
                                NormalizedRuntimeEventKind::Usage,
                                json!({"usage":result.usage}),
                                None,
                                None,
                                Some(AgentRuntimeUsage {
                                    input_tokens: Some(result.usage.input_tokens),
                                    output_tokens: Some(result.usage.output_tokens),
                                    cached_input_tokens: None,
                                }),
                            ),
                        );
                        emit(
                            sink.as_ref(),
                            &worker_id,
                            event(
                                NormalizedRuntimeEventKind::InvocationCompleted,
                                json!({"status":"completed"}),
                                None,
                                Some(json!({"status":"completed"})),
                                None,
                            ),
                        );
                        status = AgentInvocationTerminalStatus::Completed;
                    }
                    Some(Err(message)) => failure = Some(message),
                }
                if let Some(message) = &failure {
                    emit(
                        sink.as_ref(),
                        &worker_id,
                        event(
                            NormalizedRuntimeEventKind::RuntimeError,
                            json!({"error":message}),
                            Some(message.clone()),
                            None,
                            None,
                        ),
                    );
                }
                if let Ok(mut state) = worker_state.lock() {
                    state.active.remove(&worker_id);
                }
                emit(
                    sink.as_ref(),
                    &worker_id,
                    RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                        status,
                        exit_code: None,
                        signal: None,
                        runtime_error: failure.map(|message| AgentRuntimeFailure {
                            code: "jev_api".into(),
                            message,
                            details: None,
                        }),
                    }),
                );
            });
        if started.is_err() {
            state.active.remove(&id);
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::LaunchFailed,
                "Unable to launch Jev request",
            ));
        }
        Ok(())
    }

    fn resume_invocation(
        &self,
        _: RuntimeInvocationRequest,
        _: ExternalRuntimeContextId,
        _: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        Err(unsupported("Jev has no native conversation to resume; submit a fresh evaluation in the same Orchid session"))
    }

    fn cancel_invocation(&self, id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        let state = self
            .state
            .lock()
            .map_err(|_| unavailable("Jev runtime unavailable"))?;
        let token = state.active.get(id).ok_or_else(|| {
            RuntimePortError::new(
                RuntimePortErrorKind::NotActive,
                "Jev invocation is not active",
            )
        })?;
        token.cancel();
        Ok(())
    }

    fn shutdown(&self) -> Result<(), RuntimePortError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| unavailable("Jev runtime unavailable"))?;
        state.stopping = true;
        for token in state.active.values() {
            token.cancel();
        }
        Ok(())
    }
}
