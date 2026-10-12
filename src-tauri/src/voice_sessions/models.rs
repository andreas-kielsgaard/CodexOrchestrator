use crate::agent_sessions::application::ProductSessionOptions;
use crate::execution_configuration::RuntimeSelections;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoiceModel {
    pub(crate) model: String,
    pub(crate) reasoning_mode: String,
    pub(crate) fast_mode: bool,
}

impl VoiceModel {
    pub(crate) fn options(&self) -> ProductSessionOptions {
        ProductSessionOptions {
            defaults: RuntimeSelections { model: Some(self.model.clone()), reasoning_mode: Some(self.reasoning_mode.clone()), sandbox_mode: None },
            fast_mode: Some(self.fast_mode),
        }
    }
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
            response: VoiceModel { model: "gpt-6-luna".into(), reasoning_mode: "low".into(), fast_mode: true },
            routing: VoiceModel { model: "gpt-6.1-sol".into(), reasoning_mode: "medium".into(), fast_mode: false },
        }
    }
}
