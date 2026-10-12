//! Tauri state, commands, DTO mapping, and persisted update notifications.

pub(crate) mod clipboard;
mod dto;
pub(crate) use dto::AgentSessionSummaryDto;
pub(crate) mod interactions;
pub(crate) mod jev;
pub(crate) mod quick_features;
pub(crate) mod selections;

use self::dto::{
    AgentInvocationDto, AgentSessionDetailsDto, AgentSessionDto, AgentSessionUpdateDto,
    CancelAgentInvocationCommandDto, CreateAgentSessionCommandDto, ListAgentSessionsQueryDto,
    LoadAgentSessionQueryDto, RenameAgentSessionCommandDto, SendAgentSessionMessageCommandDto,
    SendAgentSessionMessageResultDto, UpdateAgentSessionHarnessCommandDto,
    UpdateAgentSessionIdentityCommandDto, UpdateAgentSessionModelOverrideCommandDto,
};
use crate::agent_sessions::application::{
    AgentSessionApplication, AgentSessionNotification, AgentSessionNotifier,
};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

pub(crate) const AGENT_SESSION_UPDATE_EVENT: &str = "agent-session-update";

pub(crate) struct AgentSessionTauriState {
    application: Arc<AgentSessionApplication>,
}

impl AgentSessionTauriState {
    pub(crate) fn new(application: Arc<AgentSessionApplication>) -> Self {
        Self { application }
    }

    pub(crate) fn application(&self) -> &AgentSessionApplication {
        &self.application
    }
}

trait AgentSessionEventPublisher: Send + Sync {
    fn emit(&self, event: &str, payload: AgentSessionUpdateDto) -> Result<(), String>;
}

struct TauriAgentSessionEventPublisher {
    app: AppHandle,
}

impl AgentSessionEventPublisher for TauriAgentSessionEventPublisher {
    fn emit(&self, event: &str, payload: AgentSessionUpdateDto) -> Result<(), String> {
        self.app
            .emit(event, payload)
            .map_err(|error| error.to_string())
    }
}

pub(crate) struct TauriAgentSessionNotifier {
    publisher: Arc<dyn AgentSessionEventPublisher>,
    completion_sound: Option<Arc<dyn Fn(&str) -> bool + Send + Sync>>,
}

impl TauriAgentSessionNotifier {
    pub(crate) fn with_completion_sound(mut self, policy: Arc<dyn Fn(&str) -> bool + Send + Sync>) -> Self {
        self.completion_sound = Some(policy);
        self
    }
    pub(crate) fn new(app: AppHandle) -> Self {
        Self {
            publisher: Arc::new(TauriAgentSessionEventPublisher { app }),
            completion_sound: None,
        }
    }
}

impl AgentSessionNotifier for TauriAgentSessionNotifier {
    fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
        let mut update: AgentSessionUpdateDto = notification.into();
        if let AgentSessionUpdateDto::InvocationTerminal { session_id, completion_sound, .. } = &mut update {
            if let Some(policy) = &self.completion_sound { *completion_sound = policy(session_id.as_str()); }
        }
        self.publisher.emit(AGENT_SESSION_UPDATE_EVENT, update)
    }
}

#[cfg(test)]
mod tests;

mod commands;
mod queries;
pub(crate) use commands::*;
pub(crate) use queries::*;

pub(crate) mod import;

pub(crate) mod preparation;
pub(crate) mod queue;
