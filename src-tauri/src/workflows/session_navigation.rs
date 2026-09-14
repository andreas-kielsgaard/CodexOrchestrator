//! Workflow-owned interpretation of generic logical Session addresses.
use super::address_references::{
    workflow_node_address, WorkflowInstanceReference, WorkflowNodeReference,
};
use crate::agent_sessions::domain::AgentSessionId;
use crate::session_events::SessionLogicalAddress;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NavigationNode {
    pub(crate) id: String,
    pub(crate) name: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NavigationInstance {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) repository_id: String,
    pub(crate) nodes: Vec<NavigationNode>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowSessionOwner {
    pub(crate) session_id: AgentSessionId,
    pub(crate) instance_id: String,
    pub(crate) node_id: String,
    pub(crate) node_name: String,
}
pub(crate) fn project_session_owners(
    instances: &[NavigationInstance],
    addresses: &[(AgentSessionId, SessionLogicalAddress)],
) -> Result<Vec<WorkflowSessionOwner>, String> {
    let mut owners = Vec::new();
    for instance in instances {
        let instance_ref =
            WorkflowInstanceReference::new(&instance.id).map_err(|e| e.to_string())?;
        for node in &instance.nodes {
            let node_ref = WorkflowNodeReference::new(&node.id).map_err(|e| e.to_string())?;
            let expected = workflow_node_address(&instance_ref, &node_ref);
            for (session_id, _) in addresses.iter().filter(|(_, address)| address == &expected) {
                owners.push(WorkflowSessionOwner {
                    session_id: session_id.clone(),
                    instance_id: instance.id.clone(),
                    node_id: node.id.clone(),
                    node_name: node.name.clone(),
                });
            }
        }
    }
    Ok(owners)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ownership_requires_both_workflow_scope_and_node_subject() {
        let instances = vec![NavigationInstance {
            id: "instance".into(),
            name: "Build".into(),
            repository_id: "repo".into(),
            nodes: vec![NavigationNode {
                id: "node".into(),
                name: "Worker".into(),
            }],
        }];
        let address = workflow_node_address(
            &WorkflowInstanceReference::new("instance").unwrap(),
            &WorkflowNodeReference::new("node").unwrap(),
        );
        let mut foreign = address.clone();
        foreign.scope =
            crate::session_events::ReferenceIdentity::new("other", "instance", "instance").unwrap();
        let owners = project_session_owners(
            &instances,
            &[
                (AgentSessionId::new("owned").unwrap(), address),
                (AgentSessionId::new("foreign").unwrap(), foreign),
            ],
        )
        .unwrap();
        assert_eq!(owners.len(), 1);
        assert_eq!(owners[0].session_id.as_str(), "owned");
    }
}
