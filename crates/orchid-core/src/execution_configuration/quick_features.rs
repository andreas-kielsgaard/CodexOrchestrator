use orchid_engine::contracts::{ProviderConfigurationRef, ProviderSkillCatalogue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Composer choices for one configuration: provider-discovered models and native skills plus
/// Orchid's product skills. This is discovery, not persisted Session policy.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeQuickFeatures {
    pub configuration: Option<ProviderConfigurationRef>,
    pub models: Vec<QuickModel>,
    pub skills: Vec<QuickSkill>,
    pub defaults: super::RuntimeSelections,
    pub limitations: Vec<String>,
}

impl RuntimeQuickFeatures {
    /// Exposes enabled, unambiguous provider-native skills under the native skill group.
    pub fn append_native_skills(&mut self, catalogue: &ProviderSkillCatalogue) {
        self.limitations
            .extend(catalogue.limitations.iter().cloned());
        let mut names = BTreeMap::<&str, Vec<_>>::new();
        for skill in catalogue.skills.iter().filter(|skill| skill.enabled) {
            names.entry(skill.name.as_str()).or_default().push(skill);
        }
        for (name, entries) in names {
            if entries.len() != 1 {
                self.limitations.push(format!(
                    "Skill '{name}' has multiple sources; choose a source explicitly in the provider configuration."
                ));
                continue;
            }
            let skill = entries[0];
            self.skills.push(QuickSkill {
                id: skill.path.clone(),
                name: skill.name.clone(),
                description: skill.description.clone(),
                invocation_text: super::skill_mentions::mention(&skill.name),
                group_id: super::capability_profile::NATIVE_SKILL_GROUP.into(),
            });
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickModel {
    pub id: String,
    pub label: String,
    pub description: String,
    pub default_reasoning_mode: Option<String>,
    pub reasoning_modes: Vec<QuickReasoningMode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QuickReasoningMode {
    pub id: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Orchid's provider-independent mention text; see `skill_mentions`.
    pub invocation_text: String,
    pub group_id: String,
}
