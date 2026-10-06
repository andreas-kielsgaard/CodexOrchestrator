use super::{
    domain::*, gateway::GatewayOwner, repository::VoiceSessionRepository, speech::SpeechSession,
};
use crate::agent_sessions::{
    application::{AgentSessionApplication, SteerAgentSessionCommand},
    domain::{AgentInvocationInputProvenance, AgentSessionId},
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
            json!({"state":self.snapshot()?,"profiles":self.profiles.list().map_err(|e|e.to_string())?.iter().map(|p|json!({"id":p.capability_profile_id,"name":p.name})).collect::<Vec<_>>() }),
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
                });
                Ok(())
            })?;
        }
        let owner = super::gateway::start(self.clone(), &state.settings)?;
        *self
            .gateway
            .lock()
            .map_err(|_| "Voice gateway unavailable")? = Some(owner);
        self.update(|state| {
            state.enabled = true;
            state.error = None;
            Ok(())
        })?;
        self.schedule_observations();
        self.view()
    }
    pub(crate) fn stop(&self) -> Result<(), String> {
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
            state.phone_connected = false;
            Ok(())
        })
    }
    pub(crate) fn require_member(&self, caller: &str) -> Result<Conversation, String> {
        let conversation = self
            .snapshot()?
            .conversation
            .ok_or("Start a voice conversation first")?;
        if conversation.main_session_id != caller
            && !conversation
                .delegations
                .iter()
                .any(|d| d.session_id == caller)
        {
            return Err("Caller does not belong to this voice conversation".into());
        }
        Ok(conversation)
    }
    pub(crate) fn input(&self, id: String, text: String) -> Result<Value, String> {
        if text.trim().is_empty() {
            return Ok(json!({"ignored":"empty transcript"}));
        }
        let state = self.snapshot()?;
        if !state.enabled {
            return Err("Voice connection is stopped".into());
        }
        let conversation = state.conversation.ok_or("No active voice conversation")?;
        if conversation.inputs.iter().any(|input| input.id == id) {
            return Ok(json!({"duplicate":true}));
        }
        let context = self.context(&conversation);
        let main =
            AgentSessionId::new(conversation.main_session_id.clone()).map_err(|e| e.to_string())?;
        let history = self.agents.load_session(&main).map_err(|e| e.to_string())?;
        let active = history
            .invocations
            .iter()
            .rev()
            .find(|h| !h.invocation.status.is_terminal());
        let receipt = if let Some(active) = active {
            match self.agents.steer_session(SteerAgentSessionCommand {
                session_id: main.clone(),
                invocation_id: active.invocation.id.clone(),
                input_id: id.clone(),
                text: format!(
                    "{text}\n\nApplication playback and pending-topic context: {context}"
                ),
                attachments: vec![],
            }) {
                Ok(receipt) => json!({"steering":receipt}),
                Err(_) => self.deliver(
                    &main,
                    text.clone(),
                    AgentInvocationInputProvenance::User,
                    &conversation,
                )?,
            }
        } else {
            self.deliver(
                &main,
                text.clone(),
                AgentInvocationInputProvenance::User,
                &conversation,
            )?
        };
        self.update(|state| {
            let conversation = state.conversation.as_mut().ok_or("Conversation ended")?;
            conversation.inputs.push(SpokenInput {
                id: id.clone(),
                text: text.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            });
            Ok(())
        })?;
        let _ = self
            .events
            .send(json!({"type":"transcript","id":id,"text":text}));
        Ok(receipt)
    }
    pub(crate) fn context(&self, conversation: &Conversation) -> Value {
        json!({"conversationId":conversation.id,"mainSessionId":conversation.main_session_id,"delegations":conversation.delegations,"pendingTopics":conversation.pending_topics,"playback":conversation.playback,
            "utterance":conversation.playback.as_ref().and_then(|p| conversation.utterances.iter().find(|u|u.id==p.utterance_id))})
    }
    pub(crate) fn deliver(
        &self,
        target: &AgentSessionId,
        text: String,
        provenance: AgentInvocationInputProvenance,
        conversation: &Conversation,
    ) -> Result<Value, String> {
        let state = self.snapshot()?;
        let mut content = InvocationContent::query(text);
        if target.as_str() == conversation.main_session_id {
            content
                .context
                .push(InvocationContextPart::InitialInstructions {
                    source: "voice_session".into(),
                    version: 1,
                    content: state.settings.instructions,
                });
        }
        content
            .context
            .push(InvocationContextPart::ReferencedProductContent {
                source: "voice_session_state".into(),
                content: self.context(conversation).to_string(),
            });
        let receipt = self
            .agents
            .deliver_product_content(target.clone(), provenance, content)
            .map_err(|e| e.to_string())?;
        Ok(
            json!({"sessionId":receipt.acknowledgement.session_id,"invocationId":receipt.acknowledgement.invocation_id,"launchAccepted":receipt.launch_accepted}),
        )
    }
    pub(crate) fn publish(&self, caller: &str, arguments: Value) -> Result<Value, String> {
        let conversation = self.require_member(caller)?;
        if caller != conversation.main_session_id {
            return Err(
                "Return material to the reasoning session; only it publishes to the phone".into(),
            );
        }
        let text = crate::orchid_control::required(&arguments, "text")?.to_string();
        if text.len() > 8000 {
            return Err("Split long explanations into concise spoken passages".into());
        }
        let kind = crate::orchid_control::required(&arguments, "kind")?.to_string();
        if !["acknowledgement", "announcement", "explanation"].contains(&kind.as_str()) {
            return Err("Invalid publication kind".into());
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
