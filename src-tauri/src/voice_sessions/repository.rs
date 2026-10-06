use super::domain::VoiceState;
use crate::persistence::ActiveDatabase;
use rusqlite::OptionalExtension;
use std::sync::Arc;
pub(crate) const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS voice_session_state (id INTEGER PRIMARY KEY CHECK(id=1), state_json TEXT NOT NULL);";
pub(crate) struct VoiceSessionRepository(pub(crate) Arc<ActiveDatabase>);
impl VoiceSessionRepository {
    pub(crate) fn load(&self) -> Result<VoiceState, String> {
        let json: Option<String> = self
            .0
            .read("load voice session", |connection| {
                connection
                    .query_row(
                        "SELECT state_json FROM voice_session_state WHERE id=1",
                        [],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| e.into_string())?;
        json.map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
            .unwrap_or_else(|| Ok(VoiceState::default()))
    }
    pub(crate) fn save(&self, state: &VoiceState) -> Result<(), String> {
        let json = serde_json::to_string(state).map_err(|e| e.to_string())?;
        self.0.write("save voice session", |transaction| {
            transaction.execute("INSERT INTO voice_session_state VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET state_json=excluded.state_json", [json]).map_err(|e|e.to_string())?;
            Ok::<_,String>(())
        }).map_err(|e|e.into_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restart_retains_conversation_material_but_requires_explicit_reconnection() {
        let database = Arc::new(
            ActiveDatabase::from_connection(
                rusqlite::Connection::open_in_memory().unwrap(),
                |connection| connection.execute_batch(SCHEMA).map_err(|e| e.to_string()),
            )
            .unwrap(),
        );
        let repository = VoiceSessionRepository(database);
        let mut state = VoiceState::default();
        state.enabled = true;
        state.phone_connected = true;
        state.settings.profile_id = "profile".into();
        state.conversation = Some(super::super::domain::Conversation {
            id: "conversation".into(),
            main_session_id: "main".into(),
            delegations: vec![],
            observed_sessions: vec!["outside-workflow".into()],
            observations: vec![],
            pending_topics: vec![super::super::domain::Topic {
                id: "topic".into(),
                title: "Result".into(),
                material: "Retained facts".into(),
            }],
            utterances: vec![],
            inputs: vec![],
            playback: Some(super::super::domain::Playback {
                utterance_id: "utterance".into(),
                played_ms: 1600,
                state: "interrupted".into(),
            }),
        });
        repository.save(&state).unwrap();
        let restarted = repository.load().unwrap();
        assert!(!restarted.enabled);
        assert!(!restarted.phone_connected);
        let conversation = restarted.conversation.unwrap();
        assert_eq!(conversation.main_session_id, "main");
        assert_eq!(conversation.pending_topics[0].material, "Retained facts");
        assert_eq!(conversation.playback.unwrap().played_ms, 1600);
        assert_eq!(conversation.observed_sessions, ["outside-workflow"]);
    }
}
