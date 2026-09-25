//! Application collaborators and persisted session notifications.

use crate::agent_sessions::domain::{
    AgentInvocation, AgentInvocationId, AgentRuntimeEvent, AgentRuntimeEventId, AgentSessionId,
};
use crate::agent_sessions::ports::RuntimeLaunchExtension;
use crate::harness_engine::domain::HarnessVersionRef;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AgentSessionNotification {
    PreparationUpdated {
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
    },
    TargetTransitionUpdated {
        session_id: AgentSessionId,
    },
    SteeringAccepted {
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
        input_id: String,
    },
    EventPersisted {
        session_id: AgentSessionId,
        event: AgentRuntimeEvent,
    },
    InvocationTerminal {
        session_id: AgentSessionId,
        invocation: AgentInvocation,
    },
    DiagnosticRecorded {
        session_id: AgentSessionId,
        invocation: AgentInvocation,
    },
}

pub(crate) trait AgentSessionNotifier: Send + Sync {
    fn notify(&self, notification: AgentSessionNotification) -> Result<(), String>;
}

use orchid_engine::contracts::ProviderConfigurationRef;

/// Application-owned authority for deriving the one native home used by a managed provider
/// launch. Callers can supply invocation-specific extensions, but never profile authority.
pub(crate) trait NativeProfileLaunchAuthority: Send + Sync {
    fn bound_configuration_ref(
        &self,
        _session_id: &AgentSessionId,
    ) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn prepare_destination_launch(
        &self,
        configuration: &ProviderConfigurationRef,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        resuming: bool,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<RuntimeLaunchExtension, String> {
        self.prepare_configured_launch(
            configuration,
            session_id,
            invocation_id,
            resuming,
            extension,
        )
    }
    fn commit_destination(
        &self,
        _configuration: &ProviderConfigurationRef,
        _session_id: &AgentSessionId,
    ) -> Result<(), String> {
        Ok(())
    }

    fn prepare_configured_launch(
        &self,
        _configuration: &ProviderConfigurationRef,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        resuming: bool,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<RuntimeLaunchExtension, String> {
        self.prepare_launch(session_id, invocation_id, resuming, extension)
    }
    fn prepare_launch(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        resuming: bool,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<RuntimeLaunchExtension, String>;
}

/// Session-owned runtime mediation consulted after application invocation and profile launch
/// configuration has been resolved. Product MCP connections are additive unless an existing
/// managed execution contract explicitly replaces its native MCP configuration.
pub(crate) trait SessionHarnessLaunchAuthority: Send + Sync {
    fn prepare_launch(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<Option<RuntimeLaunchExtension>, String>;
}

/// Resolves the Session's exact owned Harness reference before any launch configuration is built.
pub(crate) trait SessionHarnessVersionResolver: Send + Sync {
    fn resolve_session_harness_version(
        &self,
        session_id: &AgentSessionId,
        requested: &HarnessVersionRef,
    ) -> Result<HarnessVersionRef, String>;
}

pub(crate) trait AgentSessionClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub(crate) trait AgentSessionIdProvider: Send + Sync {
    fn session_id(&self) -> AgentSessionId;
    fn invocation_id(&self) -> AgentInvocationId;
    fn event_id(&self) -> AgentRuntimeEventId;
}

pub(crate) struct SystemAgentSessionProviders;

impl AgentSessionClock for SystemAgentSessionProviders {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

impl AgentSessionIdProvider for SystemAgentSessionProviders {
    fn session_id(&self) -> AgentSessionId {
        AgentSessionId::new(Uuid::new_v4().to_string()).expect("UUID is a valid session ID")
    }

    fn invocation_id(&self) -> AgentInvocationId {
        AgentInvocationId::new(Uuid::new_v4().to_string()).expect("UUID is a valid invocation ID")
    }

    fn event_id(&self) -> AgentRuntimeEventId {
        AgentRuntimeEventId::new(Uuid::new_v4().to_string()).expect("UUID is a valid event ID")
    }
}
