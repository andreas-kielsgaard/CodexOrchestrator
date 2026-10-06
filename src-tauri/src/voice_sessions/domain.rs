use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_INSTRUCTIONS: &str =
    include_str!("../../../resources/voice-sessions/instructions.md");

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoiceSettings {
    pub(crate) bind_address: String,
    pub(crate) token: String,
    pub(crate) profile_id: String,
    pub(crate) working_directory: Option<String>,
    pub(crate) instructions: String,
}
impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:4818".into(),
            token: uuid::Uuid::new_v4().simple().to_string(),
            profile_id: String::new(),
            working_directory: None,
            instructions: DEFAULT_INSTRUCTIONS.into(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoiceState {
    pub(crate) settings: VoiceSettings,
    pub(crate) conversation: Option<Conversation>,
    #[serde(skip_deserializing)]
    pub(crate) enabled: bool,
    #[serde(skip_deserializing)]
    pub(crate) phone_connected: bool,
    #[serde(skip_deserializing)]
    pub(crate) speech_configured: bool,
    #[serde(skip_deserializing)]
    pub(crate) error: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Conversation {
    pub(crate) id: String,
    pub(crate) main_session_id: String,
    pub(crate) delegations: Vec<Delegation>,
    pub(crate) observed_sessions: Vec<String>,
    pub(crate) observations: Vec<Observation>,
    pub(crate) pending_topics: Vec<Topic>,
    pub(crate) utterances: Vec<Utterance>,
    pub(crate) inputs: Vec<SpokenInput>,
    pub(crate) playback: Option<Playback>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Delegation {
    pub(crate) session_id: String,
    pub(crate) from_session_id: String,
    pub(crate) return_session_id: String,
    pub(crate) task: String,
    pub(crate) invocation_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Observation {
    pub(crate) session_id: String,
    pub(crate) invocation_id: String,
    pub(crate) return_session_id: String,
    pub(crate) detail: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Topic {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) material: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Utterance {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) text: String,
    pub(crate) topic_id: Option<String>,
    pub(crate) created_at: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpokenInput {
    pub(crate) id: String,
    pub(crate) text: String,
    pub(crate) created_at: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Playback {
    pub(crate) utterance_id: String,
    pub(crate) played_ms: u64,
    pub(crate) state: String,
}
