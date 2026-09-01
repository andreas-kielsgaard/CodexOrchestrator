use crate::session_events::{ReferenceIdentity, SessionEventDomainError, SessionLogicalAddress};
use serde::{Deserialize, Serialize};

const WORKFLOW_REFERENCE_NAMESPACE: &str = "workflow";

macro_rules! workflow_reference {
    ($name:ident, $kind:literal) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub(crate) struct $name(ReferenceIdentity);

        impl $name {
            pub(crate) fn new(id: impl Into<String>) -> Result<Self, SessionEventDomainError> {
                ReferenceIdentity::new(WORKFLOW_REFERENCE_NAMESPACE, $kind, id).map(Self)
            }

            pub(crate) fn identity(&self) -> &ReferenceIdentity {
                &self.0
            }

            pub(crate) fn into_identity(self) -> ReferenceIdentity {
                self.0
            }
        }
    };
}

workflow_reference!(WorkflowInstanceReference, "instance");
workflow_reference!(WorkflowRecipeReference, "recipe");
workflow_reference!(WorkflowNodeReference, "node");
workflow_reference!(WorkflowConnectionReference, "connection");
workflow_reference!(WorkflowEventDefinitionReference, "event_definition");

impl WorkflowEventDefinitionReference {
    pub(crate) fn for_node(
        recipe: &WorkflowRecipeReference,
        node: &WorkflowNodeReference,
    ) -> Result<Self, SessionEventDomainError> {
        Self::new(scoped_event_id(
            "user_entry",
            recipe.identity().id(),
            node.identity().id(),
        ))
    }

    pub(crate) fn for_connection(
        recipe: &WorkflowRecipeReference,
        connection: &WorkflowConnectionReference,
    ) -> Result<Self, SessionEventDomainError> {
        Self::new(scoped_event_id(
            "connection",
            recipe.identity().id(),
            connection.identity().id(),
        ))
    }
}

fn scoped_event_id(kind: &str, recipe_id: &str, element_id: &str) -> String {
    format!(
        "{kind}:{}:{recipe_id}:{}:{element_id}",
        recipe_id.len(),
        element_id.len()
    )
}

pub(crate) fn workflow_node_address(
    instance: &WorkflowInstanceReference,
    node: &WorkflowNodeReference,
) -> SessionLogicalAddress {
    SessionLogicalAddress::new(instance.identity().clone(), node.identity().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_references_preserve_identity_kind_and_opaque_id() {
        let instance = WorkflowInstanceReference::new("instance-7").unwrap();
        let recipe = WorkflowRecipeReference::new("recipe-4").unwrap();
        let node = WorkflowNodeReference::new("review-node").unwrap();
        let connection = WorkflowConnectionReference::new("review-handoff").unwrap();
        let definition =
            WorkflowEventDefinitionReference::for_connection(&recipe, &connection).unwrap();

        assert_eq!(instance.identity().namespace(), "workflow");
        assert_eq!(instance.identity().kind(), "instance");
        assert_eq!(instance.identity().id(), "instance-7");
        assert_eq!(recipe.identity().kind(), "recipe");
        assert_eq!(node.identity().kind(), "node");
        assert_eq!(connection.identity().kind(), "connection");
        assert_eq!(definition.identity().kind(), "event_definition");
        assert_eq!(
            definition.identity().id(),
            "connection:8:recipe-4:14:review-handoff"
        );
    }

    #[test]
    fn workflow_node_address_uses_instance_scope_and_node_subject() {
        let instance = WorkflowInstanceReference::new("instance-7").unwrap();
        let node = WorkflowNodeReference::new("review-node").unwrap();

        let address = workflow_node_address(&instance, &node);

        assert_eq!(address.scope, instance.into_identity());
        assert_eq!(address.subject, node.into_identity());
    }
}
