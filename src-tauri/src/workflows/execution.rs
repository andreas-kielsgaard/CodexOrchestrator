use super::authoring_service::WorkflowAuthoringService;
use super::{
    compiler::WorkflowCompiler,
    instance_domain::ResolvedRepoBranchWorktreeTarget,
    instances::{RecipeInstance, WorkflowEventAttempt, WorkflowInstanceStore},
};
use crate::agent_sessions::ports::AgentSessionRepository;
use crate::session_events::{
    ReferenceIdentity, SessionDirectory, SessionDirectoryEntry, SessionEventApplication,
    SessionEventDefinition, SessionEventOccurrence, SessionEventOccurrenceTrigger,
    SessionEventResult, SessionEventTriggerBinding,
};
use std::{collections::BTreeMap, sync::Arc};
use uuid::Uuid;

pub(crate) struct WorkflowExecutionService {
    authoring: Arc<WorkflowAuthoringService>,
    session_events: Arc<SessionEventApplication>,
    pub(crate) instances: Arc<WorkflowInstanceStore>,
    pub(crate) directory: Arc<dyn SessionDirectory>,
    pub(crate) sessions: Arc<dyn AgentSessionRepository>,
    record_observer: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl WorkflowExecutionService {
    pub(crate) fn new(
        authoring: Arc<WorkflowAuthoringService>,
        session_events: Arc<SessionEventApplication>,
        instances: Arc<WorkflowInstanceStore>,
        directory: Arc<dyn SessionDirectory>,
        sessions: Arc<dyn AgentSessionRepository>,
    ) -> Self {
        Self {
            authoring,
            session_events,
            instances,
            directory,
            sessions,
            record_observer: None,
        }
    }

    pub(crate) fn with_record_observer(
        mut self,
        observer: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> Self {
        self.record_observer = Some(observer);
        self
    }

    pub(crate) fn create_instance(
        &self,
        recipe_id: &str,
        expected_revision: u64,
        name: String,
        target: ResolvedRepoBranchWorktreeTarget,
    ) -> Result<RecipeInstance, String> {
        let recipe = self
            .authoring
            .load(recipe_id)?
            .active
            .ok_or("Activate the Workflow before creating an instance")?;
        if recipe.revision != expected_revision {
            return Err(
                "The active Workflow has changed. Reload before creating an instance.".into(),
            );
        }
        self.instances.create(name, recipe, target)
    }

    pub(crate) fn instance_sessions(
        &self,
        instance: &RecipeInstance,
    ) -> Result<Vec<SessionDirectoryEntry>, String> {
        let mut sessions = Vec::new();
        for node in &instance.recipe.nodes {
            let address = super::address_references::workflow_node_address(
                &super::address_references::WorkflowInstanceReference::new(&instance.id)
                    .map_err(|error| error.to_string())?,
                &super::address_references::WorkflowNodeReference::new(&node.node_id)
                    .map_err(|error| error.to_string())?,
            );
            sessions.extend(
                self.directory
                    .list_at_address(&address)
                    .map_err(|error| error.to_string())?,
            );
        }
        Ok(sessions)
    }

    pub(crate) fn compile_instance(
        &self,
        instance_id: &str,
        node_id: Option<&str>,
    ) -> Result<Vec<SessionEventDefinition>, String> {
        let mut instance = self.instances.load(instance_id)?;
        if let Some(node_id) = node_id {
            instance.recipe.starting_node_id = Some(node_id.into());
        }
        let mut definitions = WorkflowCompiler::compile(
            instance
                .recipe
                .runtime_compilation_input(instance_id, &instance.target.worktree.path)?,
        )
        .map_err(|error| error.to_string())?;
        super::prompt_content::reference_fixed_prompts(&instance, &mut definitions)?;
        Ok(definitions)
    }

    pub(crate) fn dispatch_user_request(
        &self,
        recipe_id: &str,
        instance_id: &str,
        text: String,
    ) -> Result<SessionEventResult, String> {
        self.dispatch_node_user_request(recipe_id, instance_id, None, text)
    }

    pub(crate) fn dispatch_node_user_request(
        &self,
        recipe_id: &str,
        instance_id: &str,
        node_id: Option<&str>,
        text: String,
    ) -> Result<SessionEventResult, String> {
        if text.trim().is_empty() {
            return Err("Workflow user request must contain text".into());
        }
        if self.instances.load(instance_id)?.recipe.recipe_id != recipe_id {
            return Err("Instance belongs to another Workflow".into());
        }
        let definitions = self.compile_instance(instance_id, node_id)?;
        let mut entries = definitions.iter().filter(|definition| {
            matches!(definition.trigger, SessionEventTriggerBinding::UserRequest)
        });
        let definition = entries
            .next()
            .ok_or_else(|| "Active Workflow has no user-entry Session Event".to_string())?;
        if entries.next().is_some() {
            return Err("Active Workflow has more than one user-entry Session Event".into());
        }
        let request = ReferenceIdentity::new(
            "orchestrator.user",
            "request",
            format!("request-{}", Uuid::new_v4()),
        )
        .map_err(|error| error.to_string())?;
        self.dispatch_definition(
            instance_id,
            definition,
            SessionEventOccurrence {
                event_group_id: event_group_identity()?,
                trigger: SessionEventOccurrenceTrigger::UserRequest { request, text },
                referenced_content: BTreeMap::new(),
                created_by_session: None,
                direct_user_options: None,
            },
        )
    }

    pub(crate) fn dispatch_compiled_occurrence(
        &self,
        recipe_id: &str,
        instance_id: &str,
        definition_ref: &ReferenceIdentity,
        occurrence: SessionEventOccurrence,
    ) -> Result<SessionEventResult, String> {
        if self.instances.load(instance_id)?.recipe.recipe_id != recipe_id {
            return Err("Instance belongs to another Workflow".into());
        }
        let definitions = self.compile_instance(instance_id, None)?;
        let definition = definitions
            .iter()
            .find(|definition| &definition.definition_ref == definition_ref)
            .ok_or_else(|| {
                format!(
                    "Active Workflow does not contain Session Event definition `{definition_ref}`"
                )
            })?;
        self.dispatch_definition(instance_id, definition, occurrence)
    }

    pub(crate) fn dispatch_definition(
        &self,
        instance_id: &str,
        definition: &SessionEventDefinition,
        mut occurrence: SessionEventOccurrence,
    ) -> Result<SessionEventResult, String> {
        let attempt = WorkflowEventAttempt {
            id: occurrence.event_group_id.id().into(),
            instance_id: instance_id.into(),
            definition_ref: definition.definition_ref.clone(),
            source_session_id: occurrence
                .created_by_session
                .as_ref()
                .map(|reference| reference.id().into()),
            created_at: chrono::Utc::now().to_rfc3339(),
            event_group: None,
            error: None,
        };
        if !self.instances.begin_attempt(&attempt)? {
            return Err(
                "This Workflow event has already been handled; see its recorded attempt.".into(),
            );
        }
        let result = (|| {
            let instance = self.instances.load(instance_id)?;
            super::prompt_content::supply_referenced_content(
                &instance,
                definition,
                &mut occurrence,
                self.sessions.as_ref(),
            )?;
            self.session_events
                .dispatch_occurrence(definition, occurrence)
                .map_err(|error| error.to_string())
        })();
        self.instances.finish_attempt(attempt, &result)?;
        if let Some(observer) = &self.record_observer {
            observer(instance_id);
        }
        result
    }
}

fn event_group_identity() -> Result<ReferenceIdentity, String> {
    ReferenceIdentity::new(
        "workflow",
        "event_group",
        format!("event-group-{}", Uuid::new_v4()),
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        execution_configuration::{
            CapabilityProfileService, CapabilitySet, InMemoryCapabilityProfileRepository,
            NodeProfile, RuntimeProfileSnapshot, RuntimeSelections, SelectedRuntimeProfileSource,
            SelectedRuntimeProfileSourceError,
        },
        session_events::{
            InMemorySessionEventStore, SessionCreationSpec, SessionDirectory,
            SessionDirectoryEntry, SessionDirectoryError, SessionEventStore,
            SessionInvocationDispatcher, SessionInvocationError, SessionInvocationReceipt,
            SessionInvocationRequest,
        },
        workflows::{
            authoring::WorkflowAuthoringNode,
            authoring_repository::SqliteWorkflowAuthoringRepository,
        },
    };
    use std::sync::Mutex;

    struct RuntimeSource;

    impl SelectedRuntimeProfileSource for RuntimeSource {
        fn selected_runtime_profile(
            &self,
        ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
            Ok(RuntimeProfileSnapshot {
                contract_version: 1,
                profile_ref: "native-codex:selected".into(),
                exposure: CapabilitySet::default(),
                locked: RuntimeSelections::default(),
            })
        }
    }

    #[derive(Default)]
    struct Directory {
        entries: Mutex<Vec<SessionDirectoryEntry>>,
    }

    impl SessionDirectory for Directory {
        fn find_exact(
            &self,
            _session: &ReferenceIdentity,
        ) -> Result<Option<SessionDirectoryEntry>, SessionDirectoryError> {
            Ok(None)
        }

        fn list_at_address(
            &self,
            _address: &crate::session_events::SessionLogicalAddress,
        ) -> Result<Vec<SessionDirectoryEntry>, SessionDirectoryError> {
            Ok(self.entries.lock().unwrap().clone())
        }

        fn create_session(
            &self,
            specification: SessionCreationSpec,
        ) -> Result<SessionDirectoryEntry, SessionDirectoryError> {
            let entry = SessionDirectoryEntry {
                session: ReferenceIdentity::new(
                    "orchestrator.agent_sessions",
                    "session",
                    "session-1",
                )
                .unwrap(),
                logical_address: Some(specification.logical_address),
                running: false,
                created_sequence: 1,
                last_addressed_sequence: None,
                created_by_event: Some(specification.created_by_event),
                created_by_session: specification.created_by_session,
            };
            self.entries.lock().unwrap().push(entry.clone());
            Ok(entry)
        }

        fn mark_addressed(
            &self,
            _session: &ReferenceIdentity,
            _event_group_id: &ReferenceIdentity,
        ) -> Result<u64, SessionDirectoryError> {
            Ok(1)
        }
    }

    #[derive(Default)]
    struct Dispatcher {
        requests: Mutex<Vec<SessionInvocationRequest>>,
    }

    impl SessionInvocationDispatcher for Dispatcher {
        fn dispatch(
            &self,
            request: SessionInvocationRequest,
        ) -> Result<SessionInvocationReceipt, SessionInvocationError> {
            self.requests.lock().unwrap().push(request);
            Ok(SessionInvocationReceipt {
                invocation: ReferenceIdentity::new(
                    "orchestrator.agent_sessions",
                    "invocation",
                    "invocation-1",
                )
                .unwrap(),
            })
        }
    }

    fn service(
        directory: Arc<Directory>,
        dispatcher: Arc<Dispatcher>,
        store: Arc<InMemorySessionEventStore>,
    ) -> (Arc<WorkflowAuthoringService>, WorkflowExecutionService) {
        let profiles = Arc::new(CapabilityProfileService::new(
            Arc::new(InMemoryCapabilityProfileRepository::default()),
            Arc::new(RuntimeSource),
        ));
        profiles
            .create(
                "capability-empty".into(),
                "Empty capabilities".into(),
                CapabilitySet::default(),
            )
            .unwrap();
        let authoring = Arc::new(WorkflowAuthoringService::new(
            Arc::new(SqliteWorkflowAuthoringRepository::in_memory()),
            profiles,
        ));
        let events = Arc::new(SessionEventApplication::new(
            directory.clone(),
            dispatcher,
            store,
        ));
        let execution = WorkflowExecutionService::new(
            authoring.clone(),
            events,
            Arc::new(WorkflowInstanceStore::in_memory()),
            directory,
            Arc::new(
                crate::agent_sessions::repository::SqliteAgentSessionRepository::open(":memory:")
                    .unwrap(),
            ),
        );
        (authoring, execution)
    }

    #[test]
    fn active_workflow_user_entry_creates_session_and_records_ordered_prompt_sources() {
        let directory = Arc::new(Directory::default());
        let dispatcher = Arc::new(Dispatcher::default());
        let store = Arc::new(InMemorySessionEventStore::default());
        let (authoring, execution) = service(directory, dispatcher.clone(), store.clone());
        let mut state = authoring.create("Review".into()).unwrap();
        state.draft.starting_node_id = Some("reviewer".into());
        state.draft.nodes.push(WorkflowAuthoringNode {
            node_id: "reviewer".into(),
            name: "Reviewer".into(),
            position_x: 0.0,
            position_y: 0.0,
            capability_profile_id: "capability-empty".into(),
            node_profile: NodeProfile {
                contract_version: 1,
                allowed_capabilities: CapabilitySet::default(),
                pinned_defaults: RuntimeSelections::default(),
            },
            initial_prompt: Some("Review carefully.".into()),
            agent_identity_id: None,
        });
        state = authoring.save_draft(state.draft).unwrap();
        authoring.activate(&state.draft.recipe_id).unwrap();
        let folder = tempfile::tempdir().unwrap();
        let instance = execution
            .create_instance(
                &state.draft.recipe_id,
                state.draft.revision,
                "Review instance".into(),
                super::super::instance_domain::ResolvedRepoBranchWorktreeTarget {
                    repository: super::super::instance_domain::WorkflowRepositoryTarget {
                        id: "repo".into(),
                        name: "Repo".into(),
                        git_common_directory: "git".into(),
                    },
                    branch: super::super::instance_domain::WorkflowBranchTarget {
                        id: "branch".into(),
                        name: "main".into(),
                    },
                    worktree: super::super::instance_domain::WorkflowWorktreeTarget {
                        id: "worktree".into(),
                        path: folder.path().to_string_lossy().into_owned(),
                    },
                },
            )
            .unwrap();

        let result = execution
            .dispatch_user_request(
                &state.draft.recipe_id,
                &instance.id,
                "Check this plan.".into(),
            )
            .unwrap();

        assert_eq!(result.deliveries.len(), 1);
        assert!(result.group.created_session.is_some());
        let request = &dispatcher.requests.lock().unwrap()[0];
        assert_eq!(request.prompt, "Check this plan.");
        assert_eq!(request.initial_prompt.as_deref(), Some("Review carefully."));
        assert!(store
            .event_group(&result.group.event_group_id)
            .unwrap()
            .is_some());
    }
}
