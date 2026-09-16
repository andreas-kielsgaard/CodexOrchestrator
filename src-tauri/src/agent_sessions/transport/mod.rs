//! Tauri state, commands, DTO mapping, and persisted update notifications.

mod dto;
pub(crate) use dto::AgentSessionSummaryDto;
pub(crate) mod interactions;
pub(crate) mod quick_features;
pub(crate) mod selections;

use self::dto::{
    AgentInvocationDto, AgentSessionDetailsDto, AgentSessionDto, AgentSessionUpdateDto,
    CancelAgentInvocationCommandDto, CreateAgentSessionCommandDto, ListAgentSessionsQueryDto,
    LoadAgentSessionQueryDto, SendAgentSessionMessageCommandDto, SendAgentSessionMessageResultDto,
    UpdateAgentSessionHarnessCommandDto, UpdateAgentSessionIdentityCommandDto,
    UpdateAgentSessionModelOverrideCommandDto,
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
}

impl TauriAgentSessionNotifier {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self {
            publisher: Arc::new(TauriAgentSessionEventPublisher { app }),
        }
    }
}

impl AgentSessionNotifier for TauriAgentSessionNotifier {
    fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
        self.publisher
            .emit(AGENT_SESSION_UPDATE_EVENT, notification.into())
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
