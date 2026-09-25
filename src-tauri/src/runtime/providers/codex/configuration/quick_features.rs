use crate::execution_configuration::{QuickModel, QuickReasoningMode, RuntimeQuickFeatures};
use crate::runtime::providers::codex::app_server::environment::CodexEnvironment;
use orchid_engine::contracts::ProviderConfigurationRef;
use serde_json::Value;

pub(super) fn project(
    configuration: ProviderConfigurationRef,
    native: &CodexEnvironment,
) -> RuntimeQuickFeatures {
    let mut result = RuntimeQuickFeatures {
        configuration: Some(configuration),
        ..Default::default()
    };
    let config = &native.config["config"];
    result.defaults.model = config["model"].as_str().map(String::from);
    result.defaults.reasoning_mode = config["model_reasoning_effort"].as_str().map(String::from);
    for model in array(&native.models).filter(|model| model["hidden"] != true) {
        let Some(id) = model["model"].as_str().or(model["id"].as_str()) else {
            continue;
        };
        if result.defaults.model.is_none() && model["isDefault"] == true {
            result.defaults.model = Some(id.into());
        }
        result.models.push(QuickModel {
            id: id.into(),
            label: model["displayName"].as_str().unwrap_or(id).into(),
            description: model["description"].as_str().unwrap_or("").into(),
            default_reasoning_mode: model["defaultReasoningEffort"].as_str().map(String::from),
            reasoning_modes: array(&model["supportedReasoningEfforts"])
                .filter_map(|mode| {
                    Some(QuickReasoningMode {
                        id: mode["reasoningEffort"].as_str()?.into(),
                        description: mode["description"].as_str().unwrap_or("").into(),
                    })
                })
                .collect(),
        });
    }
    result.append_native_skills(&crate::runtime::providers::codex::app_server::skills::project(
        &native.skills,
    ));
    result
}

fn array(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn retains_model_specific_efforts_and_only_unambiguous_enabled_skill_mentions() {
        let native = CodexEnvironment {
            models: json!([
                {"id":"a", "model":"model-a", "isDefault":true, "supportedReasoningEfforts":[{"reasoningEffort":"light"}]},
                {"id":"b", "supportedReasoningEfforts":[{"reasoningEffort":"deep"}]}
            ]),
            skills: json!({"data":[{"skills":[
                {"name":"review", "path":"/one/SKILL.md", "enabled":true},
                {"name":"hidden", "path":"/hidden/SKILL.md", "enabled":false},
                {"name":"duplicate", "path":"/a/SKILL.md"},
                {"name":"duplicate", "path":"/b/SKILL.md"}
            ],"errors":[{"message":"bad skill"}]}]}),
            config: json!({"config":{}}),
            requirements: json!({}),
        };
        let projected = project(ProviderConfigurationRef::new("codex", "profile"), &native);
        assert_eq!(projected.models[0].reasoning_modes[0].id, "light");
        assert_eq!(projected.models[1].reasoning_modes[0].id, "deep");
        assert_eq!(projected.defaults.model.as_deref(), Some("model-a"));
        assert_eq!(projected.skills.len(), 1);
        assert_eq!(projected.skills[0].invocation_text, "$review");
        assert_eq!(projected.limitations.len(), 2);
    }
}
