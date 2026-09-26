//! Orchid-owned skill roots: the Orchid skill folder and each installed OTP package's skills.
//! They are offered with every provider configuration and delivered through the session's
//! pinned manifest; nothing is copied into a provider's native home.
use super::{
    capability_profile::{otp_skill_group, ORCHID_SKILL_GROUP},
    skill_mentions, QuickSkill, RuntimeQuickFeatures, RuntimeSkillRoot,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default)]
pub(crate) struct ProductSkillRoots {
    roots: Vec<RuntimeSkillRoot>,
}

impl ProductSkillRoots {
    pub(crate) fn new(orchid: Vec<PathBuf>, otp_packages: BTreeMap<String, Vec<PathBuf>>) -> Self {
        let mut roots = orchid
            .into_iter()
            .map(|path| RuntimeSkillRoot {
                group_id: ORCHID_SKILL_GROUP.into(),
                path,
            })
            .collect::<Vec<_>>();
        for (package, paths) in otp_packages {
            let group_id = otp_skill_group(&package);
            roots.extend(paths.into_iter().map(|path| RuntimeSkillRoot {
                group_id: group_id.clone(),
                path,
            }));
        }
        Self { roots }
    }

    pub(crate) fn roots(&self) -> &[RuntimeSkillRoot] {
        &self.roots
    }

    /// Adds each root's skills to the composer choices, grouped by capability group. A name
    /// already offered by another source is reported instead of silently shadowed.
    pub(crate) fn append_quick_skills(&self, features: &mut RuntimeQuickFeatures) {
        let mut groups = Vec::<(&str, Vec<&Path>)>::new();
        for root in &self.roots {
            match groups.iter_mut().find(|(group, _)| *group == root.group_id) {
                Some((_, paths)) => paths.push(&root.path),
                None => groups.push((&root.group_id, vec![&root.path])),
            }
        }
        for (group_id, paths) in groups {
            append_group(features, &paths, group_id);
        }
    }
}

fn append_group(features: &mut RuntimeQuickFeatures, roots: &[&Path], group_id: &str) {
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
        if features.skills.iter().any(|skill| skill.name == name) {
            features.limitations.push(format!(
                "Skill '{name}' is available from multiple roots and is not selected automatically."
            ));
            continue;
        }
        features.skills.push(QuickSkill {
            id: path,
            invocation_text: skill_mentions::mention(&name),
            name,
            description: "Orchid-owned skill".into(),
            group_id: group_id.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_orchid_and_otp_skills_by_group_and_reports_name_conflicts() {
        let directory = tempfile::tempdir().unwrap();
        let orchid = directory.path().join("orchid");
        let otp = directory.path().join("otp");
        for (root, name) in [(&orchid, "review"), (&otp, "deploy"), (&otp, "review")] {
            std::fs::create_dir_all(root.join(name)).unwrap();
            std::fs::write(root.join(name).join("SKILL.md"), name).unwrap();
        }
        let roots = ProductSkillRoots::new(
            vec![orchid],
            [("jobs".to_string(), vec![otp])].into_iter().collect(),
        );
        let mut features = RuntimeQuickFeatures::default();
        roots.append_quick_skills(&mut features);

        let offered = features
            .skills
            .iter()
            .map(|skill| (skill.name.as_str(), skill.group_id.as_str(), skill.invocation_text.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            offered,
            [("review", "orchid-skills", "$review"), ("deploy", "otp:jobs:skills", "$deploy")]
        );
        assert_eq!(features.limitations.len(), 1);
    }
}
