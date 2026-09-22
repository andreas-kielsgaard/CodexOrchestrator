use crate::execution_configuration::{
    QuickModel, QuickReasoningMode, QuickSkill, RuntimeQuickFeatures,
};
use crate::runtime::codex::app_server::environment::CodexEnvironment;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

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
    append_codex_skills(
        &mut result,
        &crate::runtime::codex::app_server::skills::project(&native.skills),
    );
    result
}

pub(super) fn append_codex_skills(
    result: &mut RuntimeQuickFeatures,
    catalogue: &crate::runtime::codex::app_server::skills::CodexSkillCatalogue,
) {
    result
        .limitations
        .extend(catalogue.limitations.iter().cloned());
    let mut names =
        BTreeMap::<&str, Vec<&crate::runtime::codex::app_server::skills::CodexSkill>>::new();
    for skill in catalogue.skills.iter().filter(|skill| skill.enabled) {
        names.entry(&skill.name).or_default().push(skill);
    }
    for (name, entries) in names {
        if entries.len() != 1 {
            result.limitations.push(format!("Skill '{name}' has multiple sources; choose a source explicitly in the Codex profile."));
            continue;
        }
        let skill = entries[0];
        result.skills.push(QuickSkill {
            id: skill.path.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            invocation_text: format!("${name}"),
            group_id: "codex-profile-skills".into(),
        });
    }
}

/// Orchid-owned roots are not registered with Codex discovery. Selected entries are pinned
/// for the session's read_skill broker.
pub(super) fn append_owned_root_skills(
    result: &mut RuntimeQuickFeatures,
    roots: &[PathBuf],
    group_id: &str,
) {
    let mut discovered = BTreeMap::<String, String>::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path().join("SKILL.md");
            if !path.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            discovered.insert(name, path.to_string_lossy().into_owned());
        }
    }
    for (name, path) in discovered {
        if result.skills.iter().any(|skill| skill.name == name) {
            result.limitations.push(format!(
                "Skill '{name}' is available from multiple roots and is not selected automatically."
            ));
            continue;
        }
        result.skills.push(QuickSkill {
            id: path,
            invocation_text: format!("${name}"),
            name,
            description: "Orchid-owned skill".into(),
            group_id: group_id.into(),
        });
    }
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
