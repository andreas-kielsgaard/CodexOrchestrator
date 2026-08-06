//! Application-owned construction of bounded Handler and Implementer capability packages.
//!
//! Construction resolves an existing durable authorization only. It deliberately does not create
//! a Work Unit, attempt, Session, invocation, provider process, or application acceptance fact.

use super::{
    application::OrchestrationApplication,
    conversation_harness::{self, ConversationHarnessProfile, ConversationHarnessRole},
    conversation_harness_revision::{
        CreateHarnessRevisionCommand, CreateHarnessRevisionResult, HarnessRevision,
        HarnessRevisionCreationProvenance, HarnessRevisionError, HarnessRevisionHistoryOutcome,
        HarnessRevisionProvenanceKind, HarnessRevisionReadOutcome,
    },
    conversation_harness_working_copy::{
        HarnessEditorKind, HarnessWorkingCopyEditor, SaveHarnessWorkingCopyCommand,
    },
    execution_support::{
        AuthorizeExistingWorkUnitExecutionAttempt, ChangedFileManifestEntry, ExecutionSupportError,
        ExecutionSupportIntent, ExecutionSupportReference, ExecutionSupportResponse,
        ExecutionSupportService, WorkUnitExecutionRole,
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
    orchestration: Arc<OrchestrationApplication>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PinnedHandlerHarnessRevision {
    pub(crate) revision_id: String,
    pub(crate) harness_key: String,
    pub(crate) configuration_digest: String,
    pub(crate) repository_commit_ref: String,
    pub(crate) profile: ConversationHarnessProfile,
}
pub(crate) type PinnedImplementerHarnessRevision = PinnedHandlerHarnessRevision;

impl WorkUnitExecutionHarnessService {
    pub(crate) fn new(
        execution_support: Arc<ExecutionSupportService>,
        sessions: Arc<AgentSessionApplication>,
        orchestration: Arc<OrchestrationApplication>,
    ) -> Self {
        Self {
            execution_support,
            sessions,
            orchestration,
        }
    }

    /// New ordinal-0 attempts pin the verified original no-tool Handler revision. Existing
    /// attempts use `load_pinned_handler_revision` and never choose a later current revision.
    pub(crate) fn current_handler_revision(
        &self,
    ) -> Result<PinnedHandlerHarnessRevision, WorkUnitHarnessError> {
        let revisions = match self
            .orchestration
            .load_harness_revision_history("work_unit_handler") {
            HarnessRevisionHistoryOutcome::AvailableAndVerified { revisions } => revisions,
            HarnessRevisionHistoryOutcome::Missing => {
                return self.pinned_handler_revision_from_revision(
                    self.bootstrap_initial_handler_revision()?,
                )
            }
            HarnessRevisionHistoryOutcome::InvalidLocalCommitEvidence
            | HarnessRevisionHistoryOutcome::Unavailable => {
                return Err(WorkUnitHarnessError::Unavailable)
            }
        };
        if let Some(revision) = revisions.iter().find(|revision| {
            self.pinned_handler_revision_from_revision((*revision).clone())
                .map(|pinned| !pinned.profile.mcp.required && pinned.profile.mcp.enabled_tools.is_empty())
                .unwrap_or(false)
        }) {
            return self.pinned_handler_revision_from_revision((*revision).clone());
        }
        Err(WorkUnitHarnessError::Unavailable)
    }

    /// Publishes (once) the immutable Handler revision which exposes the one application-owned
    /// continuation action.  It deliberately does not alter an earlier pinned revision.
    pub(crate) fn current_handler_action_revision(
        &self,
    ) -> Result<PinnedHandlerHarnessRevision, WorkUnitHarnessError> {
        let history = self.orchestration.load_harness_revision_history("work_unit_handler");
        let revisions = match history {
            HarnessRevisionHistoryOutcome::AvailableAndVerified { revisions } => revisions,
            HarnessRevisionHistoryOutcome::Missing => {
                self.bootstrap_initial_handler_revision()?;
                match self.orchestration.load_harness_revision_history("work_unit_handler") {
                    HarnessRevisionHistoryOutcome::AvailableAndVerified { revisions } => revisions,
                    _ => return Err(WorkUnitHarnessError::Unavailable),
                }
            }
            _ => return Err(WorkUnitHarnessError::Unavailable),
        };
        if let Some(revision) = revisions.iter().rev().find(|revision| {
            self.pinned_handler_revision_from_revision((*revision).clone())
                .map(|pinned| pinned.profile.mcp.required
                    && pinned.profile.mcp.enabled_tools == ["request_work_unit_implementer"])
                .unwrap_or(false)
        }) {
            return self.pinned_handler_revision_from_revision((*revision).clone());
        }
        let predecessor = revisions.last().ok_or(WorkUnitHarnessError::Unavailable)?;
        let copy = self.orchestration.load_harness_working_copy("work_unit_handler")
            .map_err(|_| WorkUnitHarnessError::Unavailable)?;
        let expected = copy.as_ref().map_or(0, |copy| copy.draft_revision);
        let saved = self.orchestration.save_harness_working_copy(SaveHarnessWorkingCopyCommand {
            harness_key: "work_unit_handler".into(),
            configuration: conversation_harness::initial_work_unit_handler_revision_configuration()
                .map_err(|_| WorkUnitHarnessError::Unavailable)?,
            expected_current_revision: expected,
            editor: HarnessWorkingCopyEditor {
                kind: HarnessEditorKind::ApplicationUser,
                reference: "work-unit-handler-action-continuation".into(),
            },
            idempotency_key: format!("work-unit-handler-action-working-copy-{}", predecessor.revision_id),
        }).map_err(|_| WorkUnitHarnessError::Unavailable)?;
        let draft = match saved {
            super::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::Stored(copy)
            | super::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::IdempotentReplay(copy) => copy.draft_revision,
        };
        let revision = match self.orchestration.create_harness_revision(CreateHarnessRevisionCommand {
            harness_key: "work_unit_handler".into(),
            expected_source_draft_revision: draft,
            expected_predecessor_revision_id: Some(predecessor.revision_id.clone()),
            idempotency_key: format!("work-unit-handler-action-revision-{}", predecessor.revision_id),
            creation_provenance: HarnessRevisionCreationProvenance {
                kind: HarnessRevisionProvenanceKind::ApplicationUser,
                reference: "work-unit-handler-action-continuation".into(),
            },
        }) {
            Ok(CreateHarnessRevisionResult::Published(revision))
            | Ok(CreateHarnessRevisionResult::IdempotentReplay(revision)) => revision,
            Err(HarnessRevisionError::Conflict) => self.load_newly_published_handler_revision()?,
            Err(_) => return Err(WorkUnitHarnessError::Unavailable),
        };
        let pinned = self.pinned_handler_revision_from_revision(revision)?;
        if !pinned.profile.mcp.required
            || pinned.profile.mcp.enabled_tools != ["request_work_unit_implementer"] {
            return Err(WorkUnitHarnessError::Denied);
        }
        Ok(pinned)
    }

    /// Publishes once the immutable, read-only Handler review revision. It is selected only by
    /// the application after acceptance and never changes either prior Handler revision.
    pub(crate) fn current_handler_review_revision(&self) -> Result<PinnedHandlerHarnessRevision, WorkUnitHarnessError> {
        let revisions = match self.orchestration.load_harness_revision_history("work_unit_handler") {
            HarnessRevisionHistoryOutcome::AvailableAndVerified { revisions } => revisions,
            HarnessRevisionHistoryOutcome::Missing => {
                self.bootstrap_initial_handler_revision()?;
                match self.orchestration.load_harness_revision_history("work_unit_handler") {
                    HarnessRevisionHistoryOutcome::AvailableAndVerified { revisions } => revisions,
                    _ => return Err(WorkUnitHarnessError::Unavailable),
                }
            }
            _ => return Err(WorkUnitHarnessError::Unavailable),
        };
        let expected_tools = ["read_handler_review_evidence", "accept_implementation_outcome", "return_implementation_outcome"];
        if let Some(revision) = revisions.iter().rev().find(|revision| {
            self.pinned_handler_revision_from_revision((*revision).clone()).map(|pinned| {
                pinned.profile.mcp.required && pinned.profile.mcp.enabled_tools.iter().map(String::as_str).eq(expected_tools)
            }).unwrap_or(false)
        }) { return self.pinned_handler_revision_from_revision((*revision).clone()); }
        let predecessor = revisions.last().ok_or(WorkUnitHarnessError::Unavailable)?;
        let expected = self.orchestration.load_harness_working_copy("work_unit_handler").map_err(|_| WorkUnitHarnessError::Unavailable)?.as_ref().map_or(0, |copy| copy.draft_revision);
        let draft = match self.orchestration.save_harness_working_copy(SaveHarnessWorkingCopyCommand {
            harness_key: "work_unit_handler".into(),
            configuration: conversation_harness::handler_outcome_review_revision_configuration().map_err(|_| WorkUnitHarnessError::Unavailable)?,
            expected_current_revision: expected,
            editor: HarnessWorkingCopyEditor { kind: HarnessEditorKind::ApplicationUser, reference: "work-unit-handler-outcome-review".into() },
            idempotency_key: format!("work-unit-handler-review-working-copy-{}", predecessor.revision_id),
        }).map_err(|_| WorkUnitHarnessError::Unavailable)? {
            super::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::Stored(copy)
            | super::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::IdempotentReplay(copy) => copy.draft_revision,
        };
        let revision = match self.orchestration.create_harness_revision(CreateHarnessRevisionCommand {
            harness_key: "work_unit_handler".into(),
            expected_source_draft_revision: draft,
            expected_predecessor_revision_id: Some(predecessor.revision_id.clone()),
            idempotency_key: format!("work-unit-handler-review-revision-{}", predecessor.revision_id),
            creation_provenance: HarnessRevisionCreationProvenance { kind: HarnessRevisionProvenanceKind::ApplicationUser, reference: "work-unit-handler-outcome-review".into() },
        }) {
            Ok(CreateHarnessRevisionResult::Published(revision)) | Ok(CreateHarnessRevisionResult::IdempotentReplay(revision)) => revision,
            Err(HarnessRevisionError::Conflict) => self.load_newly_published_handler_revision()?,
            Err(_) => return Err(WorkUnitHarnessError::Unavailable),
        };
        let pinned = self.pinned_handler_revision_from_revision(revision)?;
        if !pinned.profile.mcp.required || pinned.profile.mcp.enabled_tools.iter().map(String::as_str).ne(expected_tools) { return Err(WorkUnitHarnessError::Denied); }
        Ok(pinned)
    }

    pub(crate) fn load_pinned_handler_revision(
        &self,
        revision_id: &str,
        configuration_digest: &str,
        repository_commit_ref: &str,
    ) -> Result<PinnedHandlerHarnessRevision, WorkUnitHarnessError> {
        let HarnessRevisionReadOutcome::AvailableAndVerified { revision } =
            self.orchestration.load_harness_revision(revision_id)
        else {
            return Err(WorkUnitHarnessError::Unavailable);
        };
        if revision.configuration_digest != configuration_digest
            || revision.repository_commit_ref != repository_commit_ref
        {
            return Err(WorkUnitHarnessError::Denied);
        }
        self.pinned_handler_revision_from_revision(revision)
    }
    pub(crate) fn current_implementer_revision(&self) -> Result<PinnedImplementerHarnessRevision, WorkUnitHarnessError> {
        let revision = match self.orchestration.load_harness_revision_history("work_unit_implementer") {
            HarnessRevisionHistoryOutcome::AvailableAndVerified { revisions } => revisions.iter().rev()
                .find(|revision| self.pinned_implementer_revision_from_revision((*revision).clone()).map(|pinned| !pinned.profile.mcp.required && pinned.profile.mcp.enabled_tools.is_empty()).unwrap_or(false))
                .cloned().ok_or(WorkUnitHarnessError::Unavailable)?,
            HarnessRevisionHistoryOutcome::Missing => self.bootstrap_initial_implementer_revision()?,
            _ => return Err(WorkUnitHarnessError::Unavailable),
        };
        self.pinned_implementer_revision_from_revision(revision)
    }
    /// Publishes once a separate reporting revision.  It is never substituted for an attempt's
    /// original actionless revision and is used only by a same-Session continuation.
    pub(crate) fn current_implementer_reporting_revision(&self) -> Result<PinnedImplementerHarnessRevision, WorkUnitHarnessError> {
        let revisions = match self.orchestration.load_harness_revision_history("work_unit_implementer") {
            HarnessRevisionHistoryOutcome::AvailableAndVerified { revisions } => revisions,
            HarnessRevisionHistoryOutcome::Missing => {
                self.bootstrap_initial_implementer_revision()?;
                match self.orchestration.load_harness_revision_history("work_unit_implementer") {
                    HarnessRevisionHistoryOutcome::AvailableAndVerified { revisions } => revisions,
                    _ => return Err(WorkUnitHarnessError::Unavailable),
                }
            }
            _ => return Err(WorkUnitHarnessError::Unavailable),
        };
        if let Some(revision) = revisions.iter().rev().find(|revision| {
            self.pinned_implementer_revision_from_revision((*revision).clone())
                .map(|pinned| pinned.profile.mcp.required && pinned.profile.mcp.enabled_tools == ["submit_implementation_outcome", "complete_implementation_outcome"])
                .unwrap_or(false)
        }) {
            return self.pinned_implementer_revision_from_revision((*revision).clone());
        }
        let predecessor = revisions.last().ok_or(WorkUnitHarnessError::Unavailable)?;
        let expected = self.orchestration.load_harness_working_copy("work_unit_implementer")
            .map_err(|_| WorkUnitHarnessError::Unavailable)?
            .as_ref().map_or(0, |copy| copy.draft_revision);
        let draft = match self.orchestration.save_harness_working_copy(SaveHarnessWorkingCopyCommand {
            harness_key: "work_unit_implementer".into(),
            configuration: conversation_harness::implementer_outcome_reporting_revision_configuration().map_err(|_| WorkUnitHarnessError::Unavailable)?,
            expected_current_revision: expected,
            editor: HarnessWorkingCopyEditor { kind: HarnessEditorKind::ApplicationUser, reference: "work-unit-implementer-outcome-reporting".into() },
            idempotency_key: format!("work-unit-implementer-outcome-working-copy-{}", predecessor.revision_id),
        }).map_err(|_| WorkUnitHarnessError::Unavailable)? {
            super::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::Stored(copy)
            | super::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::IdempotentReplay(copy) => copy.draft_revision,
        };
        let revision = match self.orchestration.create_harness_revision(CreateHarnessRevisionCommand {
            harness_key: "work_unit_implementer".into(),
            expected_source_draft_revision: draft,
            expected_predecessor_revision_id: Some(predecessor.revision_id.clone()),
            idempotency_key: format!("work-unit-implementer-outcome-revision-{}", predecessor.revision_id),
            creation_provenance: HarnessRevisionCreationProvenance { kind: HarnessRevisionProvenanceKind::ApplicationUser, reference: "work-unit-implementer-outcome-reporting".into() },
        }) {
            Ok(CreateHarnessRevisionResult::Published(revision))
            | Ok(CreateHarnessRevisionResult::IdempotentReplay(revision)) => revision,
            Err(HarnessRevisionError::Conflict) => self.load_newly_published_implementer_revision()?,
            Err(_) => return Err(WorkUnitHarnessError::Unavailable),
        };
        let pinned = self.pinned_implementer_revision_from_revision(revision)?;
        if !pinned.profile.mcp.required || pinned.profile.mcp.enabled_tools != ["submit_implementation_outcome", "complete_implementation_outcome"] {
            return Err(WorkUnitHarnessError::Denied);
        }
        Ok(pinned)
    }
    pub(crate) fn load_pinned_implementer_revision(&self,id:&str,digest:&str,commit:&str)->Result<PinnedImplementerHarnessRevision,WorkUnitHarnessError>{let HarnessRevisionReadOutcome::AvailableAndVerified{revision}=self.orchestration.load_harness_revision(id) else{return Err(WorkUnitHarnessError::Unavailable)};if revision.configuration_digest!=digest||revision.repository_commit_ref!=commit{return Err(WorkUnitHarnessError::Denied)}self.pinned_implementer_revision_from_revision(revision)}
    fn bootstrap_initial_implementer_revision(&self)->Result<HarnessRevision,WorkUnitHarnessError>{let copy=self.orchestration.load_harness_working_copy("work_unit_implementer").map_err(|_|WorkUnitHarnessError::Unavailable)?;let draft=match copy{Some(copy)=>copy.draft_revision,None=>match self.orchestration.save_harness_working_copy(SaveHarnessWorkingCopyCommand{harness_key:"work_unit_implementer".into(),configuration:conversation_harness::initial_work_unit_implementer_revision_configuration().map_err(|_|WorkUnitHarnessError::Unavailable)?,expected_current_revision:0,editor:HarnessWorkingCopyEditor{kind:HarnessEditorKind::ApplicationUser,reference:"work-unit-implementer-activation".into()},idempotency_key:"work-unit-implementer-initial-working-copy".into()}){Ok(super::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::Stored(copy))|Ok(super::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::IdempotentReplay(copy))=>copy.draft_revision,Err(_)=>return self.load_newly_published_implementer_revision()}};match self.orchestration.create_harness_revision(CreateHarnessRevisionCommand{harness_key:"work_unit_implementer".into(),expected_source_draft_revision:draft,expected_predecessor_revision_id:None,idempotency_key:format!("work-unit-implementer-initial-revision-{draft}"),creation_provenance:HarnessRevisionCreationProvenance{kind:HarnessRevisionProvenanceKind::ApplicationUser,reference:"work-unit-implementer-activation".into()}}){Ok(CreateHarnessRevisionResult::Published(r))|Ok(CreateHarnessRevisionResult::IdempotentReplay(r))=>Ok(r),Err(HarnessRevisionError::Conflict)=>self.load_newly_published_implementer_revision(),Err(_)=>Err(WorkUnitHarnessError::Unavailable)}}
    fn load_newly_published_implementer_revision(&self)->Result<HarnessRevision,WorkUnitHarnessError>{match self.orchestration.load_harness_revision_history("work_unit_implementer"){HarnessRevisionHistoryOutcome::AvailableAndVerified{revisions}=>revisions.last().cloned().ok_or(WorkUnitHarnessError::Unavailable),_=>Err(WorkUnitHarnessError::Unavailable)}}
    fn pinned_implementer_revision_from_revision(&self,revision:HarnessRevision)->Result<PinnedImplementerHarnessRevision,WorkUnitHarnessError>{let version=u16::try_from(revision.source_draft_revision).map_err(|_|WorkUnitHarnessError::Denied)?;let profile=conversation_harness::profile_from_immutable_implementer_revision(&revision.configuration,version).map_err(|_|WorkUnitHarnessError::Denied)?;Ok(PinnedHandlerHarnessRevision{revision_id:revision.revision_id,harness_key:revision.harness_key,configuration_digest:revision.configuration_digest,repository_commit_ref:revision.repository_commit_ref,profile})}

    fn bootstrap_initial_handler_revision(&self) -> Result<HarnessRevision, WorkUnitHarnessError> {
        let working_copy = self
            .orchestration
            .load_harness_working_copy("work_unit_handler")
            .map_err(|_| WorkUnitHarnessError::Unavailable)?;
        let draft_revision = match working_copy {
            Some(copy) => copy.draft_revision,
            None => match self.orchestration.save_harness_working_copy(SaveHarnessWorkingCopyCommand {
                harness_key: "work_unit_handler".into(),
                configuration: conversation_harness::initial_work_unit_handler_baseline_revision_configuration()
                    .map_err(|_| WorkUnitHarnessError::Unavailable)?,
                expected_current_revision: 0,
                editor: HarnessWorkingCopyEditor {
                    kind: HarnessEditorKind::ApplicationUser,
                    reference: "work-unit-handler-activation".into(),
                },
                idempotency_key: "work-unit-handler-initial-working-copy".into(),
            }) {
                Ok(result) => match result {
                    super::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::Stored(copy)
                    | super::conversation_harness_working_copy::SaveHarnessWorkingCopyResult::IdempotentReplay(copy) => copy.draft_revision,
                },
                Err(_) => return self.load_newly_published_handler_revision(),
            },
        };
        let command = CreateHarnessRevisionCommand {
            harness_key: "work_unit_handler".into(),
            expected_source_draft_revision: draft_revision,
            expected_predecessor_revision_id: None,
            idempotency_key: format!("work-unit-handler-initial-revision-{draft_revision}"),
            creation_provenance: HarnessRevisionCreationProvenance {
                kind: HarnessRevisionProvenanceKind::ApplicationUser,
                reference: "work-unit-handler-activation".into(),
            },
        };
        match self.orchestration.create_harness_revision(command) {
            Ok(CreateHarnessRevisionResult::Published(revision))
            | Ok(CreateHarnessRevisionResult::IdempotentReplay(revision)) => Ok(revision),
            Err(HarnessRevisionError::Conflict) => self.load_newly_published_handler_revision(),
            Err(_) => Err(WorkUnitHarnessError::Unavailable),
        }
    }

    fn load_newly_published_handler_revision(
        &self,
    ) -> Result<HarnessRevision, WorkUnitHarnessError> {
        match self
            .orchestration
            .load_harness_revision_history("work_unit_handler")
        {
            HarnessRevisionHistoryOutcome::AvailableAndVerified { revisions } => revisions
                .last()
                .cloned()
                .ok_or(WorkUnitHarnessError::Unavailable),
            _ => Err(WorkUnitHarnessError::Unavailable),
        }
    }

    fn pinned_handler_revision_from_revision(
        &self,
        revision: HarnessRevision,
    ) -> Result<PinnedHandlerHarnessRevision, WorkUnitHarnessError> {
        let version = u16::try_from(revision.source_draft_revision)
            .map_err(|_| WorkUnitHarnessError::Denied)?;
        let profile = conversation_harness::profile_from_immutable_handler_revision(
            &revision.configuration,
            version,
        )
        .map_err(|_| WorkUnitHarnessError::Denied)?;
        Ok(PinnedHandlerHarnessRevision {
            revision_id: revision.revision_id,
            harness_key: revision.harness_key,
            configuration_digest: revision.configuration_digest,
            repository_commit_ref: revision.repository_commit_ref,
            profile,
        })
    }

    pub(crate) fn construct_for_existing_authorization(
        &self,
        attempt_id: &str,
        role: WorkUnitHarnessRole,
    ) -> Result<WorkUnitExecutionHarnessPackage, WorkUnitHarnessError> {
        let harness = conversation_harness::profile(role.harness_role())
            .map_err(|_| WorkUnitHarnessError::Unavailable)?;
        self.construct_for_pinned_profile(attempt_id, role, harness)
    }

    /// The activation coordinator supplies a previously persisted, self-validating profile
    /// snapshot.  A reopened attempt never consults the mutable catalog for its executable
    /// package.
    pub(crate) fn construct_for_pinned_profile(
        &self,
        attempt_id: &str,
        role: WorkUnitHarnessRole,
        harness: ConversationHarnessProfile,
    ) -> Result<WorkUnitExecutionHarnessPackage, WorkUnitHarnessError> {
        if role == WorkUnitHarnessRole::Implementer && !valid_implementer_mcp_profile(&harness) {
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

    /// The activation coordinator supplies only durable application-owned identities.  This is
    /// deliberately not a Harness action and cannot authorize an Implementer route.
    pub(crate) fn authorize_handler_attempt(
        &self,
        attempt_id: &str,
        work_unit_id: &str,
        sprint_git_authority_id: &str,
    ) -> Result<(), WorkUnitHarnessError> {
        self.execution_support.authorize_existing_attempt(
            AuthorizeExistingWorkUnitExecutionAttempt {
                attempt_id: attempt_id.into(),
                work_unit_id: work_unit_id.into(),
                role: WorkUnitExecutionRole::Handler,
                sprint_git_authority_id: sprint_git_authority_id.into(),
                execution_seed_object_id: None,
            },
        )?;
        Ok(())
    }
    pub(crate) fn authorize_implementer_attempt(&self,attempt_id:&str,work_unit_id:&str,authority:&str)->Result<(),WorkUnitHarnessError>{self.authorize_implementer_attempt_at_seed(attempt_id,work_unit_id,authority,None)}

    /// The retry coordinator provides a commit it derived and pinned from the accepted
    /// ordinal-0 evidence.  No Harness, caller, or runtime route can choose this seed.
    pub(crate) fn authorize_implementer_attempt_at_seed(&self,attempt_id:&str,work_unit_id:&str,authority:&str,seed:Option<String>)->Result<(),WorkUnitHarnessError>{self.execution_support.authorize_existing_attempt(AuthorizeExistingWorkUnitExecutionAttempt{attempt_id:attempt_id.into(),work_unit_id:work_unit_id.into(),role:WorkUnitExecutionRole::Implementer,sprint_git_authority_id:authority.into(),execution_seed_object_id:seed})?;Ok(())}

    /// The provider remains unable to write protected Git metadata in workspace-write mode.
    /// Seal only its durable, already-authorized isolated candidate after its original turn ends.
    pub(crate) fn commit_implementer_candidate(&self, attempt_id: &str) -> Result<bool, WorkUnitHarnessError> {
        self.execution_support
            .commit_implementer_candidate(attempt_id)
            .map_err(Into::into)
    }
}

fn valid_implementer_mcp_profile(harness: &ConversationHarnessProfile) -> bool {
    (!harness.mcp.required && harness.mcp.enabled_tools.is_empty())
        || (harness.mcp.required
            && harness.mcp.enabled_tools
                == ["submit_implementation_outcome", "complete_implementation_outcome"])
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
        package_runtime_launch_configuration(&self.harness, &self.reference.working_directory)
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

    pub(crate) fn capture_authorization_id(&self) -> Result<String, WorkUnitHarnessError> {
        match self.execution_support.consume(&self.reference.capability_ref, ExecutionSupportIntent::CaptureAuthorization)? {
            ExecutionSupportResponse::CaptureAuthorization(value) => Ok(value),
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
    working_directory: &str,
) -> WorkUnitExecutionRuntimeLaunchConfiguration {
    let mut additional_args = harness.runtime_configuration_args();
    // A worktree-runtime instance gives Codex a private CODEX_HOME without a trust record for
    // this just-created isolated worktree. Codex 0.144 then treats it as untrusted; with approval
    // set to never that reduces a requested WorkspaceWrite invocation to read-only. The application
    // has already authenticated this exact isolated worktree through the execution-support grant,
    // so pass one ephemeral, exact-project trust override only to its writable Implementer package.
    // It neither persists trust nor widens the workspace boundary.
    if harness.runtime_options().sandbox
        == Some(crate::agent_sessions::domain::RuntimeSandboxMode::WorkspaceWrite)
    {
        additional_args.extend([
            "-c".into(),
            workspace_trust_configuration(working_directory),
        ]);
    }
    WorkUnitExecutionRuntimeLaunchConfiguration {
        requested_options: harness.runtime_options(),
        extension: RuntimeLaunchExtension {
            additional_args,
            environment: vec![],
            initial_prompt_prefix: Some(harness.initial_prompt_prefix()),
        },
    }
}

/// Encodes the single quoted TOML key segment accepted by Codex CLI's `-c` surface.  This is
/// derived only from the already-authorized package reference, never from agent input.  Values
/// are intentionally not retained in launch provenance.
fn workspace_trust_configuration(working_directory: &str) -> String {
    let mut encoded = String::with_capacity(working_directory.len());
    for character in working_directory.chars() {
        match character {
            '\\' => encoded.push_str("\\\\"),
            '"' => encoded.push_str("\\\""),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            value => encoded.push(value),
        }
    }
    format!("projects.\"{encoded}\".trust_level=\"trusted\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_expose_only_the_implemented_handler_action() {
        let handler = conversation_harness::profile(ConversationHarnessRole::WorkUnitHandler).unwrap();
        let implementer = conversation_harness::profile(ConversationHarnessRole::WorkUnitImplementer).unwrap();
        assert_eq!(handler.mcp.enabled_tools, ["request_work_unit_implementer"]);
        assert!(handler.mcp.required);
        assert!(implementer.mcp.enabled_tools.is_empty());
    }

    #[test]
    fn implementer_mcp_profile_accepts_only_actionless_or_exact_reporting_pair() {
        let actionless = conversation_harness::profile(ConversationHarnessRole::WorkUnitImplementer).unwrap();
        assert!(valid_implementer_mcp_profile(&actionless));
        let mut false_nonempty = actionless.clone();
        false_nonempty.mcp.enabled_tools = vec!["unexpected".into()];
        assert!(!valid_implementer_mcp_profile(&false_nonempty));
        let mut false_pair = actionless.clone();
        false_pair.mcp.enabled_tools = vec!["submit_implementation_outcome".into(), "complete_implementation_outcome".into()];
        assert!(!valid_implementer_mcp_profile(&false_pair));
        let mut required_empty = actionless.clone();
        required_empty.mcp.required = true;
        assert!(!valid_implementer_mcp_profile(&required_empty));
        let mut reporting = actionless;
        reporting.mcp.required = true;
        reporting.mcp.enabled_tools = vec!["submit_implementation_outcome".into(), "complete_implementation_outcome".into()];
        assert!(valid_implementer_mcp_profile(&reporting));
    }

    #[test]
    fn runtime_launch_configuration_force_carries_read_only_harness_options() {
        let profile =
            conversation_harness::profile(ConversationHarnessRole::WorkUnitHandler).unwrap();
        let configuration = package_runtime_launch_configuration(&profile, "C:/read-only");
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

    #[test]
    fn workspace_write_package_carries_only_its_exact_ephemeral_project_trust_override() {
        let implementer =
            conversation_harness::profile(ConversationHarnessRole::WorkUnitImplementer).unwrap();
        let handler = conversation_harness::profile(ConversationHarnessRole::WorkUnitHandler).unwrap();
        let working_directory = r"C:\isolated\execution-workspace";
        let writable = package_runtime_launch_configuration(&implementer, working_directory);
        let read_only = package_runtime_launch_configuration(&handler, working_directory);

        assert!(writable.extension.additional_args.windows(2).any(|arguments| {
            arguments[0] == "-c"
                && arguments[1]
                    == r#"projects."C:\\isolated\\execution-workspace".trust_level="trusted""#
        }));
        assert!(!read_only.extension.additional_args.iter().any(|argument| {
            argument.contains("trust_level") || argument.contains("execution-workspace")
        }));
        assert!(read_only.extension.environment.is_empty());
    }
}
