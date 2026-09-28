use super::{
    authoring::{
        WorkflowAuthoringNode, WorkflowAuthoringWorkspace, WorkflowRecipeDraft,
        WorkflowRecipeState, WorkflowRecipeSummary, WORKFLOW_RECIPE_CONTRACT_VERSION,
    },
    authoring_repository::WorkflowAuthoringRepository,
    compiler::WorkflowCompiler,
};
use crate::{
    execution_configuration::{CapabilityProfile, CapabilityProfileService},
    otp_host::OtpRegistry,
    workflows::compiled_plan::WorkflowCompiledPlan,
};
use std::{collections::BTreeMap, sync::Arc};
use uuid::Uuid;

pub(crate) struct WorkflowAuthoringService {
    repository: Arc<dyn WorkflowAuthoringRepository>,
    capability_profiles: Arc<CapabilityProfileService>,
    pub(crate) registry: Arc<OtpRegistry>,
}

impl WorkflowAuthoringService {
    pub(crate) fn new(
        repository: Arc<dyn WorkflowAuthoringRepository>,
        capability_profiles: Arc<CapabilityProfileService>,
        registry: Arc<OtpRegistry>,
    ) -> Self {
        Self {
            repository,
            capability_profiles,
            registry,
        }
    }

    pub(crate) fn list(&self) -> Result<Vec<WorkflowRecipeSummary>, String> {
        self.repository.list()
    }

    pub(crate) fn open_workspace(
        &self,
        preferred_recipe_id: Option<&str>,
    ) -> Result<WorkflowAuthoringWorkspace, String> {
        let summaries = self.list()?;
        let selected_id = preferred_recipe_id
            .filter(|id| summaries.iter().any(|recipe| recipe.recipe_id == *id))
            .or_else(|| summaries.first().map(|recipe| recipe.recipe_id.as_str()));
        let selected = selected_id.map(|id| self.load(id)).transpose()?;
        Ok(WorkflowAuthoringWorkspace {
            summaries,
            selected,
        })
    }

    pub(crate) fn load(&self, recipe_id: &str) -> Result<WorkflowRecipeState, String> {
        self.repository
            .load(recipe_id)?
            .ok_or_else(|| format!("Workflow recipe `{recipe_id}` does not exist"))
    }

    pub(crate) fn create(&self) -> Result<WorkflowRecipeState, String> {
        let draft = WorkflowRecipeDraft {
            contract_version: WORKFLOW_RECIPE_CONTRACT_VERSION,
            recipe_id: format!("workflow-recipe-{}", Uuid::new_v4()),
            name: String::new(),
            revision: 1,
            entry_node_ids: Vec::new(),
            nodes: Vec::new(),
            connections: Vec::new(),
        };
        draft.validate_storable()?;
        self.repository.create(&draft)
    }

    pub(crate) fn save_draft(
        &self,
        mut submitted: WorkflowRecipeDraft,
    ) -> Result<WorkflowRecipeState, String> {
        let current = self.load(&submitted.recipe_id)?;
        if submitted.revision != current.draft.revision {
            return Err(format!(
                "Workflow recipe `{}` is no longer at draft revision {}",
                submitted.recipe_id, submitted.revision
            ));
        }
        submitted.revision = current
            .draft
            .revision
            .checked_add(1)
            .ok_or_else(|| "Workflow recipe revision cannot be incremented".to_string())?;
        submitted.validate_storable()?;
        self.repository.save_draft(&submitted)
    }

    pub(crate) fn copy_node_configuration(
        &self,
        recipe_id: &str,
        expected_revision: u64,
        source_node_id: &str,
        destination_node_id: &str,
    ) -> Result<WorkflowRecipeState, String> {
        let mut state = self.load(recipe_id)?;
        if state.draft.revision != expected_revision {
            return Err(format!(
                "Workflow recipe `{recipe_id}` is no longer at draft revision {expected_revision}"
            ));
        }
        let source = state
            .draft
            .nodes
            .iter()
            .find(|node| node.node_id == source_node_id)
            .cloned()
            .ok_or_else(|| format!("Workflow node `{source_node_id}` does not exist"))?;
        let destination = state
            .draft
            .nodes
            .iter_mut()
            .find(|node| node.node_id == destination_node_id)
            .ok_or_else(|| format!("Workflow node `{destination_node_id}` does not exist"))?;
        copy_configurable_node_state(&source, destination);
        self.save_draft(state.draft)
    }

    pub(crate) fn activate(&self, recipe_id: &str) -> Result<WorkflowRecipeState, String> {
        let state = self.load(recipe_id)?;
        self.activate_revision(recipe_id, state.draft.revision)
    }

    pub(crate) fn activate_revision(
        &self,
        recipe_id: &str,
        expected_revision: u64,
    ) -> Result<WorkflowRecipeState, String> {
        let state = self.load(recipe_id)?;
        if state.draft.revision != expected_revision {
            return Err("Saved Workflow changed. Reload before activating.".into());
        }
        let validation_instance = format!("activation-validation-{}", state.draft.recipe_id);
        self.compile(&state.draft, &validation_instance)?;
        self.repository
            .activate_revision(recipe_id, expected_revision)
    }

    pub(crate) fn compile_active_for_instance(
        &self,
        recipe_id: &str,
        instance_id: &str,
    ) -> Result<WorkflowCompiledPlan, String> {
        let state = self.load(recipe_id)?;
        let active = state
            .active
            .ok_or_else(|| format!("Workflow recipe `{recipe_id}` is not active"))?;
        self.compile(&active, instance_id)
    }

    fn compile(
        &self,
        recipe: &WorkflowRecipeDraft,
        instance_id: &str,
    ) -> Result<WorkflowCompiledPlan, String> {
        let profiles = self.load_capability_profiles(recipe)?;
        let input = recipe.compilation_input(instance_id, &profiles)?;
        WorkflowCompiler::compile(input, &self.registry).map_err(|error| error.to_string())
    }

    fn load_capability_profiles(
        &self,
        recipe: &WorkflowRecipeDraft,
    ) -> Result<BTreeMap<String, CapabilityProfile>, String> {
        let mut profiles = BTreeMap::new();
        for node in &recipe.nodes {
            if profiles.contains_key(&node.capability_profile_id) {
                continue;
            }
            let profile = self
                .capability_profiles
                .read(&node.capability_profile_id)
                .map_err(|error| error.to_string())?;
            profiles.insert(node.capability_profile_id.clone(), profile);
        }
        Ok(profiles)
    }
}

fn copy_configurable_node_state(
    source: &WorkflowAuthoringNode,
    destination: &mut WorkflowAuthoringNode,
) {
    destination.capability_profile_id = source.capability_profile_id.clone();
    destination.node_profile = source.node_profile.clone();
    destination.initial_prompt = source.initial_prompt.clone();
    destination.agent_identity_id = source.agent_identity_id.clone();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_configuration::{
        CapabilitySet, InMemoryCapabilityProfileRepository, NodeProfile,
        ProviderConfigurationSource, ProviderConfigurationSourceError, RuntimeProfileSnapshot,
        RuntimeSelections, SandboxMode,
    };

    struct FixedRuntimeSource(RuntimeProfileSnapshot);

    impl ProviderConfigurationSource for FixedRuntimeSource {
        fn profile_for_configuration(
            &self,
            _reference: &str,
            _cwd: Option<&str>,
        ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError> {
            Ok(self.0.clone())
        }
    }
    use crate::workflows::{
        authoring::WorkflowAuthoringNode, authoring_repository::SqliteWorkflowAuthoringRepository,
    };

    fn runtime_profile() -> RuntimeProfileSnapshot {
        RuntimeProfileSnapshot {
            contract_version: 1,
            configuration: orchid_engine::contracts::ProviderConfigurationRef::new(
                "codex",
                "configured-runtime",
            ),
            exposure: capabilities(),
            locked: RuntimeSelections {
                sandbox_mode: Some(SandboxMode::WorkspaceWrite),
                ..RuntimeSelections::default()
            },
            provider_options: None,
        }
    }

    fn capabilities() -> CapabilitySet {
        CapabilitySet {
            models: ["codex-a".into()].into_iter().collect(),
            reasoning_modes: ["high".into()].into_iter().collect(),
            sandbox_modes: [SandboxMode::WorkspaceWrite].into_iter().collect(),
            ..CapabilitySet::default()
        }
    }

    fn service() -> WorkflowAuthoringService {
        let profiles = Arc::new(
            CapabilityProfileService::new(Arc::new(InMemoryCapabilityProfileRepository::default()))
                .with_configuration_source(Arc::new(FixedRuntimeSource(runtime_profile()))),
        );
        profiles
            .create(
                "capability-default".into(),
                "Default capabilities".into(),
                capabilities(),
            )
            .unwrap();
        WorkflowAuthoringService::new(
            Arc::new(SqliteWorkflowAuthoringRepository::in_memory()),
            profiles,
            crate::otp_host::OtpRegistry::import(&["workflow"]).unwrap(),
        )
    }

    fn node(id: &str, name: &str) -> WorkflowAuthoringNode {
        WorkflowAuthoringNode {
            node_id: id.into(),
            name: name.into(),
            position_x: 0.0,
            position_y: 0.0,
            capability_profile_id: "capability-default".into(),
            node_profile: NodeProfile {
                contract_version: 1,
                allowed_capabilities: capabilities(),
                pinned_defaults: RuntimeSelections {
                    model: Some("codex-a".into()),
                    reasoning_mode: Some("high".into()),
                    sandbox_mode: Some(SandboxMode::WorkspaceWrite),
                },
            },
            initial_prompt: Some(format!("You are {name}.")),
            agent_identity_id: None,
        }
    }

    #[test]
    fn service_saves_activates_and_compiles_recipe() {
        let service = service();
        let mut state = service.create().unwrap();
        state.draft.name = "Review".into();
        state.draft.entry_node_ids = vec!["planner".into()];
        state.draft.nodes.push(node("planner", "Planner"));
        state = service.save_draft(state.draft).unwrap();
        let active = service.activate(&state.draft.recipe_id).unwrap();

        let definitions = service
            .compile_active_for_instance(&active.draft.recipe_id, "instance-1")
            .unwrap();

        assert_eq!(definitions.nodes.len(), 1);
        assert_eq!(active.active.unwrap().revision, 2);
    }

    #[test]
    fn workspace_bootstrap_returns_summaries_and_the_preferred_recipe_together() {
        let service = service();
        let first = service.create().unwrap();
        let second = service.create().unwrap();

        let workspace = service
            .open_workspace(Some(&second.draft.recipe_id))
            .unwrap();

        assert_eq!(workspace.summaries.len(), 2);
        assert_eq!(
            workspace.selected.unwrap().draft.recipe_id,
            second.draft.recipe_id
        );
        assert!(workspace
            .summaries
            .iter()
            .any(|summary| summary.recipe_id == first.draft.recipe_id));
    }

    #[test]
    fn copy_node_configuration_preserves_destination_identity_and_position() {
        let service = service();
        let mut state = service.create().unwrap();
        state.draft.name = "Review".into();
        let mut source = node("planner", "Planner");
        source.agent_identity_id = Some("identity-avery".into());
        source.position_x = 50.0;
        let mut destination = node("reviewer", "Reviewer");
        destination.position_x = 250.0;
        destination.initial_prompt = Some("Old".into());
        state.draft.nodes = vec![source, destination];
        state = service.save_draft(state.draft).unwrap();

        let copied = service
            .copy_node_configuration(
                &state.draft.recipe_id,
                state.draft.revision,
                "planner",
                "reviewer",
            )
            .unwrap();
        let reviewer = copied
            .draft
            .nodes
            .iter()
            .find(|node| node.node_id == "reviewer")
            .unwrap();

        assert_eq!(reviewer.name, "Reviewer");
        assert_eq!(reviewer.position_x, 250.0);
        assert_eq!(reviewer.initial_prompt.as_deref(), Some("You are Planner."));
        assert_eq!(
            reviewer.agent_identity_id.as_deref(),
            Some("identity-avery")
        );
    }

    #[test]
    fn activation_rejects_structurally_incomplete_recipe() {
        let service = service();
        let state = service.create().unwrap();

        assert!(service.activate(&state.draft.recipe_id).is_err());
    }
}
