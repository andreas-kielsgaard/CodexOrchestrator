use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct VoiceWorkflowSettings {
    pub(crate) direct_reactions: bool,
    pub(crate) notification_sounds: std::collections::BTreeMap<String, bool>,
}
impl Default for VoiceWorkflowSettings {
    fn default() -> Self {
        Self {
            direct_reactions: true,
            notification_sounds: Default::default(),
        }
    }
}

impl super::VoiceSessionApplication {
    pub(crate) fn completion_sound_for_session(&self, session: &str) -> Option<bool> {
        self.workflows.completion_sound_for_session(session)
    }
}
