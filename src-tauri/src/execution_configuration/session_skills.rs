use super::{ProfileRoutePolicy, RuntimeQuickFeatures, RuntimeSkillRoot};
use crate::agent_sessions::ports::RuntimeSkillInput;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Pin selected Codex-discovered skills and Orchid-owned roots for one session.
/// No source folder is copied, linked, or added to CODEX_HOME.
pub(crate) fn compile_session_skill_inputs(
    route: &ProfileRoutePolicy,
    features: &RuntimeQuickFeatures,
    roots: &[RuntimeSkillRoot],
) -> Result<Vec<RuntimeSkillInput>, String> {
    let groups = &route.skill_groups;
    if groups.is_empty() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for skill in &features.skills {
        let path = PathBuf::from(&skill.id);
        if !path.is_absolute()
            || path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md")
        {
            continue;
        }
        let Ok(path) = path.canonicalize() else {
            continue;
        };
        let codex_discovered =
            skill.group_id == "codex-profile-skills" && groups.contains("codex-profile-skills");
        let owned_root = roots.iter().any(|root| {
            root.group_id == skill.group_id
                && groups.contains(&root.group_id)
                && root
                    .path
                    .canonicalize()
                    .is_ok_and(|root| path.starts_with(root))
        });
        if !codex_discovered && !owned_root {
            continue;
        }
        result.push(RuntimeSkillInput {
            id: path.to_string_lossy().into_owned(),
            name: skill.name.clone(),
            path: path.to_string_lossy().into_owned(),
            content_sha256: sha256_file(&path)?,
            description: skill.description.clone(),
        });
    }
    result.sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));
    result.dedup_by(|left, right| left.name == right.name && left.path == right.path);
    if result.windows(2).any(|pair| pair[0].name == pair[1].name) {
        return Err("Selected skill roots contain duplicate skill names".into());
    }
    Ok(result)
}

/// Recheck selected immutable bytes immediately before every start or resume.
pub(crate) fn validate_session_skill_inputs(
    inputs: &[RuntimeSkillInput],
) -> Result<Vec<RuntimeSkillInput>, String> {
    for input in inputs {
        let path = Path::new(&input.path);
        if input.id.trim().is_empty()
            || input.name.trim().is_empty()
            || !path.is_absolute()
            || path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md")
        {
            return Err("Pinned session skill manifest is invalid".into());
        }
        if sha256_file(path)? != input.content_sha256 {
            return Err(format!(
                "Pinned skill `{}` changed on disk. Refresh the session capability policy before starting it.",
                input.name
            ));
        }
    }
    Ok(inputs.to_vec())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let contents = std::fs::read(path)
        .map_err(|error| format!("Could not read selected skill {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(contents)))
}

pub(crate) fn pin_discovered_skill(
    skill: &orchid_engine::codex::app_server::skills::CodexSkill,
) -> Result<RuntimeSkillInput, String> {
    let path = Path::new(&skill.path)
        .canonicalize()
        .map_err(|error| format!("Discovered skill path is unavailable: {error}"))?;
    if path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
        return Err("Discovered skill does not point to SKILL.md".into());
    }
    Ok(RuntimeSkillInput {
        id: path.to_string_lossy().into_owned(),
        name: skill.name.clone(),
        path: path.to_string_lossy().into_owned(),
        content_sha256: sha256_file(&path)?,
        description: skill.description.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_configuration::{
        CapabilityProfile, CapabilitySet, ModelAllowance, ProfileRoutePolicy, RuntimeSelections,
    };

    fn capability() -> CapabilityProfile {
        CapabilityProfile {
            execution: Default::default(),
            defaults: Default::default(),
            route_policies: vec![ProfileRoutePolicy {
                route_id: "route".into(),
                execution: Default::default(),
                model_allowances: vec![ModelAllowance {
                    model_id: "model".into(),
                    minimum_reasoning: "low".into(),
                    maximum_reasoning: "high".into(),
                }],
                mcp_groups: Default::default(),
                skill_groups: ["orchid-skills".into()].into_iter().collect(),
                defaults: RuntimeSelections::default(),
            }],
            default_route_id: Some("route".into()),
            contract_version: 1,
            capability_profile_id: "profile".into(),
            name: "Profile".into(),
            revision: 1,
            allowed_capabilities: CapabilitySet::default(),
        }
    }

    #[test]
    fn materializes_only_enabled_roots_and_rejects_changed_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("orchid");
        let skill_dir = root.join("review");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let skill = skill_dir.join("SKILL.md");
        std::fs::write(&skill, "version one").unwrap();
        let features = RuntimeQuickFeatures {
            skills: vec![super::super::QuickSkill {
                id: skill.to_string_lossy().into_owned(),
                name: "review".into(),
                description: String::new(),
                invocation_text: "$review".into(),
                group_id: "orchid-skills".into(),
            }],
            ..Default::default()
        };
        let inputs = compile_session_skill_inputs(
            capability().default_route().unwrap(),
            &features,
            &[RuntimeSkillRoot {
                group_id: "orchid-skills".into(),
                path: root,
            }],
        )
        .unwrap();
        assert_eq!(inputs.len(), 1);
        assert!(validate_session_skill_inputs(&inputs).is_ok());
        std::fs::write(&skill, "version two").unwrap();
        assert!(validate_session_skill_inputs(&inputs).is_err());
    }

    #[test]
    fn codex_discovered_skill_outside_home_is_selected_by_group() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory
            .path()
            .join("repo")
            .join(".agents")
            .join("skills")
            .join("review")
            .join("SKILL.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "review instructions").unwrap();
        let features = RuntimeQuickFeatures {
            skills: vec![super::super::QuickSkill {
                id: path.to_string_lossy().into_owned(),
                name: "review".into(),
                description: "Review".into(),
                invocation_text: "$review".into(),
                group_id: "codex-profile-skills".into(),
            }],
            ..Default::default()
        };
        let mut profile = capability();
        let route = &mut profile.route_policies[0];
        assert!(compile_session_skill_inputs(route, &features, &[])
            .unwrap()
            .is_empty());
        route.skill_groups.insert("codex-profile-skills".into());
        let inputs = compile_session_skill_inputs(route, &features, &[]).unwrap();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].name, "review");
        assert!(validate_session_skill_inputs(&inputs).is_ok());
    }
}
