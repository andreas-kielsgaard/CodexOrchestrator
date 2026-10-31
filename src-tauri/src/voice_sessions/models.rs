use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoiceModel {
    pub(crate) model: String,
    pub(crate) reasoning_mode: String,
    pub(crate) fast_mode: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct VoiceModels {
    pub(crate) response: VoiceModel,
    pub(crate) routing: VoiceModel,
}

impl Default for VoiceModels {
    fn default() -> Self {
        Self {
            response: VoiceModel {
                model: "gpt-6-luna".into(),
                reasoning_mode: "low".into(),
                fast_mode: true,
            },
            routing: VoiceModel {
                model: "gpt-6.1-sol".into(),
                reasoning_mode: "medium".into(),
                fast_mode: false,
            },
        }
    }
}
