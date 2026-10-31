use super::{
    domain::*, gateway::GatewayOwner, repository::VoiceSessionRepository, speech::SpeechSession,
};
use crate::agent_sessions::application::AgentSessionApplication;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;

pub(crate) struct VoiceSessionApplication {
    pub(crate) agents: Arc<AgentSessionApplication>,
    pub(crate) workflows: Arc<crate::workflows::execution::WorkflowExecutionService>,
    pub(crate) profiles: Arc<crate::execution_configuration::CapabilityProfileService>,
    pub(crate) state: Mutex<VoiceState>,
    pub(super) selected: Mutex<Option<Arc<Self>>>,
    pub(super) instances: Mutex<std::collections::BTreeMap<String, Arc<Self>>>,
    pub(super) connected_instance: std::sync::atomic::AtomicBool,
    pub(super) instance_only: bool,
    pub(super) repository: VoiceSessionRepository,
    pub(crate) events: broadcast::Sender<Value>,
    gateway: Mutex<Option<GatewayOwner>>,
    lifecycle: Mutex<()>,
    pub(crate) speech: SpeechSession,
    pub(crate) input_dispatch: Mutex<()>,
}
impl VoiceSessionApplication {
    pub(crate) fn new(
        repository: VoiceSessionRepository,
        agents: Arc<AgentSessionApplication>,
        profiles: Arc<crate::execution_configuration::CapabilityProfileService>,
        speech_root: PathBuf,
        workflows: Arc<crate::workflows::execution::WorkflowExecutionService>,
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
            workflows,
            profiles,
            state: Mutex::new(state),
            selected: Mutex::new(None),
            instances: Mutex::new(Default::default()),
            connected_instance: std::sync::atomic::AtomicBool::new(true),
            instance_only: false,
            repository,
            events: broadcast::channel(128).0,
            gateway: Mutex::new(None),
            lifecycle: Mutex::new(()),
            speech,
            input_dispatch: Mutex::new(()),
        }))
    }
    pub(super) fn scoped(&self, state: VoiceState) -> Arc<Self> {
        Arc::new(Self {
            agents: self.agents.clone(),
            workflows: self.workflows.clone(),
            profiles: self.profiles.clone(),
            state: Mutex::new(state),
            repository: VoiceSessionRepository(self.repository.0.clone()),
            events: self.events.clone(),
            gateway: Mutex::new(None),
            lifecycle: Mutex::new(()),
            speech: SpeechSession::new(
                std::env::var_os("LOCALAPPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_default()
                    .join("Orchid/voice-speech"),
            ),
            input_dispatch: Mutex::new(()),
            selected: Mutex::new(None),
            instances: Mutex::new(Default::default()),
            connected_instance: std::sync::atomic::AtomicBool::new(false),
            instance_only: true,
        })
    }
    pub(super) fn transient<T>(
        &self,
        change: impl FnOnce(&mut VoiceState) -> T,
    ) -> Result<T, String> {
        let selected = self
            .selected
            .lock()
            .map_err(|_| "Voice selection unavailable")?
            .clone();
        if let Some(selected) = selected {
            return selected.transient(change);
        }
        let mut state = self.state.lock().map_err(|_| "Voice state unavailable")?;
        Ok(change(&mut state))
    }
    pub(crate) fn snapshot(&self) -> Result<VoiceState, String> {
        if let Some(selected) = self
            .selected
            .lock()
            .map_err(|_| "Voice selection unavailable")?
            .clone()
        {
            return selected.snapshot();
        }

        let mut state = self
            .state
            .lock()
            .map_err(|_| "Voice state unavailable")?
            .clone();
        state.speech_configured = self.speech.configured();
        if !self
            .connected_instance
            .load(std::sync::atomic::Ordering::Acquire)
        {
            state.enabled = false;
            state.phone_connected = false;
        }
        Ok(state)
    }
    pub(crate) fn view(&self) -> Result<Value, String> {
        let state = self.snapshot()?;
        Ok(
            json!({"recipeId":state.default_design_id,"instances":self.workflows.instances.list()?.into_iter().filter(|i|i.recipe.nodes.iter().any(|n|matches!(&n.binding,crate::workflows::authoring::WorkflowNodeBinding::Capability(b) if b.capability.package=="voice" && b.capability.tool=="transcript"))).map(|i|json!({"id":i.id,"name":i.name})).collect::<Vec<_>>(),"state":state,"diagnostics":self.diagnostics()?,"defaultPrompts":VoicePrompts::default(),"profiles":self.profiles.list().map_err(|e|e.to_string())?.iter().map(|p|json!({"id":p.capability_profile_id,"name":p.name})).collect::<Vec<_>>() }),
        )
    }
    pub(crate) fn update<T>(
        &self,
        change: impl FnOnce(&mut VoiceState) -> Result<T, String>,
    ) -> Result<T, String> {
        let selected = self
            .selected
            .lock()
            .map_err(|_| "Voice selection unavailable")?
            .clone();
        if let Some(selected) = selected {
            return selected.update(change);
        }
        let mut state = self.state.lock().map_err(|_| "Voice state unavailable")?;
        let mut next = state.clone();
        let result = change(&mut next)?;
        self.repository.save_scoped(
            &next,
            !self.instance_only
                || self
                    .connected_instance
                    .load(std::sync::atomic::Ordering::Acquire),
        )?;
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
        })?;
        self.ensure_default_design()?;
        Ok(())
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
        if fresh
            || state
                .conversation
                .as_ref()
                .and_then(|c| c.workflow_instance_id.as_ref())
                .is_none()
        {
            return Err("Select a Voice Workflow instance before starting the connection".into());
        }
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
        self.view()
    }
    pub(crate) fn stop(&self) -> Result<(), String> {
        self.close_gateway(true)
    }
    pub(crate) fn shutdown(&self) -> Result<(), String> {
        self.close_gateway(false)
    }
    pub(crate) fn restore_connection(self: &Arc<Self>) {
        if self.snapshot().is_ok_and(|state| {
            state.connection_requested
                && state
                    .conversation
                    .as_ref()
                    .is_some_and(|c| c.workflow_instance_id.is_some())
        }) {
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
        if !self
            .workflows
            .instance_sessions(
                &self.workflows.instances.load(
                    conversation
                        .workflow_instance_id
                        .as_deref()
                        .ok_or("Legacy Voice history is read-only")?,
                )?,
            )?
            .iter()
            .any(|entry| entry.session.id() == caller)
        {
            return Err("Caller does not belong to this voice conversation".into());
        }
        Ok(conversation)
    }
}
