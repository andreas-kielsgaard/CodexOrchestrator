use serde::{Deserialize, Serialize};

/// A skill discovered natively by a provider configuration. Product-owned skill roots are
/// discovered by Orchid itself; both sources become the same pinned session manifest entries.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSkill {
    pub name: String,
    pub description: String,
    /// Absolute path of the skill's `SKILL.md`.
    pub path: String,
    /// Provider-reported origin, for example a user home or repository scope.
    pub scope: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSkillCatalogue {
    pub skills: Vec<ProviderSkill>,
    pub limitations: Vec<String>,
}
