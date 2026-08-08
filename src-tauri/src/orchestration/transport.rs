use super::{
    application::{ManagedPlanBuilderService, OrchestrationApplication},
    bootstrap_transition::{BootstrapTransitionQueryV2, PostConfirmationTransitionService},
    file_review_originating_entry::{
        FileReviewOriginatingEntryError, FileReviewOriginatingEntryService,
    },
    repository::NativeQueryV2,
    sprint_runner_transition::{SprintRunnerTransitionQueryV1, SprintRunnerTransitionService},
};
use crate::agent_sessions::{
    application::SendAgentSessionMessageResult,
    domain::{AgentRuntimeOptions, AgentSessionId},
};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

pub(crate) struct OrchestrationTauriState {
    application: Arc<OrchestrationApplication>,
}

pub(crate) struct ContextualFileReviewTauriState {
    application: Arc<OrchestrationApplication>,
    service: Option<Arc<FileReviewOriginatingEntryService>>,
}
impl ContextualFileReviewTauriState {
    pub(crate) fn available(
        application: Arc<OrchestrationApplication>,
        service: Arc<FileReviewOriginatingEntryService>,
    ) -> Self {
        Self {
            application,
            service: Some(service),
        }
    }

    pub(crate) fn unavailable(application: Arc<OrchestrationApplication>) -> Self {
        Self {
            application,
            service: None,
        }
    }
}
impl OrchestrationTauriState {
    pub(crate) fn new(application: Arc<OrchestrationApplication>) -> Self {
        Self { application }
    }
}

pub(crate) struct ManagedPlanBuilderTauriState {
    service: Arc<ManagedPlanBuilderService>,
}

pub(crate) struct InitiationConfirmationTauriState {
    coordinator: Arc<super::confirmation::InitiationConfirmationCoordinator>,
}

pub(crate) struct BootstrapTransitionTauriState {
    service: Arc<PostConfirmationTransitionService>,
}

pub(crate) struct SprintRunnerTransitionTauriState {
    service: Arc<SprintRunnerTransitionService>,
}
impl SprintRunnerTransitionTauriState {
    pub(crate) fn new(service: Arc<SprintRunnerTransitionService>) -> Self {
        Self { service }
    }
}

impl BootstrapTransitionTauriState {
    pub(crate) fn new(service: Arc<PostConfirmationTransitionService>) -> Self {
        Self { service }
    }

    pub(crate) fn service(&self) -> &PostConfirmationTransitionService {
        &self.service
    }
}
impl InitiationConfirmationTauriState {
    pub(crate) fn new(
        coordinator: Arc<super::confirmation::InitiationConfirmationCoordinator>,
    ) -> Self {
        Self { coordinator }
    }
}

pub(crate) struct TauriInitiationConfirmationNotifier {
    app: AppHandle,
}
impl TauriInitiationConfirmationNotifier {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}
impl super::confirmation::InitiationConfirmationNotifier for TauriInitiationConfirmationNotifier {
    fn notify(
        &self,
        event: super::confirmation::InitiationConfirmationEvent,
    ) -> Result<(), String> {
        self.app
            .emit(super::confirmation::INITIATION_CONFIRMATION_EVENT, event)
            .map_err(|error| error.to_string())
    }
}
impl ManagedPlanBuilderTauriState {
    pub(crate) fn new(service: Arc<ManagedPlanBuilderService>) -> Self {
        Self { service }
    }
    pub(crate) fn service(&self) -> &ManagedPlanBuilderService {
        &self.service
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendManagedPlanBuilderMessageInput {
    session_id: Option<String>,
    submitted_text: String,
    title: Option<String>,
    working_directory: Option<String>,
    requested_options: Option<AgentRuntimeOptions>,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendManagedPlanBuilderMessageResult {
    session_id: String,
    invocation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestManagedPlanBuilderActionInput {
    session_id: Option<String>,
    title: Option<String>,
    working_directory: Option<String>,
    requested_options: Option<AgentRuntimeOptions>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReconcileManagedPlanBuilderSessionInput {
    session_id: String,
    title: Option<String>,
}
#[tauri::command]
pub(crate) fn reconcile_managed_plan_builder_session(
    state: State<'_, ManagedPlanBuilderTauriState>,
    input: ReconcileManagedPlanBuilderSessionInput,
) -> Result<super::application::ManagedPlanBuilderDraft, String> {
    let session_id = AgentSessionId::new(input.session_id).map_err(|error| error.to_string())?;
    state.service.reconcile_session(session_id, input.title)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadManagedPlanBuilderHarnessInspectionInput {
    session_id: String,
}

#[tauri::command]
pub(crate) fn load_managed_plan_builder_harness_inspection(
    state: State<'_, ManagedPlanBuilderTauriState>,
    input: LoadManagedPlanBuilderHarnessInspectionInput,
) -> Result<super::application::ManagedPlanBuilderHarnessInspection, String> {
    let session_id = AgentSessionId::new(input.session_id).map_err(|error| error.to_string())?;
    state.service.inspect_conversation_harness(session_id)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MutatePlanningDraftInput {
    epic_planning_draft_id: String,
    agent_session_id: String,
    idempotency_key: String,
    title: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestEpicInitiationConfirmationInput {
    epic_planning_draft_id: String,
    expected_revision_token: String,
    idempotency_key: String,
    root_branch: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResolveInitiationDecisionInput {
    Confirmed,
    Rejected,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolveEpicInitiationConfirmationInput {
    request_id: String,
    decision: ResolveInitiationDecisionInput,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitiationConfirmationTransportError {
    code: &'static str,
}
impl From<super::confirmation::InitiationConfirmationError>
    for InitiationConfirmationTransportError
{
    fn from(error: super::confirmation::InitiationConfirmationError) -> Self {
        let code = match error {
            super::confirmation::InitiationConfirmationError::Rejected => "rejected",
            super::confirmation::InitiationConfirmationError::RejectedNotificationFailed(_) => {
                "rejected_notification_failed"
            }
            super::confirmation::InitiationConfirmationError::ConfirmedButNotApplied(_) => {
                "confirmed_not_applied"
            }
            super::confirmation::InitiationConfirmationError::RequestNotFound => {
                "request_not_found"
            }
            super::confirmation::InitiationConfirmationError::TimedOut => "timed_out",
            super::confirmation::InitiationConfirmationError::Apply(error) => match error {
                super::domain::InitiateEpicError::RevisionConflict
                | super::domain::InitiateEpicError::ProposalMissing
                | super::domain::InitiateEpicError::DraftNotFound => "stale_proposal",
                super::domain::InitiateEpicError::Canceled => "canceled",
                super::domain::InitiateEpicError::AlreadyInitiated => "already_initiated",
                super::domain::InitiateEpicError::InvalidInput(_)
                | super::domain::InitiateEpicError::Forbidden
                | super::domain::InitiateEpicError::IdempotencyConflict
                | super::domain::InitiateEpicError::Unavailable(_) => "unavailable",
            },
            super::confirmation::InitiationConfirmationError::PersistedButIncomplete { .. } => {
                "persisted_reconciliation_required"
            }
            super::confirmation::InitiationConfirmationError::Unavailable(_) => "unavailable",
        };
        Self { code }
    }
}
#[tauri::command]
pub(crate) fn request_epic_initiation_confirmation(
    state: State<'_, InitiationConfirmationTauriState>,
    input: RequestEpicInitiationConfirmationInput,
) -> Result<super::confirmation::InitiationConfirmationRequest, InitiationConfirmationTransportError>
{
    state
        .coordinator
        .request(
            super::confirmation::InitiationRequestSource::Button,
            super::domain::InitiateEpicCommand {
                epic_planning_draft_id: super::domain::EpicPlanningDraftId::new(
                    input.epic_planning_draft_id,
                )
                .map_err(|_| InitiationConfirmationTransportError {
                    code: "unavailable",
                })?,
                expected_revision_token: input.expected_revision_token,
                actor_id: "application-user".into(),
                idempotency_key: input.idempotency_key,
                root_branch: Some(input.root_branch),
            },
        )
        .map_err(InitiationConfirmationTransportError::from)
}

#[tauri::command]
pub(crate) fn resolve_epic_initiation_confirmation(
    state: State<'_, InitiationConfirmationTauriState>,
    input: ResolveEpicInitiationConfirmationInput,
) -> Result<
    super::confirmation::InitiationConfirmationResolution,
    InitiationConfirmationTransportError,
> {
    state
        .coordinator
        .resolve(
            &input.request_id,
            match input.decision {
                ResolveInitiationDecisionInput::Confirmed => {
                    super::confirmation::UserInitiationDecision::Confirmed
                }
                ResolveInitiationDecisionInput::Rejected => {
                    super::confirmation::UserInitiationDecision::Rejected
                }
            },
        )
        .map_err(InitiationConfirmationTransportError::from)
}
#[tauri::command]
pub(crate) fn update_epic_planning_draft_title(
    state: State<'_, ManagedPlanBuilderTauriState>,
    input: MutatePlanningDraftInput,
) -> Result<(), String> {
    state.service.update_title(
        &input.epic_planning_draft_id,
        &input.agent_session_id,
        input.title.as_deref(),
        &input.idempotency_key,
    )
}
#[tauri::command]
pub(crate) fn cancel_epic_planning_draft(
    state: State<'_, ManagedPlanBuilderTauriState>,
    input: MutatePlanningDraftInput,
) -> Result<(), String> {
    state.service.cancel(
        &input.epic_planning_draft_id,
        &input.agent_session_id,
        &input.idempotency_key,
    )
}
#[tauri::command]
pub(crate) fn send_managed_plan_builder_message(
    state: State<'_, ManagedPlanBuilderTauriState>,
    input: SendManagedPlanBuilderMessageInput,
) -> Result<SendManagedPlanBuilderMessageResult, String> {
    let session_id = input
        .session_id
        .map(AgentSessionId::new)
        .transpose()
        .map_err(|error| error.to_string())?;
    state
        .service
        .send(
            session_id,
            input.submitted_text,
            input.title,
            input.working_directory,
            input.requested_options,
        )
        .map(
            |result: SendAgentSessionMessageResult| SendManagedPlanBuilderMessageResult {
                session_id: result.session_id.as_str().to_string(),
                invocation_id: result.invocation_id.as_str().to_string(),
            },
        )
}
#[tauri::command]
pub(crate) fn request_managed_plan_builder_action(
    state: State<'_, ManagedPlanBuilderTauriState>,
    input: RequestManagedPlanBuilderActionInput,
) -> Result<SendManagedPlanBuilderMessageResult, String> {
    let session_id = input
        .session_id
        .map(AgentSessionId::new)
        .transpose()
        .map_err(|error| error.to_string())?;
    state
        .service
        .request_plan(
            session_id,
            input.title,
            input.working_directory,
            input.requested_options,
        )
        .map(|result| SendManagedPlanBuilderMessageResult {
            session_id: result.session_id.as_str().to_string(),
            invocation_id: result.invocation_id.as_str().to_string(),
        })
}

/// A snapshot boundary only; callers must treat notifications/refreshes as non-authoritative.
#[tauri::command]
pub(crate) fn load_orchestration_native_query(
    state: State<'_, OrchestrationTauriState>,
) -> Result<NativeQueryV2, String> {
    state.application.native_query()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadScopedFileReviewInput {
    opaque_reference: String,
}

/// Read-only capability: the opaque reference is resolved and reauthorized by durable ownership.
#[tauri::command]
pub(crate) fn load_scoped_file_review(
    state: State<'_, OrchestrationTauriState>,
    input: LoadScopedFileReviewInput,
) -> Result<super::repository::ScopedFileReviewLoad, String> {
    state
        .application
        .load_scoped_file_review(&input.opaque_reference)
        .map_err(|_| "File Review facts are unavailable.".to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestContextualFileReviewInput {
    sprint_id: String,
}

#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum RequestContextualFileReviewResult {
    Available {
        #[serde(rename = "opaqueReference")]
        opaque_reference: String,
        #[serde(rename = "idempotentReplay")]
        idempotent_replay: bool,
    },
    Unavailable {
        reason: &'static str,
    },
}

/// Application action: Sprint context selects the private relation; success is returned only after
/// durable production and a scoped reauthorization/load have both succeeded.
#[tauri::command]
pub(crate) fn request_contextual_file_review(
    state: State<'_, ContextualFileReviewTauriState>,
    input: RequestContextualFileReviewInput,
) -> Result<RequestContextualFileReviewResult, String> {
    let Some(service) = &state.service else {
        return Ok(RequestContextualFileReviewResult::Unavailable {
            reason: "not_ready",
        });
    };
    let produced = match service.produce_for_sprint_context(&input.sprint_id) {
        Ok(produced) => produced,
        Err(error) => {
            return Ok(RequestContextualFileReviewResult::Unavailable {
                reason: contextual_file_review_reason(error),
            })
        }
    };
    match state
        .application
        .load_scoped_file_review(&produced.opaque_reference)
        .map_err(|_| "File Review facts are unavailable.".to_string())?
    {
        super::repository::ScopedFileReviewLoad::Available { .. } => {
            Ok(RequestContextualFileReviewResult::Available {
                opaque_reference: produced.opaque_reference,
                idempotent_replay: produced.idempotent_replay,
            })
        }
        super::repository::ScopedFileReviewLoad::Unavailable
        | super::repository::ScopedFileReviewLoad::Unauthorized
        | super::repository::ScopedFileReviewLoad::Invalid => {
            Ok(RequestContextualFileReviewResult::Unavailable {
                reason: "source_unavailable",
            })
        }
    }
}

fn contextual_file_review_reason(error: FileReviewOriginatingEntryError) -> &'static str {
    match error {
        FileReviewOriginatingEntryError::Unauthorized
        | FileReviewOriginatingEntryError::InvalidRequest => "not_ready",
        FileReviewOriginatingEntryError::RuntimeSourceStale
        | FileReviewOriginatingEntryError::RuntimeSourceDirty
        | FileReviewOriginatingEntryError::RuntimeSourceIncompatible
        | FileReviewOriginatingEntryError::RuntimeSourceUnavailable
        | FileReviewOriginatingEntryError::RuntimeEvidenceMismatch
        | FileReviewOriginatingEntryError::ComparisonUnavailable => "source_not_ready",
        FileReviewOriginatingEntryError::Conflict => "conflict",
        FileReviewOriginatingEntryError::RepositoryUnavailable
        | FileReviewOriginatingEntryError::RepositoryMismatch
        | FileReviewOriginatingEntryError::GitObjectUnavailable
        | FileReviewOriginatingEntryError::InvalidGitState
        | FileReviewOriginatingEntryError::LimitsExceeded
        | FileReviewOriginatingEntryError::IncompleteArtifact
        | FileReviewOriginatingEntryError::Unavailable => "unavailable",
    }
}

#[tauri::command]
pub(crate) fn load_epic_bootstrap_transition_query(
    state: State<'_, BootstrapTransitionTauriState>,
) -> Result<BootstrapTransitionQueryV2, String> {
    state.service.query().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn load_sprint_runner_transition_query(
    state: State<'_, SprintRunnerTransitionTauriState>,
) -> Result<SprintRunnerTransitionQueryV1, String> {
    state.service.query().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::InitiationConfirmationTransportError;
    use crate::orchestration::confirmation::InitiationConfirmationError;
    use crate::orchestration::domain::InitiateEpicError;

    #[test]
    fn scoped_file_review_transport_shapes_are_bounded() {
        use crate::orchestration::repository::{
            FileReviewChangedFileDto, ScopedFileReviewDocument, ScopedFileReviewLoad,
        };
        let available = ScopedFileReviewLoad::Available {
            document: ScopedFileReviewDocument {
                document_ref_id: "document".into(),
                title: "Changed files".into(),
                summary: None,
                artifact_id: "artifact".into(),
                payload: vec![1],
                changed_files: vec![FileReviewChangedFileDto {
                    changed_file_reference_id: "file".into(),
                    display_name: "src/a.ts".into(),
                    change_kind: "modified".into(),
                    previous_display_name: None,
                }],
            },
        };
        assert_eq!(
            serde_json::to_value(available).unwrap()["status"],
            "available"
        );
        for value in [
            ScopedFileReviewLoad::Unavailable,
            ScopedFileReviewLoad::Unauthorized,
            ScopedFileReviewLoad::Invalid,
        ] {
            let json = serde_json::to_value(value).unwrap();
            assert!(matches!(
                json["status"].as_str(),
                Some("unavailable" | "unauthorized" | "invalid")
            ));
        }
        let error = "database error: C:\\secret.sqlite";
        let sanitized = Err::<(), _>(error.to_string())
            .map_err(|_| "File Review facts are unavailable.".to_string());
        assert_eq!(sanitized.unwrap_err(), "File Review facts are unavailable.");
    }

    #[test]
    fn contextual_file_review_failures_are_bounded() {
        use super::{contextual_file_review_reason, RequestContextualFileReviewResult};
        use crate::orchestration::file_review_originating_entry::FileReviewOriginatingEntryError;

        assert_eq!(
            contextual_file_review_reason(FileReviewOriginatingEntryError::Unauthorized),
            "not_ready"
        );
        assert_eq!(
            contextual_file_review_reason(FileReviewOriginatingEntryError::RuntimeSourceDirty),
            "source_not_ready"
        );
        assert_eq!(
            contextual_file_review_reason(FileReviewOriginatingEntryError::LimitsExceeded),
            "unavailable"
        );
        assert_eq!(
            serde_json::to_value(RequestContextualFileReviewResult::Available {
                opaque_reference: "opaque".into(),
                idempotent_replay: true,
            })
            .unwrap(),
            serde_json::json!({
                "status": "available",
                "opaqueReference": "opaque",
                "idempotentReplay": true,
            })
        );
    }

    #[test]
    fn confirmation_errors_are_safe_codes_without_native_details() {
        for (error, expected) in [
            (InitiateEpicError::RevisionConflict, "stale_proposal"),
            (InitiateEpicError::Canceled, "canceled"),
            (InitiateEpicError::AlreadyInitiated, "already_initiated"),
            (
                InitiateEpicError::Unavailable("storage detail".into()),
                "unavailable",
            ),
        ] {
            assert_eq!(
                serde_json::to_value(InitiationConfirmationTransportError::from(
                    InitiationConfirmationError::Apply(error),
                ))
                .unwrap(),
                serde_json::json!({ "code": expected })
            );
        }
        for (error, expected) in [
            (
                InitiationConfirmationError::RejectedNotificationFailed(
                    "notification detail".into(),
                ),
                "rejected_notification_failed",
            ),
            (
                InitiationConfirmationError::ConfirmedButNotApplied("notification detail".into()),
                "confirmed_not_applied",
            ),
        ] {
            assert_eq!(
                serde_json::to_value(InitiationConfirmationTransportError::from(error)).unwrap(),
                serde_json::json!({ "code": expected })
            );
        }
    }
}
