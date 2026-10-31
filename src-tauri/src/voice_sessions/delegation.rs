use super::VoiceSessionApplication;
use crate::agent_sessions::domain::AgentSessionId;
use serde_json::{json, Value};
impl VoiceSessionApplication {
    pub(crate) fn observe(&self, caller: &str, arguments: Value) -> Result<Value, String> {
        self.require_member(caller)?;
        let sessions: Vec<String> =
            serde_json::from_value(arguments["sessionIds"].clone()).map_err(|e| e.to_string())?;
        for id in &sessions {
            self.agents
                .load_session(&AgentSessionId::new(id.clone()).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
        self.update(|state| {
            state
                .conversation
                .as_mut()
                .ok_or("No conversation")?
                .observed_sessions = sessions.clone();
            Ok(())
        })?;
        Ok(json!({"observing":sessions}))
    }
}
