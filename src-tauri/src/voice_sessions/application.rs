use super::{
    domain::*, gateway::GatewayOwner, repository::VoiceSessionRepository, speech::SpeechSession,
};
use crate::agent_sessions::{
    application::AgentSessionApplication,
    domain::AgentSessionId,
    ports::{InvocationContent, InvocationContextPart},
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;

pub(crate) struct VoiceSessionApplication {
    pub(crate) agents: Arc<AgentSessionApplication>,
    pub(crate) profiles: Arc<crate::execution_configuration::CapabilityProfileService>,
    pub(crate) state: Mutex<VoiceState>,
    repository: VoiceSessionRepository,
    pub(crate) events: broadcast::Sender<Value>,
    gateway: Mutex<Option<GatewayOwner>>,
    lifecycle: Mutex<()>,
    pub(crate) speech: SpeechSession,
    pub(crate) observer_dispatch: Mutex<()>,
    pub(crate) input_dispatch: Mutex<()>,
}
impl VoiceSessionApplication {
    pub(crate) fn new(
        repository: VoiceSessionRepository,
        agents: Arc<AgentSessionApplication>,
        profiles: Arc<crate::execution_configuration::CapabilityProfileService>,
        speech_root: PathBuf,
    ) -> Result<Arc<Self>, String> {
        let mut state = repository.load()?;
        state.enabled = false;
        state.phone_connected = false;
        state.error = None;
        if let Some(c) = &mut state.conversation {
            c.input_buffer.complete = false;
            for input in &mut c.inputs {
                input.filter_pending = false;
            }
        }
        let speech = SpeechSession::new(speech_root);
        state.speech_configured = speech.configured();
        Ok(Arc::new(Self {
            agents,
            profiles,
            state: Mutex::new(state),
            repository,
            events: broadcast::channel(128).0,
            gateway: Mutex::new(None),
            lifecycle: Mutex::new(()),
            speech,
            observer_dispatch: Mutex::new(()),
            input_dispatch: Mutex::new(()),
        }))
    }
    pub(crate) fn snapshot(&self) -> Result<VoiceState, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Voice state unavailable")?
            .clone();
        state.speech_configured = self.speech.configured();
        Ok(state)
    }
    pub(crate) fn view(&self) -> Result<Value, String> {
        Ok(
            json!({"state":self.snapshot()?,"diagnostics":self.diagnostics()?,"defaultPrompts":VoicePrompts::default(),"profiles":self.profiles.list().map_err(|e|e.to_string())?.iter().map(|p|json!({"id":p.capability_profile_id,"name":p.name})).collect::<Vec<_>>() }),
        )
    }
    pub(crate) fn update<T>(
        &self,
        change: impl FnOnce(&mut VoiceState) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut state = self.state.lock().map_err(|_| "Voice state unavailable")?;
        let mut next = state.clone();
        let result = change(&mut next)?;
        self.repository.save(&next)?;
        *state = next;
        let _ = self.events.send(json!({"type":"changed"}));
        Ok(result)
    }
    pub(crate) fn set_error(&self, error: String) {
        let _ = self.update(|state| {
            state.error = Some(error.clone());
            Ok(())
        });
        let _ = self.events.send(json!({"type":"error","message":error}));
    }
    pub(crate) fn configure(&self, settings: VoiceSettings) -> Result<(), String> {
        super::gateway::validate_address(&settings.bind_address)?;
        if settings.token.len() < 24 {
            return Err("Use an access token of at least 24 characters".into());
        }
        self.update(|state| {
            if state.enabled {
                return Err("Stop the connection before changing setup".into());
            }
            state.settings = settings;
            Ok(())
        })
    }
    pub(crate) fn start(self: &Arc<Self>, fresh: bool) -> Result<Value, String> {
        let _lane = self
            .lifecycle
            .lock()
            .map_err(|_| "Voice lifecycle unavailable")?;
        let state = self.snapshot()?;
        if state.enabled {
            return self.view();
        }
        if fresh || state.conversation.is_none() {
            let main = self.agents.create_product_session(
                "Voice conversation".into(),
                state.settings.working_directory.clone(),
                &state.settings.profile_id,
                crate::orchid_control::tool_map(),
            )?;
            self.update(|state| {
                state.conversation = Some(Conversation {
                    id: uuid::Uuid::new_v4().to_string(),
                    main_session_id: main.id.to_string(),
                    delegations: vec![],
                    observed_sessions: vec![],
                    observations: vec![],
                    pending_topics: vec![],
                    utterances: vec![],
                    inputs: vec![],
                    playback: None,
                    ..Default::default()
                });
                Ok(())
            })?;
        }
        self.ensure_pipeline()?;
        let owner = super::gateway::start(self.clone(), &state.settings)?;
        *self
            .gateway
            .lock()
            .map_err(|_| "Voice gateway unavailable")? = Some(owner);
        self.update(|state| {
            state.enabled = true;
            state.connection_requested = true;
            state.error = None;
            Ok(())
        })?;
        self.schedule_observations();
        self.view()
    }
    pub(crate) fn stop(&self) -> Result<(), String> {
        self.close_gateway(true)
    }
    pub(crate) fn shutdown(&self) -> Result<(), String> {
        self.close_gateway(false)
    }
    pub(crate) fn restore_connection(self: &Arc<Self>) {
        if self
            .snapshot()
            .is_ok_and(|state| state.connection_requested)
        {
            if let Err(error) = self.start(false) {
                self.set_error(format!("Unable to restore phone connection: {error}"));
            }
        }
    }
    fn close_gateway(&self, explicit: bool) -> Result<(), String> {
        let _lane = self
            .lifecycle
            .lock()
            .map_err(|_| "Voice lifecycle unavailable")?;
        let _ = self.events.send(json!({"type":"ended"}));
        self.gateway
            .lock()
            .map_err(|_| "Voice gateway unavailable")?
            .take();
        self.speech.close();
        self.update(|state| {
            state.enabled = false;
            if explicit {
                state.connection_requested = false;
            }
            state.phone_connected = false;
            state.speaking = false;
            state.pending_audio = 0;
            Ok(())
        })
    }
    pub(crate) fn require_member(&self, caller: &str) -> Result<Conversation, String> {
        let conversation = self
            .snapshot()?
            .conversation
            .ok_or("Start a voice conversation first")?;
        if conversation.role(caller).is_none() {
            return Err("Caller does not belong to this voice conversation".into());
        }
        Ok(conversation)
    }
    pub(crate) fn context(&self, conversation: &Conversation) -> Value {
        json!({"conversationId":conversation.id,"mainSessionId":conversation.main_session_id,"pipeline":conversation.pipeline,
            "tasks":conversation.tasks,"assignments":conversation.delegations.iter().rev().take(20).map(|d|json!({"id":d.id,"taskId":d.task_id,"revision":d.revision,"sessionId":d.session_id,"invocationId":d.invocation_id,"delivery":d.delivery,"status":d.runtime_status,"outcome":d.outcome})).collect::<Vec<_>>(),
            "pendingTopics":conversation.pending_topics.iter().map(|t|json!({"id":t.id,"title":t.title})).collect::<Vec<_>>(),"playback":conversation.playback,
            "utterance":conversation.playback.as_ref().and_then(|p| conversation.utterances.iter().find(|u|u.id==p.utterance_id))})
    }
    pub(crate) fn delivery_content(
        &self,
        target: &AgentSessionId,
        text: String,
        conversation: &Conversation,
    ) -> Result<InvocationContent, String> {
        let state = self.snapshot()?;
        let mut content = InvocationContent::query(text);
        content
            .context
            .push(InvocationContextPart::InitialInstructions {
                source: "voice_tool_access".into(),
                version: 1,
                content: state.settings.prompts.tools.clone(),
            });
        if let Some(instructions) =
            self.role_instructions(conversation, target.as_str(), &state.settings.prompts)
        {
            content
                .context
                .push(InvocationContextPart::InitialInstructions {
                    source: "voice_stage".into(),
                    version: 2,
                    content: instructions.into(),
                });
        }
        if target.as_str() == conversation.main_session_id {
            content
                .context
                .push(InvocationContextPart::ReferencedProductContent {
                    source: "voice_preferences".into(),
                    content: state.settings.instructions,
                });
        }
        content
            .context
            .push(InvocationContextPart::ReferencedProductContent {
                source: "voice_session_state".into(),
                content: self.context(conversation).to_string(),
            });
        Ok(content)
    }
    pub(crate) fn publish(
        self: &Arc<Self>,
        caller: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let conversation = self.require_member(caller)?;
        let text = crate::orchid_control::required(&arguments, "text")?.to_string();
        if text.len() > 8000 {
            return Err("Split long explanations into concise spoken passages".into());
        }
        let kind = crate::orchid_control::required(&arguments, "kind")?.to_string();
        if !["acknowledgement", "announcement", "explanation"].contains(&kind.as_str()) {
            return Err("Invalid publication kind".into());
        }
        if !super::pipeline::may_publish(conversation.role(caller), &kind) {
            return Err(
                "Only output publishes speech; the summarizer may acknowledge receipt".into(),
            );
        }
        let utterance = Utterance {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            text,
            topic_id: arguments["topicId"].as_str().map(str::to_owned),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.update(|state| {
            let conversation = state.conversation.as_mut().ok_or("Conversation ended")?;
            if let Some(topic) = arguments.get("pendingTopic") {
                let topic: Topic =
                    serde_json::from_value(topic.clone()).map_err(|e| e.to_string())?;
                conversation.pending_topics.retain(|t| t.id != topic.id);
                conversation.pending_topics.push(topic);
            }
            if let Some(id) = arguments["completedTopicId"].as_str() {
                conversation.pending_topics.retain(|t| t.id != id);
            }
            conversation.utterances.push(utterance.clone());
            Ok(())
        })?;
        self.record_activity(
            caller,
            "publication",
            json!({"utterance":utterance}),
            &self.digest_targets(),
        )?;
        self.schedule_observations();
        let sent = self
            .events
            .send(json!({"type":"speak","utterance":utterance}))
            .is_ok();
        Ok(
            json!({"utteranceId":utterance.id,"published":true,"phoneConnected":self.snapshot()?.phone_connected,"sent":sent,"played":false}),
        )
    }
    pub(crate) fn playback(&self, playback: Playback) -> Result<(), String> {
        self.update(|state| {
            let conversation = state.conversation.as_mut().ok_or("No conversation")?;
            if !conversation
                .utterances
                .iter()
                .any(|u| u.id == playback.utterance_id)
            {
                return Err("Unknown utterance".into());
            }
            conversation.playback = Some(playback);
            Ok(())
        })
    }
}
