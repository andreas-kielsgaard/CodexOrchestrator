use super::{CapabilityProfile, RuntimeQuickFeatures, RuntimeSkillRoot};
use crate::agent_sessions::ports::RuntimeSkillInput;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Compile the selected master roots into immutable, explicit Codex skill inputs for one session.
/// No source folder is copied, linked, or added to CODEX_HOME.
pub(crate) fn compile_session_skill_inputs(
    capability: &CapabilityProfile,
    features: &RuntimeQuickFeatures,
    roots: &[RuntimeSkillRoot],
) -> Result<Vec<RuntimeSkillInput>, String> {
    let groups = capability
        .default_route()
        .map(|route| &route.skill_groups)
        .ok_or_else(|| "Capability Profile has no default execution route".to_string())?;
    if groups.is_empty() {
        return Ok(Vec::new());
    }
    let selected_roots = roots
        .iter()
        .filter(|root| groups.contains(&root.group_id))
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    for skill in &features.skills {
        let path = PathBuf::from(&skill.id);
        if !path.is_absolute()
            || path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md")
        {
            continue;
        }
        if !selected_roots
            .iter()
            .any(|root| path.starts_with(&root.path))
        {
            continue;
        }
        result.push(RuntimeSkillInput {
            id: skill.id.clone(),
            name: skill.name.clone(),
            path: path.to_string_lossy().into_owned(),
            content_sha256: sha256_file(&path)?,
        });
    }
    result.sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));
    result.dedup_by(|left, right| left.name == right.name && left.path == right.path);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_configuration::{
        CapabilitySet, ModelAllowance, ProfileRoutePolicy, RuntimeSelections,
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
            }],
            ..Default::default()
        };
        let inputs = compile_session_skill_inputs(
            &capability(),
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
}
