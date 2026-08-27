use super::{configuration as canonical, domain::HarnessId};
use crate::{
    orchestration::conversation_harness_working_copy as legacy,
    workflows::domain::WorkflowHarnessConfig,
};
use sha2::{Digest, Sha256};

pub(crate) struct CanonicalWorkflowHarness {
    pub(crate) id: HarnessId,
    pub(crate) name: String,
    pub(crate) configuration: canonical::HarnessConfiguration,
}

pub(crate) fn adapt_workflow_harness(
    workflow_type_id: &str,
    node_id: &str,
    harness: &WorkflowHarnessConfig,
) -> Result<CanonicalWorkflowHarness, String> {
    Ok(CanonicalWorkflowHarness {
        id: stable_workflow_harness_id(workflow_type_id, node_id)?,
        name: harness.name().to_string(),
        configuration: convert_configuration(&harness.0),
    })
}

/// Workflow identity, not display labels or the transitional machine key, owns this opaque ID.
pub(crate) fn stable_workflow_harness_id(
    workflow_type_id: &str,
    node_id: &str,
) -> Result<HarnessId, String> {
    let workflow_type_id = required_identity(workflow_type_id, "Workflow type ID")?;
    let node_id = required_identity(node_id, "Workflow node ID")?;
    let mut digest = Sha256::new();
    digest.update(b"workflow-harness-node/v1\0");
    digest.update((workflow_type_id.len() as u64).to_be_bytes());
    digest.update(workflow_type_id.as_bytes());
    digest.update((node_id.len() as u64).to_be_bytes());
    digest.update(node_id.as_bytes());
    HarnessId::new(format!("harness-workflow-{:x}", digest.finalize()))
        .map_err(|error| error.to_string())
}

pub(crate) fn convert_configuration(
    source: &legacy::HarnessEffectiveConfiguration,
) -> canonical::HarnessConfiguration {
    canonical::HarnessConfiguration {
        identity_assignment: match &source.identity.permitted_agent_names {
            Some(identity_ids) => canonical::HarnessIdentityAssignmentPolicy::AllowList {
                identity_ids: identity_ids.clone(),
            },
            None => canonical::HarnessIdentityAssignmentPolicy::Unrestricted,
        },
        prompt_prefix: canonical::HarnessPromptPrefixConfiguration {
            content: source.prompt_prefix.content.clone(),
            initial_delivery: map_initial_delivery(source.prompt_prefix.initial_delivery),
            context_compression_delivery: map_compression_delivery(
                source.prompt_prefix.context_compression_delivery,
            ),
        },
        skills: canonical::HarnessSkillsConfiguration {
            available_discovery_policy: map_discovery(source.skills.available_discovery_policy),
            items: source
                .skills
                .items
                .iter()
                .map(|skill| canonical::HarnessSkillConfiguration {
                    name: skill.name.clone(),
                    path: skill.path.clone(),
                    purpose: skill.purpose.clone(),
                    use_when: skill.use_when.clone(),
                    policy: map_skill_policy(skill.policy),
                })
                .collect(),
        },
        tools: canonical::HarnessToolsConfiguration {
            available_discovery_policy: map_discovery(source.tools.available_discovery_policy),
            items: source
                .tools
                .items
                .iter()
                .map(|tool| canonical::HarnessToolConfiguration {
                    name: tool.name.clone(),
                    policy: map_tool_policy(tool.policy),
                })
                .collect(),
            schema_boundary: source.tools.schema_boundary.clone(),
            mcp_servers: source
                .tools
                .mcp_servers
                .iter()
                .map(|server| canonical::HarnessMcpServerExposure {
                    server_name: server.server_name.clone(),
                    access: match &server.access {
                        legacy::HarnessMcpServerAccess::EntireServer => {
                            canonical::HarnessMcpServerAccess::EntireServer
                        }
                        legacy::HarnessMcpServerAccess::SelectedTools { tool_names } => {
                            canonical::HarnessMcpServerAccess::SelectedTools {
                                tool_names: tool_names.clone(),
                            }
                        }
                    },
                })
                .collect(),
        },
        runtime: canonical::HarnessRuntimeConfiguration {
            preferred_model: source.runtime.default_model.as_ref().map(|model_id| {
                canonical::HarnessModelPreference {
                    model_id: model_id.clone(),
                    reasoning: source.runtime.default_reasoning.map(map_reasoning),
                }
            }),
            sandbox: map_sandbox(source.runtime.sandbox),
            approval_policy: map_approval(source.runtime.approval_policy),
            authority_summary: source.runtime.authority_summary.clone(),
        },
        hooks: source
            .hooks
            .iter()
            .map(|hook| canonical::HarnessHookConfiguration {
                name: hook.name.clone(),
                status: map_hook_status(hook.status),
                detail: hook.detail.clone(),
            })
            .collect(),
        update_policy: match &source.update_policy {
            legacy::HarnessUpdatePolicy::Configured {
                delivery,
                avoid_duplicate_guidance,
                notify_removed_items,
                prompt_reconstruction,
            } => canonical::HarnessUpdatePolicy::Configured {
                delivery: map_update_delivery(*delivery),
                avoid_duplicate_guidance: *avoid_duplicate_guidance,
                notify_removed_items: *notify_removed_items,
                prompt_reconstruction: map_prompt_reconstruction(*prompt_reconstruction),
            },
            legacy::HarnessUpdatePolicy::NotConfigured { reason } => {
                canonical::HarnessUpdatePolicy::NotConfigured {
                    reason: reason.clone(),
                }
            }
        },
    }
}

fn required_identity<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label} must not be empty."))
    } else {
        Ok(value)
    }
}

fn map_initial_delivery(
    value: legacy::HarnessInitialDelivery,
) -> canonical::HarnessInitialDelivery {
    match value {
        legacy::HarnessInitialDelivery::Prepend => canonical::HarnessInitialDelivery::Prepend,
    }
}

fn map_compression_delivery(
    value: legacy::HarnessContextCompressionDelivery,
) -> canonical::HarnessContextCompressionDelivery {
    match value {
        legacy::HarnessContextCompressionDelivery::Deferred => {
            canonical::HarnessContextCompressionDelivery::Deferred
        }
    }
}

fn map_discovery(value: legacy::HarnessDiscoveryPolicy) -> canonical::HarnessDiscoveryPolicy {
    match value {
        legacy::HarnessDiscoveryPolicy::Whitelist => canonical::HarnessDiscoveryPolicy::Whitelist,
        legacy::HarnessDiscoveryPolicy::Blacklist => canonical::HarnessDiscoveryPolicy::Blacklist,
    }
}

fn map_skill_policy(value: legacy::HarnessSkillPolicy) -> canonical::HarnessSkillPolicy {
    match value {
        legacy::HarnessSkillPolicy::AlwaysApplicable => {
            canonical::HarnessSkillPolicy::AlwaysApplicable
        }
        legacy::HarnessSkillPolicy::InitialIngestion => {
            canonical::HarnessSkillPolicy::InitialIngestion
        }
        legacy::HarnessSkillPolicy::Available => canonical::HarnessSkillPolicy::Available,
    }
}

fn map_tool_policy(value: legacy::HarnessToolPolicy) -> canonical::HarnessToolPolicy {
    match value {
        legacy::HarnessToolPolicy::EveryInvocation => canonical::HarnessToolPolicy::EveryInvocation,
        legacy::HarnessToolPolicy::InitialInvocation => {
            canonical::HarnessToolPolicy::InitialInvocation
        }
        legacy::HarnessToolPolicy::Available => canonical::HarnessToolPolicy::Available,
    }
}

fn map_reasoning(value: legacy::HarnessReasoningLevel) -> canonical::HarnessReasoningLevel {
    match value {
        legacy::HarnessReasoningLevel::Low => canonical::HarnessReasoningLevel::Low,
        legacy::HarnessReasoningLevel::Medium => canonical::HarnessReasoningLevel::Medium,
        legacy::HarnessReasoningLevel::High => canonical::HarnessReasoningLevel::High,
        legacy::HarnessReasoningLevel::Xhigh => canonical::HarnessReasoningLevel::Xhigh,
    }
}

fn map_sandbox(value: legacy::HarnessSandbox) -> canonical::HarnessSandbox {
    match value {
        legacy::HarnessSandbox::ReadOnly => canonical::HarnessSandbox::ReadOnly,
        legacy::HarnessSandbox::WorkspaceWrite => canonical::HarnessSandbox::WorkspaceWrite,
        legacy::HarnessSandbox::DangerFullAccess => canonical::HarnessSandbox::DangerFullAccess,
    }
}

fn map_approval(value: legacy::HarnessApprovalPolicy) -> canonical::HarnessApprovalPolicy {
    match value {
        legacy::HarnessApprovalPolicy::Never => canonical::HarnessApprovalPolicy::Never,
    }
}

fn map_hook_status(value: legacy::HarnessHookStatus) -> canonical::HarnessHookStatus {
    match value {
        legacy::HarnessHookStatus::Exposed => canonical::HarnessHookStatus::Exposed,
        legacy::HarnessHookStatus::Proposed => canonical::HarnessHookStatus::Proposed,
        legacy::HarnessHookStatus::NotConnected => canonical::HarnessHookStatus::NotConnected,
    }
}

fn map_update_delivery(value: legacy::HarnessUpdateDelivery) -> canonical::HarnessUpdateDelivery {
    match value {
        legacy::HarnessUpdateDelivery::NextPrompt => canonical::HarnessUpdateDelivery::NextPrompt,
    }
}

fn map_prompt_reconstruction(
    value: legacy::HarnessPromptReconstruction,
) -> canonical::HarnessPromptReconstruction {
    match value {
        legacy::HarnessPromptReconstruction::Deferred => {
            canonical::HarnessPromptReconstruction::Deferred
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_conversion_keeps_behavior_but_drops_transitional_ownership_fields() {
        let mut workflow = WorkflowHarnessConfig::test_definition(
            "Security reviewer",
            "Review-only authority.",
            "Inspect the change.",
            "gpt-5.6-sol",
            "high",
        );
        workflow.0.identity.machine_key = "security_reviewer".into();
        workflow.0.identity.permitted_agent_names =
            Some(vec!["identity-avery".into(), "identity-riley".into()]);
        workflow.0.skills.items = vec![legacy::HarnessSkillConfiguration {
            name: "security-review".into(),
            path: "product/skills/security-review".into(),
            purpose: "Review trust boundaries.".into(),
            use_when: "A security review is requested.".into(),
            policy: legacy::HarnessSkillPolicy::AlwaysApplicable,
        }];
        workflow.0.tools.schema_boundary = "Review tools only.".into();
        workflow.0.tools.items = vec![legacy::HarnessToolConfiguration {
            name: "submit_review".into(),
            policy: legacy::HarnessToolPolicy::Available,
        }];
        workflow.0.tools.mcp_servers = vec![legacy::HarnessMcpServerExposure {
            server_name: "workflow".into(),
            access: legacy::HarnessMcpServerAccess::SelectedTools {
                tool_names: vec!["handoff".into()],
            },
        }];
        workflow.0.hooks = vec![legacy::HarnessHookConfiguration {
            name: "turn_finished".into(),
            status: legacy::HarnessHookStatus::Exposed,
            detail: "Routes the completed review.".into(),
        }];
        workflow.0.update_policy = legacy::HarnessUpdatePolicy::Configured {
            delivery: legacy::HarnessUpdateDelivery::NextPrompt,
            avoid_duplicate_guidance: true,
            notify_removed_items: true,
            prompt_reconstruction: legacy::HarnessPromptReconstruction::Deferred,
        };

        let adapted = adapt_workflow_harness("workflow-type-1", "review-node", &workflow).unwrap();
        let encoded = serde_json::to_value(&adapted.configuration).unwrap();

        assert_eq!(
            adapted.configuration.identity_assignment,
            canonical::HarnessIdentityAssignmentPolicy::AllowList {
                identity_ids: vec!["identity-avery".into(), "identity-riley".into()]
            }
        );
        assert_eq!(adapted.configuration.skills.items.len(), 1);
        assert_eq!(adapted.configuration.tools.mcp_servers.len(), 1);
        assert_eq!(
            adapted
                .configuration
                .runtime
                .preferred_model
                .as_ref()
                .map(|preference| preference.model_id.as_str()),
            Some("gpt-5.6-sol")
        );
        assert!(encoded.get("identity").is_none());
        assert!(encoded["runtime"].get("provider").is_none());
        assert!(encoded["runtime"].get("models").is_none());
        assert!(encoded["runtime"].get("modelPolicyMode").is_none());
        assert!(!encoded.to_string().contains("security_reviewer"));
    }

    #[test]
    fn workflow_harness_id_depends_only_on_stable_workflow_and_node_identity() {
        let first = stable_workflow_harness_id("workflow-type-1", "review-node").unwrap();
        let repeated = stable_workflow_harness_id("workflow-type-1", "review-node").unwrap();
        let other_node = stable_workflow_harness_id("workflow-type-1", "other-node").unwrap();

        assert_eq!(first, repeated);
        assert_ne!(first, other_node);
        assert!(first.as_str().starts_with("harness-workflow-"));
        assert_eq!(first.as_str().len(), "harness-workflow-".len() + 64);
    }
}
