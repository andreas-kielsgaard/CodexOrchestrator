//! Application-owned construction of bounded Handler and Implementer capability packages.
//!
//! Construction resolves an existing durable authorization only. It deliberately does not create
//! a Work Unit, attempt, Session, invocation, provider process, or application acceptance fact.

use super::{
    conversation_harness::{self, ConversationHarnessProfile, ConversationHarnessRole},
    execution_support::{
        ChangedFileManifestEntry, ExecutionSupportError, ExecutionSupportIntent,
        ExecutionSupportReference, ExecutionSupportResponse, ExecutionSupportService,
        WorkUnitExecutionRole,
    },
};
use crate::{
    agent_sessions::application::{
        observation::project_invocation_observation, observation::AgentInvocationObservation,
    },
    agent_sessions::{
        application::AgentSessionApplication,
        domain::{AgentInvocationId, AgentRuntimeOptions, AgentSessionId},
        ports::RuntimeLaunchExtension,
    },
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkUnitHarnessRole {
    Handler,
    Implementer,
}

impl WorkUnitHarnessRole {
    fn execution_role(self) -> WorkUnitExecutionRole {
        match self {
            Self::Handler => WorkUnitExecutionRole::Handler,
            Self::Implementer => WorkUnitExecutionRole::Implementer,
        }
    }

    fn harness_role(self) -> ConversationHarnessRole {
        match self {
            Self::Handler => ConversationHarnessRole::WorkUnitHandler,
            Self::Implementer => ConversationHarnessRole::WorkUnitImplementer,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkUnitHarnessError {
    Denied,
    Unavailable,
    CorrelationMismatch,
}

impl From<ExecutionSupportError> for WorkUnitHarnessError {
    fn from(value: ExecutionSupportError) -> Self {
        match value {
            ExecutionSupportError::Denied
            | ExecutionSupportError::Invalid
            | ExecutionSupportError::Conflict => Self::Denied,
            ExecutionSupportError::CorrelationMismatch => Self::CorrelationMismatch,
            ExecutionSupportError::Unavailable => Self::Unavailable,
        }
    }
}

/// This service is composed by the application. `attempt_id` is an opaque application reference;
/// no raw repository, path, worktree, capability, or role route is accepted from a Harness.
pub(crate) struct WorkUnitExecutionHarnessService {
    execution_support: Arc<ExecutionSupportService>,
    sessions: Arc<AgentSessionApplication>,
}

impl WorkUnitExecutionHarnessService {
    pub(crate) fn new(
        execution_support: Arc<ExecutionSupportService>,
        sessions: Arc<AgentSessionApplication>,
    ) -> Self {
        Self {
            execution_support,
            sessions,
        }
    }

    pub(crate) fn construct_for_existing_authorization(
        &self,
        attempt_id: &str,
        role: WorkUnitHarnessRole,
    ) -> Result<WorkUnitExecutionHarnessPackage, WorkUnitHarnessError> {
        let harness = conversation_harness::profile(role.harness_role())
            .map_err(|_| WorkUnitHarnessError::Unavailable)?;
        if harness.mcp.required || !harness.mcp.enabled_tools.is_empty() {
            return Err(WorkUnitHarnessError::Unavailable);
        }
        let discovery_root = conversation_harness::role_discovery_root(role.harness_role())
            .map_err(|_| WorkUnitHarnessError::Unavailable)?;
        let reference = self
            .execution_support
            .grant_for_role(attempt_id, role.execution_role())?;
        Ok(WorkUnitExecutionHarnessPackage {
            harness,
            discovery_root,
            reference,
            execution_support: self.execution_support.clone(),
            sessions: self.sessions.clone(),
            correlation: Mutex::new(None),
        })
    }
}

/// A constructed package contains the application-derived working directory and opaque
/// capability, but exposes only semantic evidence actions. It has no Session-creation or launch
/// method. A later launch owner must bind the persisted invocation before observation is allowed.
pub(crate) struct WorkUnitExecutionHarnessPackage {
    harness: ConversationHarnessProfile,
    discovery_root: String,
    reference: ExecutionSupportReference,
    execution_support: Arc<ExecutionSupportService>,
    sessions: Arc<AgentSessionApplication>,
    correlation: Mutex<Option<(AgentSessionId, AgentInvocationId)>>,
}

/// The complete application-owned runtime input for a later launch. Constructing it neither
/// persists nor launches an invocation; its options cannot be supplied or overridden by a role.
pub(crate) struct WorkUnitExecutionRuntimeLaunchConfiguration {
    pub(crate) requested_options: AgentRuntimeOptions,
    pub(crate) extension: RuntimeLaunchExtension,
}

impl WorkUnitExecutionHarnessPackage {
    pub(crate) fn runtime_launch_configuration(
        &self,
    ) -> WorkUnitExecutionRuntimeLaunchConfiguration {
        package_runtime_launch_configuration(&self.harness)
    }

    pub(crate) fn working_directory(&self) -> &str {
        &self.reference.working_directory
    }

    pub(crate) fn discovery_root(&self) -> &str {
        &self.discovery_root
    }

    /// Binding is application-owned and fails closed once a distinct invocation is supplied.
    pub(crate) fn bind_correlated_invocation(
        &self,
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
    ) -> Result<(), WorkUnitHarnessError> {
        let mut correlation = self
            .correlation
            .lock()
            .map_err(|_| WorkUnitHarnessError::Unavailable)?;
        match correlation.as_ref() {
            None => {
                *correlation = Some((session_id, invocation_id));
                Ok(())
            }
            Some(existing) if existing == &(session_id, invocation_id) => Ok(()),
            Some(_) => Err(WorkUnitHarnessError::CorrelationMismatch),
        }
    }

    pub(crate) fn changed_file_manifest(
        &self,
    ) -> Result<Vec<ChangedFileManifestEntry>, WorkUnitHarnessError> {
        match self.execution_support.consume(
            &self.reference.capability_ref,
            ExecutionSupportIntent::ChangedFileManifest,
        )? {
            ExecutionSupportResponse::ChangedFileManifest(manifest) => Ok(manifest),
            _ => Err(WorkUnitHarnessError::Unavailable),
        }
    }

    pub(crate) fn comparison(&self) -> Result<Vec<u8>, WorkUnitHarnessError> {
        match self.execution_support.consume(
            &self.reference.capability_ref,
            ExecutionSupportIntent::Comparison,
        )? {
            ExecutionSupportResponse::Comparison(comparison) => Ok(comparison),
            _ => Err(WorkUnitHarnessError::Unavailable),
        }
    }

    pub(crate) fn evidence_content(
        &self,
        evidence_ref: &str,
    ) -> Result<Vec<u8>, WorkUnitHarnessError> {
        match self.execution_support.consume(
            &self.reference.capability_ref,
            ExecutionSupportIntent::EvidenceContent {
                evidence_ref: evidence_ref.into(),
            },
        )? {
            ExecutionSupportResponse::EvidenceContent(content) => Ok(content),
            _ => Err(WorkUnitHarnessError::Unavailable),
        }
    }

    pub(crate) fn observe_correlated_invocation(
        &self,
    ) -> Result<AgentInvocationObservation, WorkUnitHarnessError> {
        let (session_id, invocation_id) = self
            .correlation
            .lock()
            .map_err(|_| WorkUnitHarnessError::Unavailable)?
            .clone()
            .ok_or(WorkUnitHarnessError::Denied)?;
        let history = self
            .sessions
            .load_session(&session_id)
            .map_err(|_| WorkUnitHarnessError::Unavailable)?;
        let invocation = history
            .invocations
            .into_iter()
            .find(|entry| entry.invocation.id == invocation_id)
            .ok_or(WorkUnitHarnessError::CorrelationMismatch)?;
        Ok(project_invocation_observation(&invocation))
    }
}

fn package_runtime_launch_configuration(
    harness: &ConversationHarnessProfile,
) -> WorkUnitExecutionRuntimeLaunchConfiguration {
    WorkUnitExecutionRuntimeLaunchConfiguration {
        requested_options: harness.runtime_options(),
        extension: RuntimeLaunchExtension {
            additional_args: harness.runtime_configuration_args(),
            environment: vec![],
            initial_prompt_prefix: Some(harness.initial_prompt_prefix()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_expose_no_unimplemented_mcp_surface() {
        for role in [
            WorkUnitHarnessRole::Handler,
            WorkUnitHarnessRole::Implementer,
        ] {
            let profile = conversation_harness::profile(role.harness_role()).unwrap();
            assert!(profile.mcp.enabled_tools.is_empty());
            assert!(!profile.mcp.required);
        }
    }

    #[test]
    fn runtime_launch_configuration_force_carries_read_only_harness_options() {
        let profile =
            conversation_harness::profile(ConversationHarnessRole::WorkUnitHandler).unwrap();
        let configuration = package_runtime_launch_configuration(&profile);
        assert_eq!(
            configuration.extension.additional_args,
            ["-c", "approval_policy=\"never\""]
        );
        assert_eq!(
            configuration.requested_options.sandbox,
            Some(crate::agent_sessions::domain::RuntimeSandboxMode::ReadOnly)
        );
        assert!(configuration.extension.environment.is_empty());
        assert!(configuration
            .extension
            .initial_prompt_prefix
            .unwrap()
            .content
            .contains("already-authorized execution attempt"));
    }
}
