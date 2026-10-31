use super::domain::VoiceState;
use crate::persistence::ActiveDatabase;
use rusqlite::OptionalExtension;
use std::sync::Arc;
pub(crate) const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS voice_workflow_state (instance_id TEXT PRIMARY KEY, conversation_json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS voice_connection_state (id INTEGER PRIMARY KEY CHECK(id=1), state_json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS voice_session_state (id INTEGER PRIMARY KEY CHECK(id=1), state_json TEXT NOT NULL);";
pub(crate) struct VoiceSessionRepository(pub(crate) Arc<ActiveDatabase>);
impl VoiceSessionRepository {
    pub(crate) fn load_instance(
        &self,
        id: &str,
    ) -> Result<Option<super::domain::Conversation>, String> {
        self.0
            .read("load Voice workflow state", |connection| {
                let saved: Option<String> = connection
                    .query_row(
                        "SELECT conversation_json FROM voice_workflow_state WHERE instance_id=?1",
                        [id],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                saved
                    .map(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
                    .transpose()
            })
            .map_err(|e| e.into_string())
    }

    pub(crate) fn load(&self) -> Result<VoiceState, String> {
        let json: Option<String> = self
            .0
            .read("load voice session", |connection| {
                connection
                    .query_row(
                        "SELECT state_json FROM voice_connection_state WHERE id=1 UNION ALL SELECT state_json FROM voice_session_state WHERE id=1 AND NOT EXISTS(SELECT 1 FROM voice_connection_state) LIMIT 1",
                        [],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| e.into_string())?;
        json.map(|value| {
            let saved: serde_json::Value =
                serde_json::from_str(&value).map_err(|e| e.to_string())?;
            let mut state: VoiceState =
                serde_json::from_value(saved.clone()).map_err(|e| e.to_string())?;
            if saved.get("connectionRequested").is_none() {
                state.connection_requested = saved["enabled"].as_bool().unwrap_or(false);
            }
            Ok(state)
        })
        .unwrap_or_else(|| Ok(VoiceState::default()))
    }
    #[cfg(test)]
    pub(crate) fn save(&self, state: &VoiceState) -> Result<(), String> {
        self.save_scoped(state, true)
    }
    pub(crate) fn save_scoped(&self, state: &VoiceState, connection: bool) -> Result<(), String> {
        let json = serde_json::to_string(state).map_err(|e| e.to_string())?;
        self.0.write("save voice session", |transaction| {
            if let Some(c) = &state.conversation {
                if let Some(id) = &c.workflow_instance_id {
                    transaction.execute("INSERT INTO voice_workflow_state VALUES(?1,?2) ON CONFLICT(instance_id) DO UPDATE SET conversation_json=excluded.conversation_json", rusqlite::params![id,serde_json::to_string(c).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
                }
            }
            if connection {transaction.execute("INSERT INTO voice_connection_state VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET state_json=excluded.state_json", [json]).map_err(|e|e.to_string())?;}
            Ok::<_,String>(())
        }).map_err(|e|e.into_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn background_instance_updates_do_not_replace_the_phone_selection() {
        let db = Arc::new(
            ActiveDatabase::from_connection(rusqlite::Connection::open_in_memory().unwrap(), |c| {
                c.execute_batch(SCHEMA).map_err(|e| e.to_string())
            })
            .unwrap(),
        );
        let repository = VoiceSessionRepository(db);
        let mut first = VoiceState::default();
        first.conversation = Some(super::super::domain::Conversation {
            id: "one".into(),
            workflow_instance_id: Some("one".into()),
            ..Default::default()
        });
        repository.save_scoped(&first, true).unwrap();
        let mut second = first.clone();
        second.conversation.as_mut().unwrap().id = "two".into();
        second.conversation.as_mut().unwrap().workflow_instance_id = Some("two".into());
        repository.save_scoped(&second, true).unwrap();
        first
            .conversation
            .as_mut()
            .unwrap()
            .pending_topics
            .push(super::super::domain::Topic {
                id: "result".into(),
                title: "Completed".into(),
                material: "background work".into(),
            });
        repository.save_scoped(&first, false).unwrap();
        assert_eq!(repository.load().unwrap().conversation.unwrap().id, "two");
        assert_eq!(
            repository
                .load_instance("one")
                .unwrap()
                .unwrap()
                .pending_topics
                .len(),
            1
        );
        assert!(repository
            .load_instance("two")
            .unwrap()
            .unwrap()
            .pending_topics
            .is_empty());
    }
    #[test]
    fn restart_retains_connection_intent_and_conversation_but_not_live_flags() {
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
        state.connection_requested = true;
        state.phone_connected = true;
        state.settings.profile_id = "profile".into();
        state.settings.prompts.output = "Custom concise output instructions".into();
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
            ..Default::default()
        });
        repository.save(&state).unwrap();
        let restarted = repository.load().unwrap();
        assert!(!restarted.enabled);
        assert!(restarted.connection_requested);
        assert!(!restarted.phone_connected);
        assert_eq!(
            restarted.settings.prompts.output,
            "Custom concise output instructions"
        );
        let conversation = restarted.conversation.unwrap();
        assert_eq!(conversation.main_session_id, "main");
        assert_eq!(conversation.pending_topics[0].material, "Retained facts");
        assert_eq!(conversation.playback.unwrap().played_ms, 1600);
        assert_eq!(conversation.observed_sessions, ["outside-workflow"]);
    }
    #[test]
    fn legacy_enabled_gateway_migrates_to_connection_intent_and_explicit_stop_persists() {
        let database = Arc::new(
            ActiveDatabase::from_connection(
                rusqlite::Connection::open_in_memory().unwrap(),
                |connection| connection.execute_batch(SCHEMA).map_err(|e| e.to_string()),
            )
            .unwrap(),
        );
        let repository = VoiceSessionRepository(database.clone());
        let mut legacy = serde_json::to_value(VoiceState::default()).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("connectionRequested");
        legacy["enabled"] = serde_json::json!(true);
        database
            .write("legacy voice fixture", |transaction| {
                transaction
                    .execute(
                        "INSERT INTO voice_session_state VALUES(1,?1)",
                        [legacy.to_string()],
                    )
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(())
            })
            .unwrap();
        let mut state = repository.load().unwrap();
        assert!(state.connection_requested && !state.enabled);
        state.connection_requested = false;
        repository.save(&state).unwrap();
        assert!(!repository.load().unwrap().connection_requested);
    }
    #[test]
    fn restart_retains_pipeline_and_accepted_buffer_without_live_capture_flags() {
        let mut state = VoiceState::default();
        state.enabled = true;
        state.speaking = true;
        state.pending_audio = 3;
        state.conversation = Some(serde_json::from_value(serde_json::json!({
            "id":"conversation","mainSessionId":"router","delegations":[],"observedSessions":[],"observations":[],
            "pendingTopics":[],"utterances":[],"inputs":[],"playback":null,
            "pipeline":{"filter":"jev","summary":"summary","digest":"digest","output":"output"},
            "inputBuffer":{"complete":false,"blocks":[{"id":"input","text":"Orchid, inspect","createdAt":"2026-10-07T10:00:00Z","relevance":"direct_request"}]}
        })).unwrap());
        let restarted: VoiceState =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert!(!restarted.enabled && !restarted.speaking && restarted.pending_audio == 0);
        let c = restarted.conversation.unwrap();
        assert_eq!(c.pipeline.unwrap().digest, "digest");
        assert_eq!(c.input_buffer.blocks[0].text, "Orchid, inspect");
        assert!(!c.input_buffer.complete);
    }
}
