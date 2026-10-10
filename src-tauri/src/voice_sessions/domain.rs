use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_INSTRUCTIONS: &str =
    include_str!("../../../resources/voice-sessions/instructions.md");
pub(crate) const INPUT_INSTRUCTIONS: &str =
    include_str!("../../../resources/voice-sessions/input-instructions.md");
pub(crate) const SUMMARY_INSTRUCTIONS: &str =
    include_str!("../../../resources/voice-sessions/summary-instructions.md");
pub(crate) const DIGEST_INSTRUCTIONS: &str =
    include_str!("../../../resources/voice-sessions/digest-instructions.md");
pub(crate) const OUTPUT_INSTRUCTIONS: &str =
    include_str!("../../../resources/voice-sessions/output-instructions.md");

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct VoicePrompts {
    pub(crate) relevance: String,
    pub(crate) completion: String,
    pub(crate) continuation: String,
    pub(crate) summary: String,
    pub(crate) router: String,
    pub(crate) digest: String,
    pub(crate) output: String,
    pub(crate) tools: String,
}
impl Default for VoicePrompts {
    fn default() -> Self {
        Self {
            relevance: include_str!("../../../resources/voice-sessions/relevance-instructions.md")
                .into(),
            completion: include_str!(
                "../../../resources/voice-sessions/completion-instructions.md"
            )
            .into(),
            continuation: include_str!(
                "../../../resources/voice-sessions/continuation-instructions.md"
            )
            .into(),
            summary: SUMMARY_INSTRUCTIONS.into(),
            router: INPUT_INSTRUCTIONS.into(),
            digest: DIGEST_INSTRUCTIONS.into(),
            output: OUTPUT_INSTRUCTIONS.into(),
            tools: include_str!("../../../resources/voice-sessions/tool-instructions.md").into(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoiceSettings {
    pub(crate) bind_address: String,
    pub(crate) token: String,
    pub(crate) profile_id: String,
    pub(crate) working_directory: Option<String>,
    pub(crate) instructions: String,
    #[serde(default)]
    pub(crate) prompts: VoicePrompts,
}
impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:4818".into(),
            token: uuid::Uuid::new_v4().simple().to_string(),
            profile_id: String::new(),
            working_directory: None,
            instructions: DEFAULT_INSTRUCTIONS.into(),
            prompts: VoicePrompts::default(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoiceState {
    pub(crate) settings: VoiceSettings,
    pub(crate) conversation: Option<Conversation>,
    #[serde(default)]
    pub(crate) connection_requested: bool,
    #[serde(skip_deserializing)]
    pub(crate) enabled: bool,
    #[serde(skip_deserializing)]
    pub(crate) phone_connected: bool,
    #[serde(skip_deserializing)]
    pub(crate) speech_configured: bool,
    #[serde(skip_deserializing)]
    pub(crate) error: Option<String>,
    #[serde(skip_deserializing)]
    pub(crate) speaking: bool,
    #[serde(skip_deserializing)]
    pub(crate) pending_audio: usize,
    #[serde(skip_deserializing)]
    pub(crate) capture: Option<PhoneCapture>,
    #[serde(skip_deserializing)]
    pub(crate) audio_progress: AudioProgress,
    #[serde(default)]
    pub(crate) input_policy: VoiceInputPolicy,
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct PhoneCapture {
    pub(crate) recording: bool,
    pub(crate) requested_microphone: String,
    pub(crate) routed_microphone: String,
    pub(crate) external: bool,
    pub(crate) silenced: bool,
    pub(crate) level: f64,
    pub(crate) frames_read: u64,
    pub(crate) segments_sent: u64,
    pub(crate) speech_detected: bool,
    pub(crate) error: Option<String>,
    pub(crate) updated_at: String,
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct AudioProgress {
    pub(crate) received: u64,
    pub(crate) transcribed: u64,
    pub(crate) last_received_at: Option<String>,
    pub(crate) last_transcript: Option<String>,
    pub(crate) last_transcribed_at: Option<String>,
    pub(crate) error: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, Default)]
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
    #[serde(default)]
    pub(crate) pipeline: Option<PipelineSessions>,
    #[serde(default)]
    pub(crate) input_buffer: InputBuffer,
    #[serde(default)]
    pub(crate) activity: Vec<Activity>,
    #[serde(default)]
    pub(crate) last_input_release: Option<InputRelease>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PipelineSessions {
    pub(crate) filter: String,
    pub(crate) summary: String,
    pub(crate) digest: String,
    pub(crate) output: String,
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InputBuffer {
    pub(crate) blocks: Vec<SpokenInput>,
    pub(crate) complete: bool,
    #[serde(default)]
    pub(crate) continuation_of: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Activity {
    pub(crate) id: String,
    pub(crate) source: String,
    pub(crate) kind: String,
    pub(crate) created_at: String,
    pub(crate) data: serde_json::Value,
}
impl Conversation {
    pub(crate) fn role(&self, id: &str) -> Option<&'static str> {
        if id == self.main_session_id {
            return Some("router");
        }
        if let Some(p) = &self.pipeline {
            for (session, role) in [
                (&p.filter, "filter"),
                (&p.summary, "summary"),
                (&p.digest, "digest"),
                (&p.output, "output"),
            ] {
                if id == session {
                    return Some(role);
                }
            }
        }
        self.delegations
            .iter()
            .any(|d| d.session_id == id)
            .then_some("worker")
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Delegation {
    pub(crate) session_id: String,
    pub(crate) from_session_id: String,
    pub(crate) return_session_id: String,
    pub(crate) task: String,
    pub(crate) invocation_id: Option<String>,
    #[serde(default)]
    pub(crate) created_at: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Observation {
    pub(crate) session_id: String,
    pub(crate) invocation_id: String,
    pub(crate) return_session_id: String,
    pub(crate) detail: String,
    #[serde(default)]
    pub(crate) created_at: String,
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
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpokenInput {
    pub(crate) id: String,
    pub(crate) text: String,
    pub(crate) created_at: String,
    #[serde(default)]
    pub(crate) relevance: Option<String>,
    #[serde(default)]
    pub(crate) completion: Option<String>,
    #[serde(default)]
    pub(crate) filter_pending: bool,
    #[serde(default)]
    pub(crate) stream_id: Option<String>,
    #[serde(default)]
    pub(crate) start_ms: u64,
    #[serde(default)]
    pub(crate) end_ms: u64,
    #[serde(default)]
    pub(crate) revision: u64,
    #[serde(default)]
    pub(crate) evaluated_revision: u64,
    #[serde(default)]
    pub(crate) delivered_revision: u64,
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InputRelease {
    pub(crate) id: String,
    pub(crate) created_at: String,
    pub(crate) blocks: Vec<SpokenInput>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct VoiceInputPolicy {
    pub(crate) sentences_per_query: usize,
    pub(crate) pause_seconds: u64,
    pub(crate) context_seconds: u64,
    pub(crate) followup_seconds: u64,
}
impl Default for VoiceInputPolicy {
    fn default() -> Self {
        Self {
            sentences_per_query: 2,
            pause_seconds: 2,
            context_seconds: 60,
            followup_seconds: 30,
        }
    }
}
impl VoiceInputPolicy {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !(1..=20).contains(&self.sentences_per_query)
            || !(1..=60).contains(&self.pause_seconds)
            || !(10..=120).contains(&self.context_seconds)
            || !(1..=600).contains(&self.followup_seconds)
        {
            return Err("Use 1–20 sentences, 1–60 pause seconds, 10–120 context seconds, and 1–600 follow-up seconds".into());
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Playback {
    pub(crate) utterance_id: String,
    pub(crate) played_ms: u64,
    pub(crate) state: String,
}
