use super::domain::{
    WorkflowConnectionConfig, WorkflowDefinition, WorkflowElementRef, WorkflowHarnessConfig,
    WorkflowNativeQuery, WorkflowNodeConfig, WorkflowRole, WorkflowTypeSummary,
};
use std::sync::Arc;

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
}

pub(crate) struct WorkflowApplication {
    repository: Arc<dyn WorkflowRepository>,
}

impl WorkflowApplication {
    pub(crate) fn new(repository: Arc<dyn WorkflowRepository>) -> Self {
        Self { repository }
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
}
