use super::{
    authoring_service::WorkflowAuthoringService,
    compiled_plan::WorkflowCompiledPlan,
    compiler::WorkflowCompiler,
    instance_domain::ResolvedRepoBranchWorktreeTarget,
    instances::{RecipeInstance, WorkflowInstanceStore},
};
use crate::{
    agent_sessions::ports::AgentSessionRepository, otp_api::*, otp_host::OtpRegistry,
    session_events::*,
};
use std::sync::Arc;

pub(crate) struct WorkflowExecutionService {
    pub(crate) authoring: Arc<WorkflowAuthoringService>,
    pub(crate) registry: Arc<OtpRegistry>,
    pub(crate) session_events: Arc<SessionEventApplication>,
    pub(crate) instances: Arc<WorkflowInstanceStore>,
    pub(crate) directory: Arc<dyn SessionDirectory>,
    pub(crate) sessions: Arc<dyn AgentSessionRepository>,
    pub(crate) session_control: Option<Arc<dyn crate::otp_host::session_control::SessionControl>>,
    pub(crate) record_observer: Option<Arc<dyn Fn(&str) + Send + Sync>>,
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
            registry: authoring.registry.clone(),
            authoring,
            session_events,
            instances,
            directory,
            sessions,
            record_observer: None,
            session_control: None,
        }
    }
    pub(crate) fn with_session_control(
        mut self,
        control: Arc<dyn crate::otp_host::session_control::SessionControl>,
    ) -> Self {
        self.session_control = Some(control);
        self
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
            return Err("The active Workflow changed; reload before creating an instance".into());
        }
        self.instances.create(name, recipe, target)
    }
    pub(crate) fn instance_sessions(
        &self,
        instance: &RecipeInstance,
    ) -> Result<Vec<SessionDirectoryEntry>, String> {
        let mut result = vec![];
        for node in &instance.recipe.nodes {
            result.extend(
                self.directory
                    .list_at_address(&node_address(&instance.id, &node.node_id)?)
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(result)
    }
    pub(crate) fn compile_instance(
        &self,
        instance_id: &str,
        node_id: Option<&str>,
    ) -> Result<WorkflowCompiledPlan, String> {
        let mut instance = self.instances.load(instance_id)?;
        if let Some(node) = node_id {
            instance.recipe.starting_node_id = Some(node.into());
        }
        WorkflowCompiler::compile(
            instance
                .recipe
                .runtime_compilation_input(instance_id, &instance.target.worktree.path)?,
            &self.registry,
        )
    }
    pub(crate) fn dispatch_user_request(
        &self,
        recipe_id: &str,
        instance_id: &str,
        text: String,
    ) -> Result<super::instances::WorkflowActionResult, String> {
        self.dispatch_node_user_request(recipe_id, instance_id, None, text, serde_json::json!({}))
    }
    pub(crate) fn dispatch_node_user_request(
        &self,
        recipe_id: &str,
        instance_id: &str,
        node_id: Option<&str>,
        text: String,
        data: serde_json::Value,
    ) -> Result<super::instances::WorkflowActionResult, String> {
        if !data.is_object() {
            return Err("Workflow entry data must be an object".into());
        }
        let instance = self.instances.load(instance_id)?;
        if instance.recipe.recipe_id != recipe_id {
            return Err("Instance belongs to another Workflow".into());
        }
        let plan = self.compile_instance(instance_id, node_id)?;
        let occurrence_id = format!("user-request-{}", uuid::Uuid::new_v4());
        let context = InvocationContext {
            instance_id: instance_id.into(),
            occurrence_id: occurrence_id.clone(),
            capability: plan.entry_action.clone(),
            source: None,
            connection_id: None,
            output_node_id: Some(plan.starting_node.identity().id().into()),
        };
        self.dispatch_otp_action(
            &instance,
            &context,
            None,
            serde_json::json!({"text":text,"data":data}),
            plan.entry_configuration.clone(),
            Ok(vec![
                ResolvedInput {
                    reference: occurrence_id.clone(),
                    value: serde_json::Value::String(text),
                },
                ResolvedInput {
                    reference: format!("{occurrence_id}:data"),
                    value: data,
                },
            ]),
        )
    }
}
pub(crate) fn node_address(instance: &str, node: &str) -> Result<SessionLogicalAddress, String> {
    use super::address_references::*;
    Ok(workflow_node_address(
        &WorkflowInstanceReference::new(instance).map_err(|e| e.to_string())?,
        &WorkflowNodeReference::new(node).map_err(|e| e.to_string())?,
    ))
}
