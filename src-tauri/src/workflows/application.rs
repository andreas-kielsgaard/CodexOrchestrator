use super::domain::{
    WorkflowConnectionConfig, WorkflowDefinition, WorkflowElementRef, WorkflowHarnessConfig,
    WorkflowInstance, WorkflowInstanceRecord, WorkflowInstanceSession, WorkflowInstanceSummary,
    WorkflowLaunchPreparation, WorkflowNativeQuery, WorkflowNodeConfig, WorkflowRole,
    WorkflowSessionActivity, WorkflowTypeSummary,
};
use crate::agent_sessions::{
    application::{
        AgentSessionApplication, CreateAgentSessionCommand, CreateApplicationAgentSessionCommand,
        SendAgentSessionMessageCommand, SendIdempotentApplicationAgentSessionMessageCommand,
    },
    domain::{AgentInvocationId, AgentRuntimeOptions, AgentSessionId},
    ports::{InitialPromptPrefix, RuntimeLaunchExtension},
};
use chrono::Utc;
use std::{fs, path::PathBuf, sync::Arc};
use uuid::Uuid;

const SUPPORTED_CODEX_MODELS: [&str; 2] = ["gpt-5.6-sol", "gpt-5.6-terra"];

pub(crate) trait WorkflowRepository: Send + Sync {
    fn list_workflow_types(&self) -> Result<Vec<WorkflowTypeSummary>, String>;
    fn list_roles(&self) -> Result<Vec<WorkflowRole>, String>;
    fn create_role(
        &self,
        name: &str,
        harness: WorkflowHarnessConfig,
    ) -> Result<WorkflowRole, String>;
    fn update_role(
        &self,
        role_id: &str,
        name: &str,
        harness: WorkflowHarnessConfig,
    ) -> Result<WorkflowRole, String>;
    fn create_workflow_type(&self, name: &str) -> Result<WorkflowDefinition, String>;
    fn load_workflow_type(&self, workflow_type_id: &str) -> Result<WorkflowDefinition, String>;
    fn update_workflow_type(
        &self,
        workflow_type_id: &str,
        name: &str,
    ) -> Result<WorkflowDefinition, String>;
    fn save_node_draft(
        &self,
        workflow_type_id: &str,
        node: WorkflowNodeConfig,
    ) -> Result<WorkflowDefinition, String>;
    fn delete_node_draft(
        &self,
        workflow_type_id: &str,
        node_id: &str,
    ) -> Result<WorkflowDefinition, String>;
    fn detach_node_role(
        &self,
        workflow_type_id: &str,
        node_id: &str,
    ) -> Result<WorkflowDefinition, String>;
    fn save_node_as_role(
        &self,
        workflow_type_id: &str,
        node_id: &str,
        role_name: &str,
    ) -> Result<WorkflowDefinition, String>;
    fn save_connection_draft(
        &self,
        workflow_type_id: &str,
        connection: WorkflowConnectionConfig,
    ) -> Result<WorkflowDefinition, String>;
    fn delete_connection_draft(
        &self,
        workflow_type_id: &str,
        connection_id: &str,
    ) -> Result<WorkflowDefinition, String>;
    fn activate_changes(
        &self,
        workflow_type_id: &str,
        elements: &[WorkflowElementRef],
    ) -> Result<WorkflowDefinition, String>;
    fn native_query(&self) -> Result<WorkflowNativeQuery, String>;
    fn create_instance_launch(
        &self,
        preparation: WorkflowLaunchPreparation,
    ) -> Result<WorkflowInstanceRecord, String>;
    fn associate_instance_session(
        &self,
        activation_id: &str,
        workflow_instance_id: &str,
        node_id: &str,
        session_id: &str,
        associated_at: &str,
    ) -> Result<WorkflowInstanceRecord, String>;
    fn mark_instance_launch_requested(
        &self,
        activation_id: &str,
        requested_at: &str,
    ) -> Result<(), String>;
    fn mark_instance_launch_accepted(
        &self,
        activation_id: &str,
        accepted_at: &str,
    ) -> Result<(), String>;
    fn mark_instance_launch_failed(
        &self,
        activation_id: &str,
        stage: &str,
        reason: &str,
        failed_at: &str,
    ) -> Result<(), String>;
    fn list_workflow_instances(&self) -> Result<Vec<WorkflowInstanceRecord>, String>;
    fn load_workflow_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceRecord, String>;
}

#[derive(Clone, Debug)]
pub(crate) struct BindWorkflowSessionHarness {
    pub(crate) session_id: AgentSessionId,
    pub(crate) runtime_instance_id: String,
    pub(crate) workflow_instance_id: String,
    pub(crate) recipe_id: String,
    pub(crate) node_id: String,
    pub(crate) harness: WorkflowHarnessConfig,
}

pub(crate) trait WorkflowSessionHarnessBinder: Send + Sync {
    fn bind_workflow_session(&self, request: BindWorkflowSessionHarness) -> Result<(), String>;
}

pub(crate) struct WorkflowApplication {
    repository: Arc<dyn WorkflowRepository>,
    sessions: Arc<AgentSessionApplication>,
    harnesses: Arc<dyn WorkflowSessionHarnessBinder>,
    instance_root: PathBuf,
}

impl WorkflowApplication {
    pub(crate) fn new(
        repository: Arc<dyn WorkflowRepository>,
        sessions: Arc<AgentSessionApplication>,
        harnesses: Arc<dyn WorkflowSessionHarnessBinder>,
        instance_root: PathBuf,
    ) -> Self {
        Self {
            repository,
            sessions,
            harnesses,
            instance_root,
        }
    }

    pub(crate) fn list_workflow_types(&self) -> Result<Vec<WorkflowTypeSummary>, String> {
        self.repository.list_workflow_types()
    }

    pub(crate) fn list_roles(&self) -> Result<Vec<WorkflowRole>, String> {
        self.repository.list_roles()
    }

    pub(crate) fn create_role(
        &self,
        name: &str,
        harness: WorkflowHarnessConfig,
    ) -> Result<WorkflowRole, String> {
        self.repository.create_role(name, harness)
    }

    pub(crate) fn update_role(
        &self,
        role_id: &str,
        name: &str,
        harness: WorkflowHarnessConfig,
    ) -> Result<WorkflowRole, String> {
        self.repository.update_role(role_id, name, harness)
    }

    pub(crate) fn create_workflow_type(&self, name: &str) -> Result<WorkflowDefinition, String> {
        self.repository.create_workflow_type(name)
    }

    pub(crate) fn load_workflow_type(
        &self,
        workflow_type_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository.load_workflow_type(workflow_type_id)
    }

    pub(crate) fn update_workflow_type(
        &self,
        workflow_type_id: &str,
        name: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository.update_workflow_type(workflow_type_id, name)
    }

    pub(crate) fn save_node_draft(
        &self,
        workflow_type_id: &str,
        node: WorkflowNodeConfig,
    ) -> Result<WorkflowDefinition, String> {
        self.repository.save_node_draft(workflow_type_id, node)
    }

    pub(crate) fn delete_node_draft(
        &self,
        workflow_type_id: &str,
        node_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository.delete_node_draft(workflow_type_id, node_id)
    }

    pub(crate) fn detach_node_role(
        &self,
        workflow_type_id: &str,
        node_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository.detach_node_role(workflow_type_id, node_id)
    }

    pub(crate) fn save_node_as_role(
        &self,
        workflow_type_id: &str,
        node_id: &str,
        role_name: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository
            .save_node_as_role(workflow_type_id, node_id, role_name)
    }

    pub(crate) fn save_connection_draft(
        &self,
        workflow_type_id: &str,
        connection: WorkflowConnectionConfig,
    ) -> Result<WorkflowDefinition, String> {
        self.repository
            .save_connection_draft(workflow_type_id, connection)
    }

    pub(crate) fn delete_connection_draft(
        &self,
        workflow_type_id: &str,
        connection_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository
            .delete_connection_draft(workflow_type_id, connection_id)
    }

    pub(crate) fn activate_changes(
        &self,
        workflow_type_id: &str,
        elements: &[WorkflowElementRef],
    ) -> Result<WorkflowDefinition, String> {
        self.repository.activate_changes(workflow_type_id, elements)
    }

    pub(crate) fn native_query(&self) -> Result<WorkflowNativeQuery, String> {
        self.repository.native_query()
    }

    pub(crate) fn launch_workflow_instance(
        &self,
        workflow_type_id: &str,
        name: Option<&str>,
        starting_prompt: &str,
    ) -> Result<WorkflowInstance, String> {
        if starting_prompt.trim().is_empty() {
            return Err("A starting prompt is required.".to_string());
        }
        let definition = self.repository.load_workflow_type(workflow_type_id)?;
        let recipe = definition
            .active_recipe
            .ok_or_else(|| "Activate the Workflow type before launching it.".to_string())?;
        let starts = recipe
            .nodes
            .iter()
            .filter(|node| node.is_starting_point)
            .collect::<Vec<_>>();
        if starts.len() != 1 {
            return Err(format!(
                "The active recipe must contain exactly one starting node; found {}.",
                starts.len()
            ));
        }
        let start = starts[0];
        let launch_extension = launch_extension(&start.harness)?;
        let requested_options = AgentRuntimeOptions {
            model: nonempty(&start.harness.runtime.model),
            sandbox: None,
        };
        let instance_id = format!("workflow-instance-{}", Uuid::new_v4());
        let session_id = format!("workflow-session-{}", Uuid::new_v4());
        let invocation_id = format!("workflow-invocation-{}", Uuid::new_v4());
        let activation_id = format!("workflow-activation-{}", Uuid::new_v4());
        let instance_name = name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                crate::agent_sessions::application::title_from_message(starting_prompt)
            });
        let working_directory = self.instance_root.join(&instance_id);
        fs::create_dir_all(&working_directory)
            .map_err(|error| format!("Unable to create Workflow instance directory: {error}"))?;
        let working_directory = working_directory.to_string_lossy().into_owned();
        let requested_at = Utc::now().to_rfc3339();
        let preparation = WorkflowLaunchPreparation {
            instance_id: instance_id.clone(),
            workflow_type_id: workflow_type_id.to_string(),
            recipe_id: recipe.id.clone(),
            name: instance_name.clone(),
            starting_prompt: starting_prompt.to_string(),
            working_directory: working_directory.clone(),
            activation_id: activation_id.clone(),
            target_node_id: start.id.clone(),
            target_session_id: session_id.clone(),
            target_invocation_id: invocation_id.clone(),
            requested_at,
        };
        self.repository.create_instance_launch(preparation)?;

        let session = AgentSessionId::new(session_id.clone()).map_err(|error| error.to_string())?;
        if let Err(error) =
            self.sessions
                .create_application_session(CreateApplicationAgentSessionCommand {
                    session_id: session.clone(),
                    session: CreateAgentSessionCommand {
                        title: Some(instance_name),
                        working_directory: Some(working_directory.clone()),
                        requested_options: requested_options.clone(),
                    },
                })
        {
            self.record_failure(&activation_id, "session_creation", &error.to_string());
            return self.load_workflow_instance(&instance_id);
        }
        let associated_at = Utc::now().to_rfc3339();
        if let Err(error) = self.repository.associate_instance_session(
            &activation_id,
            &instance_id,
            &start.id,
            &session_id,
            &associated_at,
        ) {
            self.record_failure(&activation_id, "session_association", &error);
            return self.load_workflow_instance(&instance_id);
        }
        if let Err(error) = self.harnesses.bind_workflow_session(BindWorkflowSessionHarness {
            session_id: session.clone(),
            runtime_instance_id: invocation_id.clone(),
            workflow_instance_id: instance_id.clone(),
            recipe_id: recipe.id.clone(),
            node_id: start.id.clone(),
            harness: start.harness.clone(),
        }) {
            self.record_failure(&activation_id, "harness_binding", &error);
            return self.load_workflow_instance(&instance_id);
        }
        if let Err(error) = self
            .repository
            .mark_instance_launch_requested(&activation_id, &Utc::now().to_rfc3339())
        {
            self.record_failure(&activation_id, "launch_request_recording", &error);
            return self.load_workflow_instance(&instance_id);
        }
        let invocation =
            AgentInvocationId::new(invocation_id).map_err(|error| error.to_string())?;
        let launch = self
            .sessions
            .send_idempotent_user_message_with_launch_observation(
                SendIdempotentApplicationAgentSessionMessageCommand {
                    invocation_id: invocation,
                    message: SendAgentSessionMessageCommand {
                        session_id: Some(session),
                        submitted_text: starting_prompt.to_string(),
                        title: None,
                        working_directory: Some(working_directory),
                        requested_options: Some(requested_options),
                    },
                },
                launch_extension,
            );
        match launch {
            Ok(result) if result.launch_accepted => self
                .repository
                .mark_instance_launch_accepted(&activation_id, &Utc::now().to_rfc3339())?,
            Ok(_) => {
                self.record_failure(
                    &activation_id,
                    "runtime_launch",
                    "The Agent Session launch was not accepted.",
                );
            }
            Err(error) => {
                self.record_failure(&activation_id, "runtime_launch", &error.to_string());
            }
        }
        self.load_workflow_instance(&instance_id)
    }

    pub(crate) fn list_workflow_instances(&self) -> Result<Vec<WorkflowInstanceSummary>, String> {
        self.repository
            .list_workflow_instances()?
            .into_iter()
            .map(|record| {
                self.project_instance(record)
                    .map(|instance| instance.summary)
            })
            .collect()
    }

    pub(crate) fn load_workflow_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstance, String> {
        self.project_instance(
            self.repository
                .load_workflow_instance(workflow_instance_id)?,
        )
    }

    fn project_instance(&self, record: WorkflowInstanceRecord) -> Result<WorkflowInstance, String> {
        let mut sessions = Vec::with_capacity(record.session_associations.len());
        for association in &record.session_associations {
            let session_id = AgentSessionId::new(association.session_id.clone())
                .map_err(|error| error.to_string())?;
            let history = self
                .sessions
                .load_session(&session_id)
                .map_err(|error| error.to_string())?;
            let active = history
                .invocations
                .iter()
                .any(|entry| entry.invocation.status.is_active());
            sessions.push(WorkflowInstanceSession {
                node_id: association.node_id.clone(),
                session_id: association.session_id.clone(),
                title: history.session.title,
                activity: if active {
                    WorkflowSessionActivity::Active
                } else {
                    WorkflowSessionActivity::Idle
                },
                latest_turn_summary: history
                    .invocations
                    .last()
                    .map(|entry| summarize(&entry.invocation.submitted_text)),
                associated_at: association.associated_at.clone(),
            });
        }
        let active_session_count = sessions
            .iter()
            .filter(|session| session.activity == WorkflowSessionActivity::Active)
            .count() as u32;
        let summary = WorkflowInstanceSummary {
            id: record.id.clone(),
            workflow_type_id: record.workflow_type_id.clone(),
            workflow_type_name: record.workflow_type_name.clone(),
            recipe_id: record.recipe.id.clone(),
            name: record.name.clone(),
            session_count: sessions.len() as u32,
            active_session_count,
            idle_session_count: sessions.len() as u32 - active_session_count,
            launch_status: record.launch_activation.status,
            created_at: record.created_at.clone(),
        };
        Ok(WorkflowInstance {
            summary,
            starting_prompt: record.starting_prompt,
            working_directory: record.working_directory,
            recipe: record.recipe,
            sessions,
            launch_activation: record.launch_activation,
        })
    }

    fn record_failure(&self, activation_id: &str, stage: &str, reason: &str) {
        let _ = self.repository.mark_instance_launch_failed(
            activation_id,
            stage,
            reason,
            &Utc::now().to_rfc3339(),
        );
    }
}

fn launch_extension(
    harness: &WorkflowHarnessConfig,
) -> Result<Option<RuntimeLaunchExtension>, String> {
    if !harness.skills.is_empty() {
        return Err("Workflow launch does not support Harness skills yet.".to_string());
    }
    if !harness.hooks.is_empty() {
        return Err("Workflow launch does not support Harness hooks yet.".to_string());
    }
    let provider = harness.runtime.provider.trim();
    if !provider.is_empty() && !provider.eq_ignore_ascii_case("codex") {
        return Err(format!(
            "Workflow launch supports only the Codex provider; found {}.",
            harness.runtime.provider
        ));
    }
    validate_supported_codex_model(&harness.runtime.model)?;
    let reasoning = harness.runtime.reasoning_effort.trim();
    if !reasoning.is_empty() && !["low", "medium", "high", "xhigh"].contains(&reasoning) {
        return Err(format!(
            "Unsupported Codex reasoning effort {reasoning}; use low, medium, high, or xhigh."
        ));
    }
    if harness.instructions.contains('\0') || harness.instructions.len() > 65_536 {
        return Err(
            "Workflow Harness instructions are invalid for direct prompt delivery.".to_string(),
        );
    }
    let mut extension = RuntimeLaunchExtension::default();
    if !reasoning.is_empty() {
        extension.additional_args = vec![
            "-c".to_string(),
            format!("model_reasoning_effort=\"{reasoning}\""),
        ];
    }
    if let Some(instructions) = nonempty(&harness.instructions) {
        extension.initial_prompt_prefix = Some(InitialPromptPrefix {
            source: "workflow_recipe_node_instructions".to_string(),
            version: 1,
            content: instructions,
        });
    }
    Ok((extension != RuntimeLaunchExtension::default()).then_some(extension))
}

fn validate_runtime_literal(value: &str, label: &str) -> Result<(), String> {
    if value != value.trim()
        || value.len() > 128
        || value.chars().any(|character| character.is_control())
    {
        return Err(format!("Workflow Harness {label} is invalid."));
    }
    Ok(())
}

fn validate_supported_codex_model(value: &str) -> Result<(), String> {
    validate_runtime_literal(value, "model")?;
    let model = value.trim();
    if !model.is_empty() && !SUPPORTED_CODEX_MODELS.contains(&model) {
        return Err(format!(
            "Unsupported Codex model {model}; use {}.",
            SUPPORTED_CODEX_MODELS.join(" or ")
        ));
    }
    Ok(())
}

fn nonempty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_string())
}

fn summarize(value: &str) -> String {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if value.chars().count() <= 120 {
        value
    } else {
        format!("{}...", value.chars().take(117).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent_sessions::{
            application::{
                AgentSessionNotification, AgentSessionNotifier, SystemAgentSessionProviders,
            },
            domain::{AgentInvocationId, AgentRuntimeOptions, ExternalRuntimeContextId},
            ports::{
                AgentRuntime, AgentRuntimeUpdateSink, RuntimeInvocationMode,
                RuntimeInvocationPreflight, RuntimeInvocationRequest, RuntimePortError,
            },
            repository::SqliteAgentSessionRepository,
        },
        workflows::{
            domain::{
                WorkflowElementKind, WorkflowElementRef, WorkflowHarnessRuntimeSettings,
                WorkflowLaunchStatus, WorkflowNodeHarness,
            },
            repository::SqliteWorkflowRepository,
        },
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };

    #[derive(Default)]
    struct RecordingRuntime {
        launches: Mutex<Vec<RuntimeInvocationRequest>>,
        fail_preflight: AtomicBool,
    }

    impl AgentRuntime for RecordingRuntime {
        fn preflight_invocation(
            &self,
            _: RuntimeInvocationMode,
            requested_options: &AgentRuntimeOptions,
        ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
            if self.fail_preflight.load(Ordering::SeqCst) {
                return Err(RuntimePortError::new(
                    crate::agent_sessions::ports::RuntimePortErrorKind::UnsupportedOptions,
                    "unsupported requested options",
                ));
            }
            Ok(RuntimeInvocationPreflight {
                effective_options: requested_options.clone(),
            })
        }

        fn start_invocation(
            &self,
            request: RuntimeInvocationRequest,
            _: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.launches.lock().unwrap().push(request);
            Ok(())
        }

        fn resume_invocation(
            &self,
            _: RuntimeInvocationRequest,
            _: ExternalRuntimeContextId,
            _: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            panic!("a fresh Workflow launch cannot resume")
        }

        fn cancel_invocation(&self, _: &AgentInvocationId) -> Result<(), RuntimePortError> {
            Ok(())
        }
    }

    struct RecordingHarnessBinder;

    impl WorkflowSessionHarnessBinder for RecordingHarnessBinder {
        fn bind_workflow_session(&self, _: BindWorkflowSessionHarness) -> Result<(), String> {
            Ok(())
        }
    }

    struct FailingHarnessBinder;

    impl WorkflowSessionHarnessBinder for FailingHarnessBinder {
        fn bind_workflow_session(&self, _: BindWorkflowSessionHarness) -> Result<(), String> {
            Err("managed upstream is unavailable".to_string())
        }
    }

    struct NoopNotifier;

    impl AgentSessionNotifier for NoopNotifier {
        fn notify(&self, _: AgentSessionNotification) -> Result<(), String> {
            Ok(())
        }
    }

    fn fixture() -> (
        tempfile::TempDir,
        Arc<SqliteWorkflowRepository>,
        Arc<RecordingRuntime>,
        WorkflowApplication,
    ) {
        fixture_with_binder(Arc::new(RecordingHarnessBinder))
    }

    fn fixture_with_binder(
        harnesses: Arc<dyn WorkflowSessionHarnessBinder>,
    ) -> (
        tempfile::TempDir,
        Arc<SqliteWorkflowRepository>,
        Arc<RecordingRuntime>,
        WorkflowApplication,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("workflow-launch.sqlite");
        let agent_connection = crate::storage::open_active_database(&database_path).unwrap();
        let agent_repository =
            Arc::new(SqliteAgentSessionRepository::new(agent_connection).unwrap());
        let runtime = Arc::new(RecordingRuntime::default());
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            agent_repository,
            runtime.clone(),
            Arc::new(NoopNotifier),
            providers.clone(),
            providers,
            Some("codex-test".to_string()),
        ));
        let workflows = Arc::new(SqliteWorkflowRepository::open(&database_path).unwrap());
        let application = WorkflowApplication::new(
            workflows.clone(),
            sessions,
            harnesses,
            directory.path().join("workflow-instances"),
        );
        (directory, workflows, runtime, application)
    }

    fn activate_start(
        repository: &SqliteWorkflowRepository,
        mut harness: WorkflowHarnessConfig,
    ) -> String {
        let definition = repository.create_workflow_type("Review loop").unwrap();
        let workflow_type_id = definition.workflow_type.id;
        if harness.harness_name.is_empty() {
            harness.harness_name = "Architecture reviewer".to_string();
        }
        repository
            .save_node_draft(
                &workflow_type_id,
                WorkflowNodeConfig {
                    id: "start".to_string(),
                    name: "Review architecture".to_string(),
                    harness_name: harness.harness_name.clone(),
                    role_name: None,
                    position_x: 80.0,
                    position_y: 100.0,
                    is_starting_point: true,
                    harness: Some(WorkflowNodeHarness::Standalone { config: harness }),
                },
            )
            .unwrap();
        repository
            .activate_changes(
                &workflow_type_id,
                &[WorkflowElementRef {
                    kind: WorkflowElementKind::Node,
                    id: "start".to_string(),
                }],
            )
            .unwrap();
        workflow_type_id
    }

    #[test]
    fn launch_materializes_one_human_prompt_without_claiming_provider_activity() {
        let (_directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig {
                harness_name: "Architecture reviewer".to_string(),
                role_identity: "Reviewer".to_string(),
                instructions: "Review the architecture carefully.".to_string(),
                skills: Vec::new(),
                mcp_servers: Vec::new(),
                hooks: Vec::new(),
                runtime: WorkflowHarnessRuntimeSettings {
                    provider: "codex".to_string(),
                    model: "gpt-5.6-sol".to_string(),
                    reasoning_effort: "high".to_string(),
                },
            },
        );
        let prompt = "Inspect the proposed architecture and identify the main risk.";

        let instance = application
            .launch_workflow_instance(&workflow_type_id, None, prompt)
            .unwrap();

        assert_eq!(
            instance.summary.launch_status,
            WorkflowLaunchStatus::LaunchAccepted
        );
        assert_eq!(instance.summary.session_count, 1);
        assert_eq!(instance.summary.active_session_count, 1);
        assert_eq!(instance.summary.name, prompt);
        assert_eq!(instance.starting_prompt, prompt);
        assert_eq!(instance.launch_activation.source_kind, "human");
        assert_eq!(
            instance.launch_activation.delivery_kind,
            "direct_prompt_runtime_v1"
        );
        assert_eq!(instance.launch_activation.session_mode, "fresh");
        assert_eq!(instance.launch_activation.context_inheritance, "none");
        assert_eq!(instance.launch_activation.compression, "none");
        assert!(instance.launch_activation.associated_at.is_some());
        assert!(instance.launch_activation.launch_requested_at.is_some());
        assert!(instance.launch_activation.launch_accepted_at.is_some());
        assert!(PathBuf::from(&instance.working_directory).is_dir());

        let launches = runtime.launches.lock().unwrap();
        assert_eq!(launches.len(), 1);
        assert!(launches[0].submitted_text.contains(prompt));
        assert!(launches[0]
            .submitted_text
            .contains("Review the architecture carefully."));
        assert_eq!(launches[0].options.model.as_deref(), Some("gpt-5.6-sol"));
        let extension = launches[0].launch_extension.as_ref().unwrap();
        assert_eq!(
            extension
                .initial_prompt_prefix
                .as_ref()
                .map(|prefix| prefix.content.as_str()),
            Some("Review the architecture carefully.")
        );
        assert_eq!(
            extension.additional_args,
            ["-c", "model_reasoning_effort=\"high\""]
        );
        drop(launches);

        let loaded = application
            .sessions
            .load_session(&AgentSessionId::new(instance.sessions[0].session_id.clone()).unwrap())
            .unwrap();
        assert_eq!(loaded.invocations.len(), 1);
        assert_eq!(loaded.invocations[0].invocation.submitted_text, prompt);
        assert_eq!(
            loaded.invocations[0].invocation.input_provenance,
            crate::agent_sessions::domain::AgentInvocationInputProvenance::User
        );
        assert!(loaded.invocations[0].invocation.status.is_active());

        let listed = application.list_workflow_instances().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, instance.summary.id);
    }

    #[test]
    fn workflow_instance_projection_reopens_with_agent_session_history() {
        let (directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig {
                harness_name: "Architecture reviewer".to_string(),
                instructions: "Review the architecture carefully.".to_string(),
                runtime: WorkflowHarnessRuntimeSettings {
                    provider: "codex".to_string(),
                    model: "gpt-5.6-terra".to_string(),
                    reasoning_effort: "medium".to_string(),
                },
                ..WorkflowHarnessConfig::default()
            },
        );
        let prompt = "Review this durable Workflow launch.";
        let launched = application
            .launch_workflow_instance(&workflow_type_id, Some("Durable review"), prompt)
            .unwrap();
        let instance_id = launched.summary.id.clone();
        let recipe_id = launched.recipe.id.clone();
        let session_id = launched.sessions[0].session_id.clone();

        drop(application);
        drop(repository);
        drop(runtime);

        let database_path = directory.path().join("workflow-launch.sqlite");
        let agent_connection = crate::storage::open_active_database(&database_path).unwrap();
        let agent_repository =
            Arc::new(SqliteAgentSessionRepository::new(agent_connection).unwrap());
        let reopened_runtime = Arc::new(RecordingRuntime::default());
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            agent_repository,
            reopened_runtime.clone(),
            Arc::new(NoopNotifier),
            providers.clone(),
            providers,
            Some("codex-test".to_string()),
        ));
        let workflows = Arc::new(SqliteWorkflowRepository::open(&database_path).unwrap());
        let reopened = WorkflowApplication::new(
            workflows,
            sessions,
            Arc::new(RecordingHarnessBinder),
            directory.path().join("workflow-instances"),
        );

        let loaded = reopened.load_workflow_instance(&instance_id).unwrap();
        assert_eq!(loaded.summary.name, "Durable review");
        assert_eq!(loaded.summary.launch_status, WorkflowLaunchStatus::LaunchAccepted);
        assert_eq!(loaded.recipe.id, recipe_id);
        assert_eq!(loaded.starting_prompt, prompt);
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].session_id, session_id);
        assert_eq!(loaded.sessions[0].latest_turn_summary.as_deref(), Some(prompt));
        assert_eq!(reopened.list_workflow_instances().unwrap().len(), 1);
        assert!(reopened_runtime.launches.lock().unwrap().is_empty());
    }

    #[test]
    fn unsupported_harness_capabilities_fail_before_instance_or_runtime_creation() {
        let (_directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig {
                harness_name: "Reviewer".to_string(),
                skills: vec!["security-review".to_string()],
                ..WorkflowHarnessConfig::default()
            },
        );

        let error = application
            .launch_workflow_instance(&workflow_type_id, None, "Review this.")
            .unwrap_err();

        assert!(error.contains("does not support Harness skills yet"));
        assert!(application.list_workflow_instances().unwrap().is_empty());
        assert!(runtime.launches.lock().unwrap().is_empty());
    }

    #[test]
    fn unsupported_model_fails_before_instance_or_runtime_creation() {
        let (_directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig {
                harness_name: "Reviewer".to_string(),
                runtime: WorkflowHarnessRuntimeSettings {
                    provider: "codex".to_string(),
                    model: "unverified-model".to_string(),
                    reasoning_effort: "medium".to_string(),
                },
                ..WorkflowHarnessConfig::default()
            },
        );

        let error = application
            .launch_workflow_instance(&workflow_type_id, None, "Review this.")
            .unwrap_err();

        assert!(error.contains("Unsupported Codex model unverified-model"));
        assert!(application.list_workflow_instances().unwrap().is_empty());
        assert!(runtime.launches.lock().unwrap().is_empty());
    }

    #[test]
    fn runtime_rejection_retains_the_failed_instance_without_claiming_acceptance() {
        let (_directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig {
                harness_name: "Reviewer".to_string(),
                runtime: WorkflowHarnessRuntimeSettings {
                    provider: "codex".to_string(),
                    model: "gpt-5.6-sol".to_string(),
                    reasoning_effort: "medium".to_string(),
                },
                ..WorkflowHarnessConfig::default()
            },
        );
        runtime.fail_preflight.store(true, Ordering::SeqCst);

        let instance = application
            .launch_workflow_instance(&workflow_type_id, Some("Rejected review"), "Review this.")
            .unwrap();

        assert_eq!(instance.summary.launch_status, WorkflowLaunchStatus::Failed);
        assert_eq!(instance.summary.name, "Rejected review");
        assert!(instance.launch_activation.launch_requested_at.is_some());
        assert!(instance.launch_activation.launch_accepted_at.is_none());
        assert!(instance.launch_activation.failed_at.is_some());
        assert_eq!(
            instance.launch_activation.failure_stage.as_deref(),
            Some("runtime_launch")
        );
        assert!(instance
            .launch_activation
            .failure_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("launch was not accepted")));
        assert!(runtime.launches.lock().unwrap().is_empty());
    }

    #[test]
    fn harness_binding_failure_is_durable_and_prevents_runtime_launch() {
        let (_directory, repository, runtime, application) =
            fixture_with_binder(Arc::new(FailingHarnessBinder));
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig {
                harness_name: "Reviewer".to_string(),
                mcp_servers: vec![crate::workflows::domain::WorkflowMcpServerExposure {
                    server_name: "plan_builder".to_string(),
                    access: crate::workflows::domain::WorkflowMcpServerAccess::EntireServer,
                }],
                ..WorkflowHarnessConfig::default()
            },
        );

        let instance = application
            .launch_workflow_instance(&workflow_type_id, None, "Review this.")
            .unwrap();

        assert_eq!(instance.summary.launch_status, WorkflowLaunchStatus::Failed);
        assert_eq!(
            instance.launch_activation.failure_stage.as_deref(),
            Some("harness_binding")
        );
        assert!(instance
            .launch_activation
            .failure_reason
            .as_deref()
            .unwrap()
            .contains("managed upstream"));
        assert!(instance.launch_activation.launch_requested_at.is_none());
        assert!(runtime.launches.lock().unwrap().is_empty());
    }
}
