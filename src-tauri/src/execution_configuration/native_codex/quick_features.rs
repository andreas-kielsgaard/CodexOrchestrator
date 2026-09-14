use crate::execution_configuration::{
    QuickModel, QuickReasoningMode, QuickSkill, RuntimeQuickFeatures,
};
use crate::runtime::codex::app_server::environment::CodexEnvironment;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn project(profile_ref: String, native: &CodexEnvironment) -> RuntimeQuickFeatures {
    let mut result = RuntimeQuickFeatures {
        profile_ref,
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
    // A textual invocation is supported by Codex. Do not offer ambiguous names as exact choices.
    let mut skills: BTreeMap<String, BTreeMap<String, &Value>> = BTreeMap::new();
    for group in array(&native.skills["data"]) {
        if array(&group["errors"]).next().is_some() {
            result
                .limitations
                .push("Some skills could not be discovered.".into());
        }
        for skill in array(&group["skills"]).filter(|skill| skill["enabled"] != false) {
            let (Some(name), Some(path)) = (skill["name"].as_str(), skill["path"].as_str()) else {
                continue;
            };
            skills
                .entry(name.into())
                .or_default()
                .insert(path.into(), skill);
        }
    }
    for (name, paths) in skills {
        if paths.len() != 1 {
            result.limitations.push(format!(
                "Skill '{name}' has multiple sources and cannot be selected by name."
            ));
            continue;
        }
        let (path, skill) = paths.into_iter().next().unwrap();
        result.skills.push(QuickSkill {
            id: path,
            invocation_text: format!("${name}"),
            name,
            description: skill["interface"]["shortDescription"]
                .as_str()
                .or(skill["description"].as_str())
                .unwrap_or("")
                .into(),
        });
    }
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
        let projected = project("profile".into(), &native);
        assert_eq!(projected.models[0].reasoning_modes[0].id, "light");
        assert_eq!(projected.models[1].reasoning_modes[0].id, "deep");
        assert_eq!(projected.defaults.model.as_deref(), Some("model-a"));
        assert_eq!(projected.skills.len(), 1);
        assert_eq!(projected.skills[0].invocation_text, "$review");
        assert_eq!(projected.limitations.len(), 2);
    }
}
