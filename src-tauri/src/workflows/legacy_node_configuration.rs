use super::domain::WorkflowHarnessConfig;
use crate::agent_sessions::{
    domain::AgentRuntimeOptions,
    ports::{InitialPromptPrefix, RuntimeLaunchExtension},
};

const SUPPORTED_CODEX_MODELS: [&str; 2] = ["gpt-5.6-sol", "gpt-5.6-terra"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkflowNodeLaunchConfiguration {
    pub(crate) requested_options: AgentRuntimeOptions,
    pub(crate) extension: Option<RuntimeLaunchExtension>,
}

pub(crate) trait WorkflowNodeConfigurationSource: Send + Sync {
    fn resolve(
        &self,
        harness: &WorkflowHarnessConfig,
    ) -> Result<WorkflowNodeLaunchConfiguration, String>;
}

/// Compatibility implementation for the composite Workflow Harness configuration.
///
/// This preserves current launch behavior behind an injectable legacy producer. It resolves
/// before Session reuse is known, so it is not the final phase-aware node-profile boundary.
#[derive(Default)]
pub(crate) struct LegacyWorkflowNodeConfigurationSource;

impl WorkflowNodeConfigurationSource for LegacyWorkflowNodeConfigurationSource {
    fn resolve(
        &self,
        harness: &WorkflowHarnessConfig,
    ) -> Result<WorkflowNodeLaunchConfiguration, String> {
        if !harness.skills().is_empty() {
            return Err("Workflow launch does not support Harness skills yet.".to_string());
        }
        if !harness.hooks().is_empty() {
            return Err("Workflow launch does not support Harness hooks yet.".to_string());
        }
        validate_supported_codex_model(harness.default_model())?;
        let reasoning = harness.default_reasoning().unwrap_or_default();
        if harness.prompt_prefix().contains('\0') || harness.prompt_prefix().len() > 65_536 {
            return Err(
                "Workflow Harness instructions are invalid for direct prompt delivery.".to_string(),
            );
        }

        let requested_options = AgentRuntimeOptions {
            model: nonempty(harness.default_model()),
            sandbox: None,
        };
        let mut extension = RuntimeLaunchExtension::default();
        if !reasoning.is_empty() {
            extension.additional_args = vec![
                "-c".to_string(),
                format!("model_reasoning_effort=\"{reasoning}\""),
            ];
        }
        if let Some(instructions) = nonempty(harness.prompt_prefix()) {
            extension.initial_prompt_prefix = Some(InitialPromptPrefix {
                source: "workflow_recipe_node_instructions".to_string(),
                version: 1,
                content: instructions,
            });
        }

        Ok(WorkflowNodeLaunchConfiguration {
            requested_options,
            extension: (extension != RuntimeLaunchExtension::default()).then_some(extension),
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::conversation_harness_working_copy::{
        HarnessHookConfiguration, HarnessHookStatus, HarnessSkillConfiguration, HarnessSkillPolicy,
    };

    #[test]
    fn resolves_the_existing_model_reasoning_and_prompt_contract() {
        let harness = WorkflowHarnessConfig::test_definition(
            "Reviewer",
            "",
            "  Review the plan.  ",
            "gpt-5.6-sol",
            "high",
        );

        let resolved = LegacyWorkflowNodeConfigurationSource
            .resolve(&harness)
            .expect("legacy configuration");

        assert_eq!(
            resolved.requested_options.model.as_deref(),
            Some("gpt-5.6-sol")
        );
        assert_eq!(resolved.requested_options.sandbox, None);
        let extension = resolved.extension.expect("launch extension");
        assert_eq!(
            extension.additional_args,
            ["-c", "model_reasoning_effort=\"high\""]
        );
        assert_eq!(
            extension.initial_prompt_prefix,
            Some(InitialPromptPrefix {
                source: "workflow_recipe_node_instructions".to_string(),
                version: 1,
                content: "Review the plan.".to_string(),
            })
        );
    }

    #[test]
    fn returns_no_extension_or_requested_model_for_an_empty_harness() {
        let resolved = LegacyWorkflowNodeConfigurationSource
            .resolve(&WorkflowHarnessConfig::default())
            .expect("legacy configuration");

        assert_eq!(resolved.requested_options, AgentRuntimeOptions::default());
        assert_eq!(resolved.extension, None);
    }

    #[test]
    fn preserves_the_existing_unsupported_skills_and_hooks_errors() {
        let mut with_skill = WorkflowHarnessConfig::default();
        with_skill.0.skills.items.push(HarnessSkillConfiguration {
            name: "review".to_string(),
            path: "review".to_string(),
            purpose: String::new(),
            use_when: String::new(),
            policy: HarnessSkillPolicy::Available,
        });
        assert_eq!(
            LegacyWorkflowNodeConfigurationSource.resolve(&with_skill),
            Err("Workflow launch does not support Harness skills yet.".to_string())
        );

        let mut with_hook = WorkflowHarnessConfig::default();
        with_hook.0.hooks.push(HarnessHookConfiguration {
            name: "after-turn".to_string(),
            status: HarnessHookStatus::Exposed,
            detail: String::new(),
        });
        assert_eq!(
            LegacyWorkflowNodeConfigurationSource.resolve(&with_hook),
            Err("Workflow launch does not support Harness hooks yet.".to_string())
        );
    }

    #[test]
    fn preserves_model_and_prompt_validation_errors() {
        let unsupported =
            WorkflowHarnessConfig::test_definition("Reviewer", "", "", "gpt-unsupported", "");
        assert_eq!(
            LegacyWorkflowNodeConfigurationSource.resolve(&unsupported),
            Err(
                "Unsupported Codex model gpt-unsupported; use gpt-5.6-sol or gpt-5.6-terra."
                    .to_string()
            )
        );

        let mut invalid_prompt = WorkflowHarnessConfig::default();
        invalid_prompt.0.prompt_prefix.content = "invalid\0prompt".to_string();
        assert_eq!(
            LegacyWorkflowNodeConfigurationSource.resolve(&invalid_prompt),
            Err(
                "Workflow Harness instructions are invalid for direct prompt delivery.".to_string()
            )
        );
    }
}
