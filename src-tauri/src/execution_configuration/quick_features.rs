use serde::{Deserialize, Serialize};

/// Provider-projected choices for a composer; this is discovery, not persisted Session policy.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeQuickFeatures {
    pub(crate) profile_ref: String,
    pub(crate) models: Vec<QuickModel>,
    pub(crate) skills: Vec<QuickSkill>,
    pub(crate) defaults: super::RuntimeSelections,
    pub(crate) limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuickModel {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) default_reasoning_mode: Option<String>,
    pub(crate) reasoning_modes: Vec<QuickReasoningMode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct QuickReasoningMode {
    pub(crate) id: String,
    pub(crate) description: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuickSkill {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) invocation_text: String,
    pub(crate) group_id: String,
}
