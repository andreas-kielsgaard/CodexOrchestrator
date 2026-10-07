use super::{domain::*, VoiceSessionApplication};
use crate::agent_sessions::domain::AgentSessionId;
use serde_json::{json, Value};

fn input_status(state: &VoiceState) -> (&'static str, &'static str) {
    if !state.enabled { return ("Stopped", "Resume the voice connection to accept input."); }
    if state.error.is_some() { return ("Error", "Input processing failed. See the error below."); }
    if state.pending_audio > 0 { return ("Transcribing", "Audio reached the laptop and is waiting for transcription."); }
    if let Some(c) = &state.conversation {
        if c.inputs.iter().any(|i| i.filter_pending) { return ("Checking input with JEV", "Deciding relevance and whether the input is complete."); }
        if !c.input_buffer.blocks.is_empty() {
            if !c.input_buffer.complete { return ("Waiting for you to finish", "Input was accepted, but JEV thinks more is coming. Indicate when you are ready, for example: That is all, please handle it."); }
            if state.speaking { return ("Waiting for speech to stop", "JEV marked the input complete; recording is still detecting speech."); }
            return ("Ready for reasoning", "Accepted input is ready for the reasoning sessions.");
        }
        if c.inputs.last().is_some_and(|i| i.relevance.as_deref() == Some("unaddressed")) {
            return ("Last input ignored", "JEV did not identify this speech as addressed to Orchid or related to a task.");
        }
    }
    if state.speaking { return ("Speech detected", "Recording speech into segments before transcription."); }
    ("Waiting for input", "No accepted input is currently held.")
}
impl VoiceSessionApplication {
    // Capture telemetry is transient; do not rewrite the conversation file every second.
    pub(crate) fn capture_report(&self, mut capture: PhoneCapture) -> Result<(), String> {
        capture.level = capture.level.clamp(0.0, 1.0);
        capture.updated_at = chrono::Utc::now().to_rfc3339();
        self.state.lock().map_err(|_| "Voice state unavailable")?.capture = Some(capture);
        Ok(())
    }
    pub(crate) fn audio_received(&self) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "Voice state unavailable")?;
        state.audio_progress.received += 1;
        state.audio_progress.last_received_at = Some(chrono::Utc::now().to_rfc3339());
        state.audio_progress.error = None;
        Ok(())
    }
    pub(crate) fn audio_transcribed(&self, result: Result<&str, &str>) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "Voice state unavailable")?;
        match result {
            Ok(text) => {
                state.audio_progress.transcribed += 1;
                state.audio_progress.last_transcript = Some(text.into());
                state.audio_progress.last_transcribed_at = Some(chrono::Utc::now().to_rfc3339());
                state.audio_progress.error = None;
            }
            Err(error) => state.audio_progress.error = Some(error.into()),
        }
        Ok(())
    }
    pub(crate) fn diagnostics(&self) -> Result<Value, String> {
        let state = self.snapshot()?;
        let (title, detail) = input_status(&state);
        let mut stages = vec![];
        if let Some(c) = &state.conversation {
            let mut ids = vec![("router", c.main_session_id.clone())];
            if let Some(p) = &c.pipeline {
                ids.extend([("summary", p.summary.clone()), ("digest", p.digest.clone()), ("output", p.output.clone())]);
            }
            ids.extend(c.delegations.iter().map(|d| ("worker", d.session_id.clone())));
            ids.sort(); ids.dedup();
            for (role, id) in ids {
                let history = self.agents.load_session(&AgentSessionId::new(id.clone()).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                let latest = history.invocations.last();
                let queued = c.observations.iter().filter(|o| o.return_session_id == id).count();
                stages.push(json!({"role":role,"sessionId":id,"status":latest.map(|h|h.invocation.status),"queued":queued,
                    "error":latest.and_then(|h|h.invocation.runtime_error.as_ref()).map(|e|&e.message)}));
            }
        }
        let last = state.conversation.as_ref().and_then(|c|c.inputs.last());
        Ok(json!({"updatedAt":chrono::Utc::now().to_rfc3339(),"phoneConnected":state.phone_connected,
            "capture":state.capture,"audio":state.audio_progress,"pendingAudio":state.pending_audio,
            "input":{"title":title,"detail":detail,"last":last},"stages":stages,"error":state.error}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostics_distinguish_held_input_from_idle_and_failed_filtering() {
        let mut state = VoiceState { enabled: true, conversation: Some(Conversation::default()), ..Default::default() };
        assert_eq!(input_status(&state).0, "Waiting for input");
        let block = SpokenInput { id:"input".into(), text:"Orchid, can you hear me?".into(), created_at:String::new(), relevance:Some("direct_request".into()), completion:Some("continue".into()), filter_pending:false };
        state.conversation.as_mut().unwrap().input_buffer.blocks.push(block.clone());
        state.conversation.as_mut().unwrap().inputs.push(block);
        assert_eq!(input_status(&state).0, "Waiting for you to finish");
        state.conversation.as_mut().unwrap().inputs[0].filter_pending = true;
        assert_eq!(input_status(&state).0, "Checking input with JEV");
        state.pending_audio = 1;
        assert_eq!(input_status(&state).0, "Transcribing");
        state.error = Some("JEV unavailable".into());
        assert_eq!(input_status(&state).0, "Error");
    }
    #[test]
    fn ignored_speech_is_visible_without_entering_reasoning() {
        let mut state = VoiceState { enabled:true, conversation:Some(Conversation::default()), ..Default::default() };
        state.conversation.as_mut().unwrap().inputs.push(SpokenInput { id:"ambient".into(), text:"Coffee?".into(), created_at:String::new(), relevance:Some("unaddressed".into()), completion:None, filter_pending:false });
        assert_eq!(input_status(&state).0, "Last input ignored");
    }
}
