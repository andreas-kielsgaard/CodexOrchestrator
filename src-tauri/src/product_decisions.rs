//! Durable, application-owned Product Decision versions. This module deliberately does not
//! publish decisions into orchestration or infer any application scope.

use crate::agent_sessions::{
    application::{
        AgentSessionApplication, ApplicationInvocationLaunchEvidence, CreateAgentSessionCommand,
        CreateApplicationAgentSessionCommand, SendAgentSessionMessageCommand,
        SendIdempotentApplicationAgentSessionMessageCommand,
    },
    domain::{AgentInvocationId, AgentRuntimeOptions, AgentSessionId},
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{path::Path, sync::Mutex};
use tauri::State;
use uuid::Uuid;

pub(crate) const PRODUCT_DECISION_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS product_decisions (
  decision_id TEXT PRIMARY KEY,
  epic_id TEXT NOT NULL,
  current_version INTEGER NOT NULL CHECK(current_version >= 1),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS product_decision_versions (
  version_id TEXT PRIMARY KEY,
  decision_id TEXT NOT NULL,
  version INTEGER NOT NULL CHECK(version >= 1),
  title TEXT NOT NULL,
  statement TEXT NOT NULL,
  intent TEXT NOT NULL,
  acceptance_provenance_json TEXT NOT NULL CHECK(json_valid(acceptance_provenance_json)),
  accepted_at TEXT NOT NULL,
  UNIQUE(decision_id, version),
  FOREIGN KEY(decision_id) REFERENCES product_decisions(decision_id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS product_decision_evidence (
  version_id TEXT NOT NULL,
  evidence_id TEXT NOT NULL,
  evidence_kind TEXT NOT NULL CHECK(evidence_kind IN ('current_agent_passage','historical_unresolved')),
  evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json)),
  PRIMARY KEY(version_id, evidence_id),
  FOREIGN KEY(version_id) REFERENCES product_decision_versions(version_id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS product_decision_acceptance_commands (
  idempotency_key TEXT PRIMARY KEY,
  payload_fingerprint TEXT NOT NULL,
  version_id TEXT NOT NULL,
  FOREIGN KEY(version_id) REFERENCES product_decision_versions(version_id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS product_decision_correction_conversations (
  correction_id TEXT PRIMARY KEY,
  epic_id TEXT NOT NULL,
  decision_id TEXT NOT NULL,
  base_version INTEGER NOT NULL CHECK(base_version >= 1),
  session_id TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  UNIQUE(epic_id, decision_id, base_version)
);
CREATE TABLE IF NOT EXISTS product_decision_correction_proposals (
  proposal_id TEXT PRIMARY KEY,
  correction_id TEXT NOT NULL,
  title TEXT NOT NULL,
  statement TEXT NOT NULL,
  intent TEXT NOT NULL,
  proposal_passage_json TEXT NOT NULL CHECK(json_valid(proposal_passage_json)),
  created_at TEXT NOT NULL,
  FOREIGN KEY(correction_id) REFERENCES product_decision_correction_conversations(correction_id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS product_decision_correction_initializations (
  correction_id TEXT PRIMARY KEY,
  initial_invocation_id TEXT NOT NULL UNIQUE,
  initial_prompt TEXT NOT NULL,
  FOREIGN KEY(correction_id) REFERENCES product_decision_correction_conversations(correction_id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS product_decision_correction_acceptances (
  proposal_id TEXT PRIMARY KEY,
  human_interaction_id TEXT NOT NULL,
  idempotency_key TEXT NOT NULL UNIQUE,
  FOREIGN KEY(proposal_id) REFERENCES product_decision_correction_proposals(proposal_id) ON DELETE RESTRICT
);"#;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentPassage {
    kind: AgentPassageReferenceKind,
    session_id: String,
    invocation_id: String,
    passage: AgentPassageKind,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentPassageReferenceKind {
    AgentSessionPassage,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum AgentPassageKind {
    SubmittedInput,
    Outcome,
    Activity { runtime_event_id: String },
    FinalResponse { runtime_event_id: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ProductDecisionEvidenceOriginReference {
    HumanInteraction { opaque_id: String },
    AgentSessionCompleted { opaque_id: String },
    WorkUnitApproved { opaque_id: String },
    SprintCompleted { opaque_id: String },
    EpicCompleted { opaque_id: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum AcceptanceProvenance {
    /// The command itself is the explicit human acceptance; no reason is required.
    ManualHumanApplication {
        human_interaction_origin: ProductDecisionEvidenceOriginReference,
    },
    /// Agent material remains proposal-only until this explicit human acceptance command records it.
    AgentAssisted {
        human_interaction_origin: ProductDecisionEvidenceOriginReference,
        proposal_passage: AgentPassage,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurrentActionableEvidence {
    evidence_id: String,
    origin_reference: ProductDecisionEvidenceOriginReference,
    destination: AgentPassage,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoricalUnresolvedEvidence {
    evidence_id: String,
    origin_reference: ProductDecisionEvidenceOriginReference,
    label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AcceptProductDecisionVersionInput {
    decision_id: String,
    epic_id: String,
    expected_current_version: Option<i64>,
    idempotency_key: String,
    title: String,
    statement: String,
    intent: String,
    acceptance_provenance: AcceptanceProvenance,
    #[serde(default)]
    current_actionable_evidence: Vec<CurrentActionableEvidence>,
    #[serde(default)]
    historical_unresolved_evidence: Vec<HistoricalUnresolvedEvidence>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDecisionVersion {
    version_id: String,
    decision_id: String,
    epic_id: String,
    version: i64,
    title: String,
    statement: String,
    intent: String,
    acceptance_provenance: AcceptanceProvenance,
    current_actionable_evidence: Vec<CurrentActionableEvidence>,
    historical_unresolved_evidence: Vec<HistoricalUnresolvedEvidence>,
    accepted_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDecisionCurrent {
    decision_id: String,
    epic_id: String,
    current_version: ProductDecisionVersion,
    /// Current is official, but PD-2 deliberately makes no publication/application claim.
    application_state: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDecisionQuery {
    epic_id: String,
    decisions: Vec<ProductDecisionCurrent>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ProductDecisionError {
    InvalidInput,
    RevisionConflict,
    IdempotencyConflict,
    NotFound,
    Unavailable,
}
impl ProductDecisionError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::RevisionConflict => "revision_conflict",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::NotFound => "not_found",
            Self::Unavailable => "unavailable",
        }
    }
}

pub(crate) struct ProductDecisionRepository {
    connection: Mutex<Connection>,
}
pub(crate) struct ProductDecisionTauriState {
    repository: Arc<ProductDecisionRepository>,
    sessions: Arc<AgentSessionApplication>,
}
impl ProductDecisionTauriState {
    pub(crate) fn new(
        repository: Arc<ProductDecisionRepository>,
        sessions: Arc<AgentSessionApplication>,
    ) -> Self {
        Self {
            repository,
            sessions,
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDecisionCorrectionConversation {
    correction_id: String,
    epic_id: String,
    decision_id: String,
    base_version: i64,
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_proposal: Option<ProductDecisionCorrectionProposal>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDecisionCorrectionProposal {
    proposal_id: String,
    correction_id: String,
    title: String,
    statement: String,
    intent: String,
    proposal_passage: AgentPassage,
}

struct ProductDecisionCorrectionInitialization {
    initial_invocation_id: String,
    initial_prompt: String,
}

struct ProductDecisionCorrectionAcceptance {
    human_interaction_id: String,
    idempotency_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDecisionHistoryInput {
    epic_id: String,
    decision_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDecisionCurrentQueryInput {
    epic_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartProductDecisionCorrectionInput {
    epic_id: String,
    decision_id: String,
    base_version: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendProductDecisionCorrectionMessageInput {
    correction_id: String,
    submitted_text: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDecisionCorrectionMessageResult {
    session_id: String,
    invocation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveProductDecisionCorrectionProposalInput {
    correction_id: String,
    title: String,
    statement: String,
    intent: String,
    proposal_passage: AgentPassage,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AcceptProductDecisionCorrectionProposalInput {
    proposal_id: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDecisionTransportError {
    code: &'static str,
}
impl From<ProductDecisionError> for ProductDecisionTransportError {
    fn from(value: ProductDecisionError) -> Self {
        Self { code: value.code() }
    }
}
#[tauri::command]
pub(crate) fn accept_product_decision_version(
    state: State<'_, ProductDecisionTauriState>,
    input: AcceptProductDecisionVersionInput,
) -> Result<ProductDecisionVersion, ProductDecisionTransportError> {
    state.repository.accept(input).map_err(Into::into)
}
#[tauri::command]
pub(crate) fn load_product_decision_current_query(
    state: State<'_, ProductDecisionTauriState>,
    input: ProductDecisionCurrentQueryInput,
) -> Result<ProductDecisionQuery, ProductDecisionTransportError> {
    state.repository.query(&input.epic_id).map_err(Into::into)
}
#[tauri::command]
pub(crate) fn load_product_decision_history(
    state: State<'_, ProductDecisionTauriState>,
    input: ProductDecisionHistoryInput,
) -> Result<Vec<ProductDecisionVersion>, ProductDecisionTransportError> {
    state
        .repository
        .history(&input.epic_id, &input.decision_id)
        .map_err(Into::into)
}
/// Starts (or reopens) one durable, decision-bound Agent Session. The application supplies the
/// context; this command does not create a Product Decision version or proposal.
#[tauri::command]
pub(crate) fn start_product_decision_correction_conversation(
    state: State<'_, ProductDecisionTauriState>,
    input: StartProductDecisionCorrectionInput,
) -> Result<ProductDecisionCorrectionConversation, ProductDecisionTransportError> {
    start_product_decision_correction(&state, input)
}

fn start_product_decision_correction(
    state: &ProductDecisionTauriState,
    input: StartProductDecisionCorrectionInput,
) -> Result<ProductDecisionCorrectionConversation, ProductDecisionTransportError> {
    let session_id = AgentSessionId::new(format!("product-decision-correction-{}", Uuid::new_v4()))
        .map_err(|_| ProductDecisionTransportError {
            code: "unavailable",
        })?;
    let invocation_id = state.sessions.allocate_application_invocation_id();
    let prompt = correction_initial_prompt(&input.epic_id, &input.decision_id, input.base_version);
    let correction = state.repository.start_correction(
        &input.epic_id,
        &input.decision_id,
        input.base_version,
        session_id.as_str(),
        invocation_id.as_str(),
        &prompt,
    )?;
    ensure_correction_initialization(&state, &correction)?;
    Ok(correction)
}
#[tauri::command]
pub(crate) fn send_product_decision_correction_message(
    state: State<'_, ProductDecisionTauriState>,
    input: SendProductDecisionCorrectionMessageInput,
) -> Result<ProductDecisionCorrectionMessageResult, ProductDecisionTransportError> {
    send_product_decision_correction(&state, input)
}

fn send_product_decision_correction(
    state: &ProductDecisionTauriState,
    input: SendProductDecisionCorrectionMessageInput,
) -> Result<ProductDecisionCorrectionMessageResult, ProductDecisionTransportError> {
    let correction = state.repository.load_correction(&input.correction_id)?;
    ensure_correction_initialization(&state, &correction)?;
    let session_id = correction.session_id;
    let result = state
        .sessions
        .send_message(SendAgentSessionMessageCommand {
            session_id: Some(AgentSessionId::new(session_id).map_err(|_| {
                ProductDecisionTransportError {
                    code: "invalid_input",
                }
            })?),
            submitted_text: input.submitted_text,
            title: None,
            working_directory: None,
            requested_options: None,
        })
        .map_err(|_| ProductDecisionTransportError {
            code: "unavailable",
        })?;
    Ok(ProductDecisionCorrectionMessageResult {
        session_id: result.session_id.as_str().into(),
        invocation_id: result.invocation_id.as_str().into(),
    })
}
#[tauri::command]
pub(crate) fn save_product_decision_correction_proposal(
    state: State<'_, ProductDecisionTauriState>,
    input: SaveProductDecisionCorrectionProposalInput,
) -> Result<ProductDecisionCorrectionProposal, ProductDecisionTransportError> {
    retain_product_decision_correction_proposal(&state, input)
}

fn retain_product_decision_correction_proposal(
    state: &ProductDecisionTauriState,
    input: SaveProductDecisionCorrectionProposalInput,
) -> Result<ProductDecisionCorrectionProposal, ProductDecisionTransportError> {
    state
        .repository
        .save_correction_proposal(input)
        .map_err(Into::into)
}
#[tauri::command]
pub(crate) fn accept_product_decision_correction_proposal(
    state: State<'_, ProductDecisionTauriState>,
    input: AcceptProductDecisionCorrectionProposalInput,
) -> Result<ProductDecisionVersion, ProductDecisionTransportError> {
    accept_product_decision_correction(&state, input)
}

fn accept_product_decision_correction(
    state: &ProductDecisionTauriState,
    input: AcceptProductDecisionCorrectionProposalInput,
) -> Result<ProductDecisionVersion, ProductDecisionTransportError> {
    state
        .repository
        .accept_correction_proposal(input)
        .map_err(Into::into)
}

fn correction_initial_prompt(epic_id: &str, decision_id: &str, base_version: i64) -> String {
    format!(
        "You are assisting with a proposed correction to one Product Decision. This conversation is bound to Epic {epic_id} and decision {decision_id} at version {base_version}. Discuss the correction only. Your responses are proposal material only: do not claim acceptance, publication, application, or changes to orchestration. When ready, clearly propose a title, statement, and intent for the user to review."
    )
}

/// Restores the one application-owned context prompt before ordinary user continuation. The
/// invocation identity and content are stored with the correction, so a reopen never creates a
/// replacement session or silently treats a merely persisted prompt as launched.
fn ensure_correction_initialization(
    state: &ProductDecisionTauriState,
    correction: &ProductDecisionCorrectionConversation,
) -> Result<(), ProductDecisionTransportError> {
    let initialization = state
        .repository
        .correction_initialization(&correction.correction_id)?;
    let session_id = AgentSessionId::new(correction.session_id.clone()).map_err(|_| {
        ProductDecisionTransportError {
            code: "invalid_input",
        }
    })?;
    let invocation_id =
        AgentInvocationId::new(initialization.initial_invocation_id).map_err(|_| {
            ProductDecisionTransportError {
                code: "invalid_input",
            }
        })?;
    state
        .sessions
        .create_application_session(CreateApplicationAgentSessionCommand {
            session_id: session_id.clone(),
            session: CreateAgentSessionCommand {
                title: Some("Product Decision correction".into()),
                working_directory: None,
                requested_options: AgentRuntimeOptions::default(),
            },
        })
        .map_err(|_| ProductDecisionTransportError {
            code: "unavailable",
        })?;
    let command = SendIdempotentApplicationAgentSessionMessageCommand {
        invocation_id: invocation_id.clone(),
        message: SendAgentSessionMessageCommand {
            session_id: Some(session_id.clone()),
            submitted_text: initialization.initial_prompt,
            title: None,
            working_directory: None,
            requested_options: None,
        },
    };
    match state
        .sessions
        .application_invocation_launch_evidence(&invocation_id, &session_id)
        .map_err(|_| ProductDecisionTransportError {
            code: "unavailable",
        })? {
        ApplicationInvocationLaunchEvidence::LaunchAccepted => Ok(()),
        ApplicationInvocationLaunchEvidence::NeverPersisted => {
            state
                .sessions
                .prepare_idempotent_application_invocation(command.clone())
                .map_err(|_| ProductDecisionTransportError {
                    code: "unavailable",
                })?;
            let launch = state
                .sessions
                .launch_prepared_application_invocation_with_launch_observation(command, None)
                .map_err(|_| ProductDecisionTransportError {
                    code: "unavailable",
                })?;
            if launch.launch_accepted {
                Ok(())
            } else {
                Err(ProductDecisionTransportError {
                    code: "unavailable",
                })
            }
        }
        ApplicationInvocationLaunchEvidence::PersistedNotAccepted => {
            let launch = state
                .sessions
                .launch_prepared_application_invocation_with_launch_observation(command, None)
                .map_err(|_| ProductDecisionTransportError {
                    code: "unavailable",
                })?;
            if launch.launch_accepted {
                Ok(())
            } else {
                Err(ProductDecisionTransportError {
                    code: "unavailable",
                })
            }
        }
    }
}
impl ProductDecisionRepository {
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, ProductDecisionError> {
        let connection = Connection::open(path).map_err(|_| ProductDecisionError::Unavailable)?;
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(|_| ProductDecisionError::Unavailable)?;
        connection
            .execute_batch(PRODUCT_DECISION_SCHEMA)
            .map_err(|_| ProductDecisionError::Unavailable)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub(crate) fn accept(
        &self,
        input: AcceptProductDecisionVersionInput,
    ) -> Result<ProductDecisionVersion, ProductDecisionError> {
        validate_input(&input)?;
        let fingerprint =
            serde_json::to_string(&input).map_err(|_| ProductDecisionError::InvalidInput)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ProductDecisionError::Unavailable)?;
        if let Some((existing_fingerprint, version_id)) = transaction.query_row("SELECT payload_fingerprint,version_id FROM product_decision_acceptance_commands WHERE idempotency_key=?1", [&input.idempotency_key], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional().map_err(|_| ProductDecisionError::Unavailable)? {
            return if existing_fingerprint == fingerprint { load_version(&transaction, &version_id) } else { Err(ProductDecisionError::IdempotencyConflict) };
        }
        validate_acceptance_provenance(&transaction, &input.acceptance_provenance)?;
        for evidence in &input.current_actionable_evidence {
            validate_origin(&evidence.origin_reference)?;
            validate_passage(&transaction, &evidence.destination)?;
        }
        for evidence in &input.historical_unresolved_evidence {
            validate_origin(&evidence.origin_reference)?;
        }
        let existing: Option<(String, i64)> = transaction
            .query_row(
                "SELECT epic_id,current_version FROM product_decisions WHERE decision_id=?1",
                [&input.decision_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let next_version = match existing {
            None if input.expected_current_version.is_none() => {
                let now = Utc::now().to_rfc3339();
                transaction.execute("INSERT INTO product_decisions(decision_id,epic_id,current_version,created_at,updated_at) VALUES(?1,?2,1,?3,?3)", params![input.decision_id,input.epic_id,now]).map_err(|_| ProductDecisionError::Unavailable)?;
                1
            }
            Some((epic, current))
                if epic == input.epic_id && input.expected_current_version == Some(current) =>
            {
                current + 1
            }
            _ => return Err(ProductDecisionError::RevisionConflict),
        };
        let version_id = format!("product-decision-version-{}", Uuid::new_v4());
        let accepted_at = Utc::now().to_rfc3339();
        let provenance = serde_json::to_string(&input.acceptance_provenance)
            .map_err(|_| ProductDecisionError::InvalidInput)?;
        transaction.execute("INSERT INTO product_decision_versions(version_id,decision_id,version,title,statement,intent,acceptance_provenance_json,accepted_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)", params![version_id,input.decision_id,next_version,input.title,input.statement,input.intent,provenance,accepted_at]).map_err(|_| ProductDecisionError::Unavailable)?;
        for evidence in &input.current_actionable_evidence {
            insert_evidence(
                &transaction,
                &version_id,
                &evidence.evidence_id,
                "current_agent_passage",
                evidence,
            )?;
        }
        for evidence in &input.historical_unresolved_evidence {
            insert_evidence(
                &transaction,
                &version_id,
                &evidence.evidence_id,
                "historical_unresolved",
                evidence,
            )?;
        }
        transaction.execute("UPDATE product_decisions SET current_version=?2,updated_at=?3 WHERE decision_id=?1", params![input.decision_id,next_version,accepted_at]).map_err(|_| ProductDecisionError::Unavailable)?;
        transaction.execute("INSERT INTO product_decision_acceptance_commands(idempotency_key,payload_fingerprint,version_id) VALUES(?1,?2,?3)", params![input.idempotency_key,fingerprint,version_id]).map_err(|_| ProductDecisionError::Unavailable)?;
        let version = load_version(&transaction, &version_id)?;
        transaction
            .commit()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        Ok(version)
    }

    pub(crate) fn query(
        &self,
        epic_id: &str,
    ) -> Result<ProductDecisionQuery, ProductDecisionError> {
        if epic_id.trim().is_empty() {
            return Err(ProductDecisionError::InvalidInput);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let mut statement = connection.prepare("SELECT d.decision_id,d.epic_id,v.version_id FROM product_decisions d JOIN product_decision_versions v ON v.decision_id=d.decision_id AND v.version=d.current_version WHERE d.epic_id=?1 ORDER BY d.decision_id").map_err(|_| ProductDecisionError::Unavailable)?;
        let rows = statement
            .query_map([epic_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let mut decisions = Vec::new();
        for row in rows {
            let (decision_id, epic_id, version_id) =
                row.map_err(|_| ProductDecisionError::Unavailable)?;
            decisions.push(ProductDecisionCurrent {
                decision_id,
                epic_id,
                current_version: load_version(&connection, &version_id)?,
                application_state: "not_applied",
            });
        }
        Ok(ProductDecisionQuery {
            epic_id: epic_id.into(),
            decisions,
        })
    }

    pub(crate) fn history(
        &self,
        epic_id: &str,
        decision_id: &str,
    ) -> Result<Vec<ProductDecisionVersion>, ProductDecisionError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let exists: Option<()> = connection
            .query_row(
                "SELECT 1 FROM product_decisions WHERE decision_id=?1 AND epic_id=?2",
                params![decision_id, epic_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        if exists.is_none() {
            return Err(ProductDecisionError::NotFound);
        }
        let mut statement = connection.prepare("SELECT version_id FROM product_decision_versions WHERE decision_id=?1 ORDER BY version").map_err(|_| ProductDecisionError::Unavailable)?;
        let ids = statement
            .query_map([decision_id], |r| r.get::<_, String>(0))
            .map_err(|_| ProductDecisionError::Unavailable)?;
        ids.map(|id| {
            load_version(
                &connection,
                &id.map_err(|_| ProductDecisionError::Unavailable)?,
            )
        })
        .collect()
    }

    fn start_correction(
        &self,
        epic_id: &str,
        decision_id: &str,
        base_version: i64,
        session_id: &str,
        initial_invocation_id: &str,
        initial_prompt: &str,
    ) -> Result<ProductDecisionCorrectionConversation, ProductDecisionError> {
        if epic_id.trim().is_empty()
            || decision_id.trim().is_empty()
            || session_id.trim().is_empty()
            || initial_invocation_id.trim().is_empty()
            || initial_prompt.trim().is_empty()
            || base_version < 1
        {
            return Err(ProductDecisionError::InvalidInput);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let current: Option<i64> = transaction
            .query_row(
                "SELECT current_version FROM product_decisions WHERE epic_id=?1 AND decision_id=?2",
                params![epic_id, decision_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        if current != Some(base_version) {
            return Err(ProductDecisionError::RevisionConflict);
        }
        if let Some(mut existing) = transaction
            .query_row(
                "SELECT correction_id,epic_id,decision_id,base_version,session_id FROM product_decision_correction_conversations WHERE epic_id=?1 AND decision_id=?2 AND base_version=?3",
                params![epic_id, decision_id, base_version],
                correction_from_row,
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?
        {
            existing.latest_proposal = load_latest_correction_proposal(&transaction, &existing.correction_id)?;
            return Ok(existing);
        }
        let correction = ProductDecisionCorrectionConversation {
            correction_id: format!("product-decision-correction-{}", Uuid::new_v4()),
            epic_id: epic_id.into(),
            decision_id: decision_id.into(),
            base_version,
            session_id: session_id.into(),
            latest_proposal: None,
        };
        transaction
            .execute(
                "INSERT INTO product_decision_correction_conversations(correction_id,epic_id,decision_id,base_version,session_id,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
                params![correction.correction_id, correction.epic_id, correction.decision_id, correction.base_version, correction.session_id, Utc::now().to_rfc3339()],
            )
            .map_err(|_| ProductDecisionError::Unavailable)?;
        transaction
            .execute(
                "INSERT INTO product_decision_correction_initializations(correction_id,initial_invocation_id,initial_prompt) VALUES(?1,?2,?3)",
                params![correction.correction_id, initial_invocation_id, initial_prompt],
            )
            .map_err(|_| ProductDecisionError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        Ok(correction)
    }

    fn load_correction(
        &self,
        correction_id: &str,
    ) -> Result<ProductDecisionCorrectionConversation, ProductDecisionError> {
        self.connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?
            .query_row(
                "SELECT correction_id,epic_id,decision_id,base_version,session_id FROM product_decision_correction_conversations WHERE correction_id=?1",
                [correction_id],
                correction_from_row,
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?
            .ok_or(ProductDecisionError::NotFound)
    }

    fn correction_initialization(
        &self,
        correction_id: &str,
    ) -> Result<ProductDecisionCorrectionInitialization, ProductDecisionError> {
        self.connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?
            .query_row(
                "SELECT initial_invocation_id,initial_prompt FROM product_decision_correction_initializations WHERE correction_id=?1",
                [correction_id],
                |row| Ok(ProductDecisionCorrectionInitialization { initial_invocation_id: row.get(0)?, initial_prompt: row.get(1)? }),
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?
            .ok_or(ProductDecisionError::Unavailable)
    }

    fn correction_acceptance(
        &self,
        proposal_id: &str,
    ) -> Result<ProductDecisionCorrectionAcceptance, ProductDecisionError> {
        self.connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?
            .query_row(
                "SELECT human_interaction_id,idempotency_key FROM product_decision_correction_acceptances WHERE proposal_id=?1",
                [proposal_id],
                |row| Ok(ProductDecisionCorrectionAcceptance { human_interaction_id: row.get(0)?, idempotency_key: row.get(1)? }),
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?
            .ok_or(ProductDecisionError::Unavailable)
    }

    fn save_correction_proposal(
        &self,
        input: SaveProductDecisionCorrectionProposalInput,
    ) -> Result<ProductDecisionCorrectionProposal, ProductDecisionError> {
        if input.correction_id.trim().is_empty()
            || input.title.trim().is_empty()
            || input.statement.trim().is_empty()
            || input.intent.trim().is_empty()
            || !matches!(
                input.proposal_passage.passage,
                AgentPassageKind::FinalResponse { .. }
            )
        {
            return Err(ProductDecisionError::InvalidInput);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let session_id: String = transaction
            .query_row(
                "SELECT session_id FROM product_decision_correction_conversations WHERE correction_id=?1",
                [&input.correction_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?
            .ok_or(ProductDecisionError::NotFound)?;
        if input.proposal_passage.session_id != session_id {
            return Err(ProductDecisionError::InvalidInput);
        }
        validate_passage(&transaction, &input.proposal_passage)?;
        let proposal = ProductDecisionCorrectionProposal {
            proposal_id: format!("product-decision-correction-proposal-{}", Uuid::new_v4()),
            correction_id: input.correction_id,
            title: input.title.trim().into(),
            statement: input.statement.trim().into(),
            intent: input.intent.trim().into(),
            proposal_passage: input.proposal_passage,
        };
        transaction
            .execute(
                "INSERT INTO product_decision_correction_proposals(proposal_id,correction_id,title,statement,intent,proposal_passage_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![proposal.proposal_id, proposal.correction_id, proposal.title, proposal.statement, proposal.intent, serde_json::to_string(&proposal.proposal_passage).map_err(|_| ProductDecisionError::InvalidInput)?, Utc::now().to_rfc3339()],
            )
            .map_err(|_| ProductDecisionError::Unavailable)?;
        transaction
            .execute(
                "INSERT INTO product_decision_correction_acceptances(proposal_id,human_interaction_id,idempotency_key) VALUES(?1,?2,?3)",
                params![
                    proposal.proposal_id,
                    format!("product-decision-human-acceptance-{}", Uuid::new_v4()),
                    format!("product-decision-agent-correction-{}", Uuid::new_v4()),
                ],
            )
            .map_err(|_| ProductDecisionError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        Ok(proposal)
    }

    fn accept_correction_proposal(
        &self,
        input: AcceptProductDecisionCorrectionProposalInput,
    ) -> Result<ProductDecisionVersion, ProductDecisionError> {
        if input.proposal_id.trim().is_empty() {
            return Err(ProductDecisionError::InvalidInput);
        }
        let (proposal, correction) = self.load_correction_proposal(&input.proposal_id)?;
        let acceptance = self.correction_acceptance(&proposal.proposal_id)?;
        if let Some(existing) = self.replay_correction_acceptance(&acceptance, &proposal)? {
            return Ok(existing);
        }
        let current = self
            .query(&correction.epic_id)?
            .decisions
            .into_iter()
            .find(|decision| decision.decision_id == correction.decision_id)
            .ok_or(ProductDecisionError::NotFound)?;
        let mut evidence = current.current_version.current_actionable_evidence;
        let proposal_evidence_id = format!("agent-assisted-proposal:{}", proposal.proposal_id);
        if !evidence
            .iter()
            .any(|item| item.evidence_id == proposal_evidence_id)
        {
            evidence.push(CurrentActionableEvidence {
                evidence_id: proposal_evidence_id,
                origin_reference: ProductDecisionEvidenceOriginReference::HumanInteraction {
                    opaque_id: acceptance.human_interaction_id.clone(),
                },
                destination: proposal.proposal_passage.clone(),
            });
        }
        self.accept(AcceptProductDecisionVersionInput {
            decision_id: correction.decision_id,
            epic_id: correction.epic_id,
            expected_current_version: Some(correction.base_version),
            idempotency_key: acceptance.idempotency_key,
            title: proposal.title,
            statement: proposal.statement,
            intent: proposal.intent,
            acceptance_provenance: AcceptanceProvenance::AgentAssisted {
                human_interaction_origin:
                    ProductDecisionEvidenceOriginReference::HumanInteraction {
                        opaque_id: acceptance.human_interaction_id,
                    },
                proposal_passage: proposal.proposal_passage,
            },
            current_actionable_evidence: evidence,
            historical_unresolved_evidence: current.current_version.historical_unresolved_evidence,
        })
    }

    fn replay_correction_acceptance(
        &self,
        acceptance: &ProductDecisionCorrectionAcceptance,
        proposal: &ProductDecisionCorrectionProposal,
    ) -> Result<Option<ProductDecisionVersion>, ProductDecisionError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let version_id: Option<String> = connection
            .query_row(
                "SELECT version_id FROM product_decision_acceptance_commands WHERE idempotency_key=?1",
                [&acceptance.idempotency_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let Some(version_id) = version_id else {
            return Ok(None);
        };
        let version = load_version(&connection, &version_id)?;
        match &version.acceptance_provenance {
            AcceptanceProvenance::AgentAssisted {
                human_interaction_origin:
                    ProductDecisionEvidenceOriginReference::HumanInteraction { opaque_id },
                proposal_passage,
            } if opaque_id == &acceptance.human_interaction_id
                && proposal_passage == &proposal.proposal_passage
                && version.title == proposal.title
                && version.statement == proposal.statement
                && version.intent == proposal.intent
                && version.current_actionable_evidence.iter().any(|evidence| {
                    evidence.evidence_id
                        == format!("agent-assisted-proposal:{}", proposal.proposal_id)
                }) =>
            {
                Ok(Some(version))
            }
            _ => Err(ProductDecisionError::IdempotencyConflict),
        }
    }

    fn load_correction_proposal(
        &self,
        proposal_id: &str,
    ) -> Result<
        (
            ProductDecisionCorrectionProposal,
            ProductDecisionCorrectionConversation,
        ),
        ProductDecisionError,
    > {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        connection
            .query_row(
                "SELECT p.proposal_id,p.correction_id,p.title,p.statement,p.intent,p.proposal_passage_json,c.correction_id,c.epic_id,c.decision_id,c.base_version,c.session_id FROM product_decision_correction_proposals p JOIN product_decision_correction_conversations c ON c.correction_id=p.correction_id WHERE p.proposal_id=?1",
                [proposal_id],
                |row| {
                    let passage: String = row.get(5)?;
                    Ok((
                        ProductDecisionCorrectionProposal {
                            proposal_id: row.get(0)?,
                            correction_id: row.get(1)?,
                            title: row.get(2)?,
                            statement: row.get(3)?,
                            intent: row.get(4)?,
                            proposal_passage: serde_json::from_str(&passage).map_err(|_| rusqlite::Error::InvalidQuery)?,
                        },
                        ProductDecisionCorrectionConversation {
                            correction_id: row.get(6)?,
                            epic_id: row.get(7)?,
                            decision_id: row.get(8)?,
                            base_version: row.get(9)?,
                            session_id: row.get(10)?,
                            latest_proposal: None,
                        },
                    ))
                },
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?
            .ok_or(ProductDecisionError::NotFound)
    }
}

fn load_latest_correction_proposal(
    connection: &Connection,
    correction_id: &str,
) -> Result<Option<ProductDecisionCorrectionProposal>, ProductDecisionError> {
    connection
        .query_row(
            "SELECT proposal_id,correction_id,title,statement,intent,proposal_passage_json FROM product_decision_correction_proposals WHERE correction_id=?1 ORDER BY created_at DESC,proposal_id DESC LIMIT 1",
            [correction_id],
            |row| {
                let passage: String = row.get(5)?;
                Ok(ProductDecisionCorrectionProposal {
                    proposal_id: row.get(0)?,
                    correction_id: row.get(1)?,
                    title: row.get(2)?,
                    statement: row.get(3)?,
                    intent: row.get(4)?,
                    proposal_passage: serde_json::from_str(&passage)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                })
            },
        )
        .optional()
        .map_err(|_| ProductDecisionError::Unavailable)
}

fn correction_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProductDecisionCorrectionConversation> {
    Ok(ProductDecisionCorrectionConversation {
        correction_id: row.get(0)?,
        epic_id: row.get(1)?,
        decision_id: row.get(2)?,
        base_version: row.get(3)?,
        session_id: row.get(4)?,
        latest_proposal: None,
    })
}

fn insert_evidence<T: Serialize>(
    connection: &rusqlite::Transaction<'_>,
    version_id: &str,
    evidence_id: &str,
    kind: &str,
    value: &T,
) -> Result<(), ProductDecisionError> {
    connection.execute("INSERT INTO product_decision_evidence(version_id,evidence_id,evidence_kind,evidence_json) VALUES(?1,?2,?3,?4)", params![version_id,evidence_id,kind,serde_json::to_string(value).map_err(|_| ProductDecisionError::InvalidInput)?]).map_err(|_| ProductDecisionError::Unavailable)?;
    Ok(())
}
fn load_version(
    connection: &Connection,
    version_id: &str,
) -> Result<ProductDecisionVersion, ProductDecisionError> {
    let (decision_id,version,title,statement,intent,provenance,accepted_at):(String,i64,String,String,String,String,String)=connection.query_row("SELECT decision_id,version,title,statement,intent,acceptance_provenance_json,accepted_at FROM product_decision_versions WHERE version_id=?1",[version_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).map_err(|_| ProductDecisionError::NotFound)?;
    let epic_id = connection
        .query_row(
            "SELECT epic_id FROM product_decisions WHERE decision_id=?1",
            [&decision_id],
            |r| r.get(0),
        )
        .map_err(|_| ProductDecisionError::Unavailable)?;
    let mut current_actionable_evidence = Vec::new();
    let mut historical_unresolved_evidence = Vec::new();
    let mut statement_query=connection.prepare("SELECT evidence_kind,evidence_json FROM product_decision_evidence WHERE version_id=?1 ORDER BY evidence_id").map_err(|_| ProductDecisionError::Unavailable)?;
    let evidence = statement_query
        .query_map([version_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|_| ProductDecisionError::Unavailable)?;
    for item in evidence {
        let (kind, json) = item.map_err(|_| ProductDecisionError::Unavailable)?;
        match kind.as_str() {
            "current_agent_passage" => current_actionable_evidence
                .push(serde_json::from_str(&json).map_err(|_| ProductDecisionError::Unavailable)?),
            "historical_unresolved" => historical_unresolved_evidence
                .push(serde_json::from_str(&json).map_err(|_| ProductDecisionError::Unavailable)?),
            _ => return Err(ProductDecisionError::Unavailable),
        }
    }
    Ok(ProductDecisionVersion {
        version_id: version_id.into(),
        decision_id,
        epic_id,
        version,
        title,
        statement,
        intent,
        acceptance_provenance: serde_json::from_str(&provenance)
            .map_err(|_| ProductDecisionError::Unavailable)?,
        current_actionable_evidence,
        historical_unresolved_evidence,
        accepted_at,
    })
}
fn validate_input(input: &AcceptProductDecisionVersionInput) -> Result<(), ProductDecisionError> {
    for value in [
        &input.decision_id,
        &input.epic_id,
        &input.idempotency_key,
        &input.title,
        &input.statement,
        &input.intent,
    ] {
        if value.trim().is_empty() {
            return Err(ProductDecisionError::InvalidInput);
        }
    }
    if input
        .expected_current_version
        .is_some_and(|value| value < 1)
    {
        return Err(ProductDecisionError::InvalidInput);
    }
    let mut ids = std::collections::HashSet::new();
    for evidence in &input.current_actionable_evidence {
        if evidence.evidence_id.trim().is_empty() || !ids.insert(&evidence.evidence_id) {
            return Err(ProductDecisionError::InvalidInput);
        }
    }
    for evidence in &input.historical_unresolved_evidence {
        if evidence.evidence_id.trim().is_empty()
            || evidence.label.trim().is_empty()
            || !ids.insert(&evidence.evidence_id)
        {
            return Err(ProductDecisionError::InvalidInput);
        }
    }
    Ok(())
}
fn validate_origin(
    origin: &ProductDecisionEvidenceOriginReference,
) -> Result<(), ProductDecisionError> {
    let opaque_id = match origin {
        ProductDecisionEvidenceOriginReference::HumanInteraction { opaque_id }
        | ProductDecisionEvidenceOriginReference::AgentSessionCompleted { opaque_id }
        | ProductDecisionEvidenceOriginReference::WorkUnitApproved { opaque_id }
        | ProductDecisionEvidenceOriginReference::SprintCompleted { opaque_id }
        | ProductDecisionEvidenceOriginReference::EpicCompleted { opaque_id } => opaque_id,
    };
    if opaque_id.trim().is_empty() {
        return Err(ProductDecisionError::InvalidInput);
    }
    Ok(())
}
fn validate_human_acceptance_origin(
    origin: &ProductDecisionEvidenceOriginReference,
) -> Result<(), ProductDecisionError> {
    if !matches!(
        origin,
        ProductDecisionEvidenceOriginReference::HumanInteraction { .. }
    ) {
        return Err(ProductDecisionError::InvalidInput);
    }
    validate_origin(origin)
}
fn validate_acceptance_provenance(
    connection: &rusqlite::Transaction<'_>,
    provenance: &AcceptanceProvenance,
) -> Result<(), ProductDecisionError> {
    match provenance {
        AcceptanceProvenance::ManualHumanApplication {
            human_interaction_origin,
        } => validate_human_acceptance_origin(human_interaction_origin),
        AcceptanceProvenance::AgentAssisted {
            human_interaction_origin,
            proposal_passage,
        } => {
            validate_human_acceptance_origin(human_interaction_origin)?;
            validate_passage(connection, proposal_passage)
        }
    }
}
fn validate_passage(
    connection: &rusqlite::Transaction<'_>,
    passage: &AgentPassage,
) -> Result<(), ProductDecisionError> {
    if passage.kind != AgentPassageReferenceKind::AgentSessionPassage
        || passage.session_id.trim().is_empty()
        || passage.invocation_id.trim().is_empty()
    {
        return Err(ProductDecisionError::InvalidInput);
    }
    let exists: Option<()> = connection
        .query_row(
            "SELECT 1 FROM agent_session_invocations WHERE id=?1 AND session_id=?2",
            params![passage.invocation_id, passage.session_id],
            |_| Ok(()),
        )
        .optional()
        .map_err(|_| ProductDecisionError::Unavailable)?;
    if exists.is_none() {
        return Err(ProductDecisionError::InvalidInput);
    }
    if let AgentPassageKind::Activity { runtime_event_id }
    | AgentPassageKind::FinalResponse { runtime_event_id } = &passage.passage
    {
        if runtime_event_id.trim().is_empty() {
            return Err(ProductDecisionError::InvalidInput);
        }
        let normalized_json: Option<String> = connection
            .query_row(
                "SELECT normalized_json FROM agent_session_runtime_events WHERE id=?1 AND invocation_id=?2 AND normalized_json IS NOT NULL",
                params![runtime_event_id, passage.invocation_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let normalized: serde_json::Value = normalized_json
            .as_deref()
            .and_then(|value| serde_json::from_str(value).ok())
            .ok_or(ProductDecisionError::InvalidInput)?;
        let supported = match &passage.passage {
            AgentPassageKind::Activity { .. } => {
                normalized["kind"] == "tool_activity" && normalized["toolActivity"].is_object()
            }
            AgentPassageKind::FinalResponse { .. } => {
                normalized["kind"] == "agent_message" && normalized["details"]["role"] == "final"
            }
            _ => unreachable!("only event-backed passage kinds enter this branch"),
        };
        if !supported {
            return Err(ProductDecisionError::InvalidInput);
        }
    }
    if matches!(passage.passage, AgentPassageKind::Outcome) {
        let terminal: Option<()> = connection
            .query_row(
                "SELECT 1 FROM agent_session_invocations WHERE id=?1 AND session_id=?2 AND status IN ('completed','failed','canceled','interrupted') AND completed_at IS NOT NULL",
                params![passage.invocation_id, passage.session_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        if terminal.is_none() {
            return Err(ProductDecisionError::InvalidInput);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_sessions::{
        application::{
            AgentSessionNotification, AgentSessionNotifier, SystemAgentSessionProviders,
        },
        ports::{
            AgentRuntime, AgentRuntimeUpdateSink, RuntimeInvocationMode,
            RuntimeInvocationPreflight, RuntimeInvocationRequest, RuntimePortError,
        },
        repository::SqliteAgentSessionRepository,
    };
    #[cfg(feature = "live-tests")]
    use crate::runtime::codex::CodexCliRuntime;
    #[cfg(feature = "live-tests")]
    use crate::runtime::processes::ProcessLaunchSpec;
    #[cfg(feature = "live-tests")]
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    struct NoopNotifier;
    impl AgentSessionNotifier for NoopNotifier {
        fn notify(&self, _notification: AgentSessionNotification) -> Result<(), String> {
            Ok(())
        }
    }

    struct LaunchAcceptedRuntime;
    impl AgentRuntime for LaunchAcceptedRuntime {
        fn preflight_invocation(
            &self,
            _mode: RuntimeInvocationMode,
            _requested_options: &AgentRuntimeOptions,
        ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
            Ok(RuntimeInvocationPreflight {
                effective_options: AgentRuntimeOptions::default(),
            })
        }

        fn start_invocation(
            &self,
            _request: RuntimeInvocationRequest,
            _update_sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            Ok(())
        }

        fn resume_invocation(
            &self,
            request: RuntimeInvocationRequest,
            _external_context_id: crate::agent_sessions::domain::ExternalRuntimeContextId,
            update_sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.start_invocation(request, update_sink)
        }

        fn cancel_invocation(
            &self,
            _invocation_id: &AgentInvocationId,
        ) -> Result<(), RuntimePortError> {
            Ok(())
        }
    }

    fn command_state(path: &Path) -> ProductDecisionTauriState {
        let connection = crate::storage::open_active_database(path).unwrap();
        let session_repository = Arc::new(SqliteAgentSessionRepository::new(connection).unwrap());
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            session_repository,
            Arc::new(LaunchAcceptedRuntime),
            Arc::new(NoopNotifier),
            providers.clone(),
            providers,
            Some("test-runtime".into()),
        ));
        ProductDecisionTauriState::new(
            Arc::new(ProductDecisionRepository::open(path).unwrap()),
            sessions,
        )
    }

    #[cfg(feature = "live-tests")]
    fn live_command_state(
        path: &Path,
        launches: Arc<Mutex<Vec<ProcessLaunchSpec>>>,
    ) -> ProductDecisionTauriState {
        let connection = crate::storage::open_active_database(path).unwrap();
        let session_repository = Arc::new(SqliteAgentSessionRepository::new(connection).unwrap());
        let providers = Arc::new(SystemAgentSessionProviders);
        let runtime = Arc::new(CodexCliRuntime::system("codex", None).with_launch_observer(
            Arc::new(move |spec| {
                launches.lock().unwrap().push(spec.clone());
            }),
        ));
        let sessions = Arc::new(AgentSessionApplication::new(
            session_repository,
            runtime,
            Arc::new(NoopNotifier),
            providers.clone(),
            providers,
            None,
        ));
        ProductDecisionTauriState::new(
            Arc::new(ProductDecisionRepository::open(path).unwrap()),
            sessions,
        )
    }

    #[cfg(feature = "live-tests")]
    fn wait_for_terminal_invocation(
        state: &ProductDecisionTauriState,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
    ) -> Result<crate::agent_sessions::ports::AgentSessionHistory, String> {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let history = state
                .sessions
                .load_session(session_id)
                .map_err(|error| error.to_string())?;
            let invocation = history
                .invocations
                .iter()
                .find(|item| item.invocation.id == *invocation_id)
                .ok_or_else(|| "live Product Decision invocation disappeared".to_string())?;
            if invocation.invocation.status.is_terminal() {
                return if invocation.invocation.status
                    == crate::agent_sessions::domain::AgentInvocationStatus::Completed
                {
                    Ok(history)
                } else {
                    Err(format!(
                        "live Product Decision invocation terminal failure: status={:?}, error={:?}",
                        invocation.invocation.status, invocation.invocation.runtime_error
                    ))
                };
            }
            if Instant::now() >= deadline {
                return Err("timed out waiting for a live Product Decision invocation".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn seed_command_decision(repository: &ProductDecisionRepository) -> ProductDecisionVersion {
        let mut seeded = input("command-seed", None);
        seeded.acceptance_provenance = AcceptanceProvenance::ManualHumanApplication {
            human_interaction_origin: ProductDecisionEvidenceOriginReference::HumanInteraction {
                opaque_id: "test-human".into(),
            },
        };
        seeded.current_actionable_evidence.clear();
        seeded.historical_unresolved_evidence = vec![HistoricalUnresolvedEvidence {
            evidence_id: "prior-live-evidence".into(),
            origin_reference: ProductDecisionEvidenceOriginReference::HumanInteraction {
                opaque_id: "prior-live-human".into(),
            },
            label: "Retained isolated live evidence".into(),
        }];
        repository.accept(seeded).unwrap()
    }
    fn repo() -> (tempfile::TempDir, ProductDecisionRepository) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("active.sqlite");
        let connection = crate::storage::open_active_database(&path).unwrap();
        connection.execute("INSERT INTO agent_sessions(id,title,availability,requested_options_json,created_at,updated_at) VALUES('session','s','available','{}','t','t')",[]).unwrap();
        connection.execute("INSERT INTO agent_session_invocations(id,session_id,submitted_text,input_provenance,status,requested_options_json,started_at,completed_at,created_at,updated_at) VALUES('invocation','session','text','user','completed','{}','t','t','t','t')",[]).unwrap();
        connection.execute("INSERT INTO agent_session_runtime_events(id,invocation_id,sequence,source,raw_payload_json,normalized_json,recorded_at) VALUES('event','invocation',0,'runtime','{}',?1,'t'),('activity','invocation',1,'runtime','{}',?2,'t')", [r#"{"kind":"agent_message","text":"done","details":{"role":"final"},"toolActivity":null}"#, r#"{"kind":"tool_activity","text":"tool","details":{"itemType":"mcp_tool_call"},"toolActivity":{"phase":"completed","itemId":"item","server":"server","tool":"tool","status":null,"resultClassification":"succeeded"}}"#]).unwrap();
        drop(connection);
        (dir, ProductDecisionRepository::open(path).unwrap())
    }
    fn input(key: &str, expected: Option<i64>) -> AcceptProductDecisionVersionInput {
        AcceptProductDecisionVersionInput {
            decision_id: "decision".into(),
            epic_id: "epic".into(),
            expected_current_version: expected,
            idempotency_key: key.into(),
            title: "Title".into(),
            statement: "Statement".into(),
            intent: "Intent".into(),
            acceptance_provenance: AcceptanceProvenance::AgentAssisted {
                human_interaction_origin:
                    ProductDecisionEvidenceOriginReference::HumanInteraction {
                        opaque_id: "human-acceptance".into(),
                    },
                proposal_passage: AgentPassage {
                    kind: AgentPassageReferenceKind::AgentSessionPassage,
                    session_id: "session".into(),
                    invocation_id: "invocation".into(),
                    passage: AgentPassageKind::FinalResponse {
                        runtime_event_id: "event".into(),
                    },
                },
            },
            current_actionable_evidence: vec![CurrentActionableEvidence {
                evidence_id: "current".into(),
                origin_reference: ProductDecisionEvidenceOriginReference::AgentSessionCompleted {
                    opaque_id: "agent-record".into(),
                },
                destination: AgentPassage {
                    kind: AgentPassageReferenceKind::AgentSessionPassage,
                    session_id: "session".into(),
                    invocation_id: "invocation".into(),
                    passage: AgentPassageKind::SubmittedInput,
                },
            }],
            historical_unresolved_evidence: vec![HistoricalUnresolvedEvidence {
                evidence_id: "old".into(),
                origin_reference: ProductDecisionEvidenceOriginReference::WorkUnitApproved {
                    opaque_id: "legacy-work-unit".into(),
                },
                label: "Retained history".into(),
            }],
        }
    }
    #[test]
    fn persists_immutable_versions_with_current_not_applied_and_history() {
        let (dir, repo) = repo();
        let first = repo.accept(input("one", None)).unwrap();
        let mut second_input = input("two", Some(1));
        second_input.statement = "Corrected".into();
        let second = repo.accept(second_input).unwrap();
        assert_eq!((first.version, second.version), (1, 2));
        assert_eq!(
            repo.query("epic").unwrap().decisions[0].application_state,
            "not_applied"
        );
        assert_eq!(repo.history("epic", "decision").unwrap().len(), 2);
        drop(repo);
        let reopened = ProductDecisionRepository::open(dir.path().join("active.sqlite")).unwrap();
        assert_eq!(
            reopened.query("epic").unwrap().decisions[0]
                .current_version
                .statement,
            "Corrected"
        );
    }
    #[test]
    fn idempotency_and_revision_conflicts_do_not_last_write_win() {
        let (_dir, repo) = repo();
        let first = repo.accept(input("one", None)).unwrap();
        assert_eq!(
            repo.accept(input("one", None)).unwrap().version,
            first.version
        );
        let mut different = input("one", None);
        different.title = "different".into();
        assert_eq!(
            repo.accept(different),
            Err(ProductDecisionError::IdempotencyConflict)
        );
        assert_eq!(repo.accept(input("stale", Some(1))).unwrap().version, 2);
        assert_eq!(
            repo.accept(input("concurrent", Some(1))),
            Err(ProductDecisionError::RevisionConflict)
        );
    }
    #[test]
    fn manual_acceptance_is_typed_without_a_required_reason() {
        let (_dir, repo) = repo();
        let mut manual = input("manual", None);
        manual.acceptance_provenance = AcceptanceProvenance::ManualHumanApplication {
            human_interaction_origin: ProductDecisionEvidenceOriginReference::HumanInteraction {
                opaque_id: "human-manual".into(),
            },
        };
        manual.current_actionable_evidence.clear();
        let accepted = repo.accept(manual).unwrap();
        assert_eq!(
            accepted.acceptance_provenance,
            AcceptanceProvenance::ManualHumanApplication {
                human_interaction_origin:
                    ProductDecisionEvidenceOriginReference::HumanInteraction {
                        opaque_id: "human-manual".into()
                    },
            }
        );
    }
    #[test]
    fn rejects_foreign_or_mismatched_agent_evidence() {
        let (_dir, repo) = repo();
        let mut foreign = input("foreign", None);
        foreign.acceptance_provenance = AcceptanceProvenance::AgentAssisted {
            human_interaction_origin: ProductDecisionEvidenceOriginReference::HumanInteraction {
                opaque_id: "human-acceptance".into(),
            },
            proposal_passage: AgentPassage {
                kind: AgentPassageReferenceKind::AgentSessionPassage,
                session_id: "foreign".into(),
                invocation_id: "invocation".into(),
                passage: AgentPassageKind::SubmittedInput,
            },
        };
        assert_eq!(
            repo.accept(foreign),
            Err(ProductDecisionError::InvalidInput)
        );
        let mut mismatch = input("mismatch", None);
        mismatch.current_actionable_evidence[0].destination.passage = AgentPassageKind::Activity {
            runtime_event_id: "foreign".into(),
        };
        assert_eq!(
            repo.accept(mismatch),
            Err(ProductDecisionError::InvalidInput)
        );
        let first = repo.accept(input("ok", None)).unwrap();
        let mut foreign_epic = input("foreign-epic", Some(first.version));
        foreign_epic.epic_id = "other".into();
        assert_eq!(
            repo.accept(foreign_epic),
            Err(ProductDecisionError::RevisionConflict)
        );
        assert_eq!(
            repo.history("other", "decision"),
            Err(ProductDecisionError::NotFound)
        );
    }
    #[test]
    fn current_query_is_epic_scoped_and_retains_typed_current_and_historical_origins() {
        let (_dir, repo) = repo();
        repo.accept(input("one", None)).unwrap();
        let mut other = input("other", None);
        other.decision_id = "other-decision".into();
        other.epic_id = "other-epic".into();
        repo.accept(other).unwrap();
        let query = repo.query("epic").unwrap();
        assert_eq!(query.epic_id, "epic");
        assert_eq!(query.decisions.len(), 1);
        assert_eq!(query.decisions[0].decision_id, "decision");
        let version = &query.decisions[0].current_version;
        assert!(matches!(
            &version.current_actionable_evidence[0].destination.passage,
            AgentPassageKind::SubmittedInput
        ));
        assert!(matches!(
            &version.historical_unresolved_evidence[0].origin_reference,
            ProductDecisionEvidenceOriginReference::WorkUnitApproved { .. }
        ));
        assert!(repo
            .query("other-epic")
            .unwrap()
            .decisions
            .iter()
            .all(|item| item.epic_id == "other-epic"));
    }
    #[test]
    fn wire_contract_uses_exact_navigation_destinations_and_rejects_unsupported_origins() {
        let value = serde_json::to_value(input("wire", None)).unwrap();
        assert_eq!(
            value["acceptanceProvenance"]["proposalPassage"]["kind"],
            "agent_session_passage"
        );
        assert_eq!(
            value["acceptanceProvenance"]["humanInteractionOrigin"]["opaqueId"],
            "human-acceptance"
        );
        assert_eq!(
            value["acceptanceProvenance"]["proposalPassage"]["passage"]["kind"],
            "final_response"
        );
        assert_eq!(
            value["acceptanceProvenance"]["proposalPassage"]["passage"]["runtimeEventId"],
            "event"
        );
        let mut malformed = value;
        malformed["historicalUnresolvedEvidence"][0]["originReference"]["kind"] =
            serde_json::json!("unsupported");
        assert!(serde_json::from_value::<AcceptProductDecisionVersionInput>(malformed).is_err());
        let (_dir, repo) = repo();
        let mut invalid_manual = input("invalid-manual", None);
        invalid_manual.acceptance_provenance = AcceptanceProvenance::ManualHumanApplication {
            human_interaction_origin: ProductDecisionEvidenceOriginReference::EpicCompleted {
                opaque_id: "not-human".into(),
            },
        };
        assert_eq!(
            repo.accept(invalid_manual),
            Err(ProductDecisionError::InvalidInput)
        );
    }
    #[test]
    fn persisted_normalized_event_kind_must_match_its_requested_passage_anchor() {
        let (_dir, repo) = repo();
        let mut activity_as_final = input("activity-as-final", None);
        activity_as_final.acceptance_provenance = AcceptanceProvenance::AgentAssisted {
            human_interaction_origin: ProductDecisionEvidenceOriginReference::HumanInteraction {
                opaque_id: "human".into(),
            },
            proposal_passage: AgentPassage {
                kind: AgentPassageReferenceKind::AgentSessionPassage,
                session_id: "session".into(),
                invocation_id: "invocation".into(),
                passage: AgentPassageKind::FinalResponse {
                    runtime_event_id: "activity".into(),
                },
            },
        };
        assert_eq!(
            repo.accept(activity_as_final),
            Err(ProductDecisionError::InvalidInput)
        );
        let mut final_as_activity = input("final-as-activity", None);
        final_as_activity.current_actionable_evidence[0]
            .destination
            .passage = AgentPassageKind::Activity {
            runtime_event_id: "event".into(),
        };
        assert_eq!(
            repo.accept(final_as_activity),
            Err(ProductDecisionError::InvalidInput)
        );
        let mut activity = input("activity", None);
        activity.current_actionable_evidence[0].destination.passage = AgentPassageKind::Activity {
            runtime_event_id: "activity".into(),
        };
        assert!(repo.accept(activity).is_ok());
    }

    #[test]
    fn agent_assisted_correction_is_bound_to_its_base_and_only_explicit_acceptance_creates_a_version(
    ) {
        let (_dir, repo) = repo();
        let first = repo.accept(input("first", None)).unwrap();
        let correction = repo
            .start_correction(
                "epic",
                "decision",
                first.version,
                "session",
                "initial",
                "prompt",
            )
            .unwrap();
        let proposal = repo
            .save_correction_proposal(SaveProductDecisionCorrectionProposalInput {
                correction_id: correction.correction_id.clone(),
                title: "Corrected title".into(),
                statement: "Corrected statement".into(),
                intent: "Corrected intent".into(),
                proposal_passage: AgentPassage {
                    kind: AgentPassageReferenceKind::AgentSessionPassage,
                    session_id: "session".into(),
                    invocation_id: "invocation".into(),
                    passage: AgentPassageKind::FinalResponse {
                        runtime_event_id: "event".into(),
                    },
                },
            })
            .unwrap();
        let reopened = repo
            .start_correction(
                "epic",
                "decision",
                first.version,
                "unused-session",
                "unused-initial",
                "unused prompt",
            )
            .unwrap();
        assert_eq!(reopened.session_id, correction.session_id);
        let initialization = repo
            .correction_initialization(&correction.correction_id)
            .unwrap();
        assert_eq!(initialization.initial_invocation_id, "initial");
        assert_eq!(initialization.initial_prompt, "prompt");
        assert_eq!(
            reopened
                .latest_proposal
                .as_ref()
                .map(|item| item.proposal_id.as_str()),
            Some(proposal.proposal_id.as_str())
        );
        assert_eq!(repo.history("epic", "decision").unwrap().len(), 1);
        let distinct_proposal = repo
            .save_correction_proposal(SaveProductDecisionCorrectionProposalInput {
                correction_id: correction.correction_id.clone(),
                title: "A different correction".into(),
                statement: "Different statement".into(),
                intent: "Different intent".into(),
                proposal_passage: proposal.proposal_passage.clone(),
            })
            .unwrap();
        let acceptance = repo.correction_acceptance(&proposal.proposal_id).unwrap();
        let accepted = repo
            .accept_correction_proposal(AcceptProductDecisionCorrectionProposalInput {
                proposal_id: proposal.proposal_id.clone(),
            })
            .unwrap();
        assert_eq!(accepted.version, 2);
        assert!(matches!(
            accepted.acceptance_provenance,
            AcceptanceProvenance::AgentAssisted {
                human_interaction_origin:
                    ProductDecisionEvidenceOriginReference::HumanInteraction { .. },
                ..
            }
        ));
        assert!(accepted.current_actionable_evidence.iter().any(|evidence| {
            evidence.evidence_id == format!("agent-assisted-proposal:{}", proposal.proposal_id)
                && matches!(
                    evidence.origin_reference,
                    ProductDecisionEvidenceOriginReference::HumanInteraction { ref opaque_id }
                        if opaque_id == &acceptance.human_interaction_id
                )
                && evidence.destination == proposal.proposal_passage
        }));
        assert_eq!(
            repo.accept_correction_proposal(AcceptProductDecisionCorrectionProposalInput {
                proposal_id: distinct_proposal.proposal_id,
            }),
            Err(ProductDecisionError::RevisionConflict)
        );
        assert_eq!(
            repo.accept_correction_proposal(AcceptProductDecisionCorrectionProposalInput {
                proposal_id: proposal.proposal_id,
            })
            .unwrap()
            .version,
            2
        );
    }

    #[test]
    fn command_start_reopens_one_persisted_initialization_without_replacing_its_session_or_prompt()
    {
        let directory = tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let state = command_state(&path);
        let current = seed_command_decision(&state.repository);

        // This represents a process stop after the decision-bound correction was committed but
        // before application-owned prompt delivery. The next command must launch this exact row.
        let persisted = state
            .repository
            .start_correction(
                "epic",
                "decision",
                current.version,
                "product-decision-correction-recovery",
                "product-decision-correction-initial-recovery",
                "recover this exact decision context",
            )
            .unwrap();
        let started = start_product_decision_correction(
            &state,
            StartProductDecisionCorrectionInput {
                epic_id: "epic".into(),
                decision_id: "decision".into(),
                base_version: current.version,
            },
        )
        .unwrap();
        let duplicate = start_product_decision_correction(
            &state,
            StartProductDecisionCorrectionInput {
                epic_id: "epic".into(),
                decision_id: "decision".into(),
                base_version: current.version,
            },
        )
        .unwrap();

        assert_eq!(started.correction_id, persisted.correction_id);
        assert_eq!(started.session_id, persisted.session_id);
        assert_eq!(duplicate.correction_id, persisted.correction_id);
        assert_eq!(duplicate.session_id, persisted.session_id);
        assert_eq!(
            state
                .repository
                .correction_initialization(&persisted.correction_id)
                .unwrap()
                .initial_prompt,
            "recover this exact decision context"
        );
        let session = state
            .sessions
            .load_session(
                &AgentSessionId::new(persisted.session_id).expect("stored Session ID is valid"),
            )
            .unwrap();
        assert_eq!(session.invocations.len(), 1);
        assert_eq!(
            session.invocations[0].invocation.submitted_text,
            "recover this exact decision context"
        );
        assert_eq!(
            state
                .sessions
                .application_invocation_launch_evidence(
                    &session.invocations[0].invocation.id,
                    &session.session.id,
                )
                .unwrap(),
            ApplicationInvocationLaunchEvidence::LaunchAccepted
        );
    }

    #[test]
    #[cfg(feature = "live-tests")]
    #[ignore = "requires CODEX_PRODUCT_DECISION_LIVE_SMOKE=true and launches a real decision-bound Codex Session"]
    fn product_decision_live_correction_retains_exact_final_response() {
        assert!(matches!(
            std::env::var("CODEX_PRODUCT_DECISION_LIVE_SMOKE").as_deref(),
            Ok("true") | Ok("1") | Ok("yes")
        ));
        let directory = tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let launches = Arc::new(Mutex::new(Vec::new()));
        let state = live_command_state(&path, launches.clone());
        let current = seed_command_decision(&state.repository);
        let result = (|| -> Result<serde_json::Value, String> {
            let correction = start_product_decision_correction(
                &state,
                StartProductDecisionCorrectionInput {
                    epic_id: "epic".into(),
                    decision_id: "decision".into(),
                    base_version: current.version,
                },
            )
            .map_err(|error| format!("start: {error:?}"))?;
            let initialization = state
                .repository
                .correction_initialization(&correction.correction_id)
                .map_err(|error| format!("initialization: {error:?}"))?;
            let session_id = AgentSessionId::new(correction.session_id.clone())
                .map_err(|error| error.to_string())?;
            let initial_id = AgentInvocationId::new(initialization.initial_invocation_id)
                .map_err(|error| error.to_string())?;
            wait_for_terminal_invocation(&state, &session_id, &initial_id)?;
            let launches_before_reopen = launches.lock().unwrap().len();
            let reopened = start_product_decision_correction(
                &state,
                StartProductDecisionCorrectionInput {
                    epic_id: "epic".into(),
                    decision_id: "decision".into(),
                    base_version: current.version,
                },
            )
            .map_err(|error| format!("reopen: {error:?}"))?;
            let reopened_initialization = state
                .repository
                .correction_initialization(&reopened.correction_id)
                .map_err(|error| format!("reopen initialization: {error:?}"))?;
            if reopened.correction_id != correction.correction_id
                || reopened.session_id != correction.session_id
                || reopened_initialization.initial_invocation_id != initial_id.as_str()
                || reopened_initialization.initial_prompt != initialization.initial_prompt
                || launches.lock().unwrap().len() != launches_before_reopen
            {
                return Err("reopen changed the persisted correction initialization".into());
            }
            let continuation = send_product_decision_correction(&state, SendProductDecisionCorrectionMessageInput { correction_id: correction.correction_id.clone(), submitted_text: "Return one concise correction proposal for review: title, statement, and intent. Do not accept or apply it.".into() }).map_err(|error| format!("continuation: {error:?}"))?;
            let continuation_id = AgentInvocationId::new(continuation.invocation_id)
                .map_err(|error| error.to_string())?;
            let history = wait_for_terminal_invocation(&state, &session_id, &continuation_id)?;
            let response = history.invocations.iter().find(|item| item.invocation.id == continuation_id).and_then(|item| item.events.iter().rev().find(|event| event.normalized.as_ref().is_some_and(|normalized| normalized.kind == crate::agent_sessions::domain::NormalizedRuntimeEventKind::AgentMessage && normalized.details.as_ref().and_then(|details| details["role"].as_str()) == Some("final")))).ok_or_else(|| "no persisted final response".to_string())?;
            let proposal = retain_product_decision_correction_proposal(
                &state,
                SaveProductDecisionCorrectionProposalInput {
                    correction_id: correction.correction_id.clone(),
                    title: "Live correction proposal".into(),
                    statement: "Test-owned proposal retained from the exact final response.".into(),
                    intent: "Prove proposal-only passage retention without acceptance.".into(),
                    proposal_passage: AgentPassage {
                        kind: AgentPassageReferenceKind::AgentSessionPassage,
                        session_id: correction.session_id.clone(),
                        invocation_id: continuation_id.as_str().into(),
                        passage: AgentPassageKind::FinalResponse {
                            runtime_event_id: response.id.as_str().into(),
                        },
                    },
                },
            )
            .map_err(|error| format!("save proposal: {error:?}"))?;
            let accepted = accept_product_decision_correction(
                &state,
                AcceptProductDecisionCorrectionProposalInput {
                    proposal_id: proposal.proposal_id.clone(),
                },
            )
            .map_err(|error| format!("accept proposal: {error:?}"))?;
            let replay = accept_product_decision_correction(
                &state,
                AcceptProductDecisionCorrectionProposalInput {
                    proposal_id: proposal.proposal_id.clone(),
                },
            )
            .map_err(|error| format!("replay proposal acceptance: {error:?}"))?;
            let query = state
                .repository
                .query("epic")
                .map_err(|error| format!("current query: {error:?}"))?;
            let version_history = state
                .repository
                .history("epic", "decision")
                .map_err(|error| format!("history: {error:?}"))?;
            let exact_destination = AgentPassage {
                kind: AgentPassageReferenceKind::AgentSessionPassage,
                session_id: correction.session_id.clone(),
                invocation_id: continuation_id.as_str().into(),
                passage: AgentPassageKind::FinalResponse {
                    runtime_event_id: response.id.as_str().into(),
                },
            };
            if replay.version_id != accepted.version_id
                || accepted.version != 2
                || query.decisions.len() != 1
                || query.decisions[0].application_state != "not_applied"
                || query.decisions[0].current_version.version_id != accepted.version_id
                || version_history.len() != 2
                || accepted.current_actionable_evidence.len() != 1
                || accepted.current_actionable_evidence[0].destination != exact_destination
                || accepted.historical_unresolved_evidence.len() != 1
                || !matches!(accepted.acceptance_provenance, AcceptanceProvenance::AgentAssisted { human_interaction_origin: ProductDecisionEvidenceOriginReference::HumanInteraction { .. }, ref proposal_passage } if *proposal_passage == exact_destination)
            {
                return Err("accepted live Product Decision state did not retain the exact immutable evidence contract".into());
            }
            Ok(
                serde_json::json!({"observed":{"isolatedTestOwnedDatabase":true,"correctionId":correction.correction_id,"sessionId":correction.session_id,"initialInvocationId":initial_id,"continuationInvocationId":continuation_id,"finalResponseEventId":response.id,"proposalId":proposal.proposal_id,"acceptedVersionId":accepted.version_id,"acceptedVersion":accepted.version,"startReopenExactAndNoNewLaunch":true,"launchCount":launches.lock().unwrap().len(),"initialPrompt":"completed","userContinuation":"completed","exactFinalResponseRetained":true,"explicitAcceptanceReplaySameVersion":true,"currentDecisionCount":query.decisions.len(),"historyVersionCount":version_history.len(),"currentActionableEvidenceCount":accepted.current_actionable_evidence.len(),"retainedPriorEvidenceCount":accepted.historical_unresolved_evidence.len(),"applicationState":query.decisions[0].application_state},"unobserved":{"publicationOrApplication":true},"launches":launches.lock().unwrap().iter().map(|launch| serde_json::json!({"program":launch.program,"args":launch.args,"workingDirectory":launch.working_directory})).collect::<Vec<_>>() }),
            )
        })();
        let shutdown = state.sessions.shutdown_runtime();
        let evidence = match result {
            Ok(value) => value,
            Err(error) => serde_json::json!({"observed":{},"unobserved":{"reason":error}}),
        };
        println!("PRODUCT_DECISION_LIVE_EVIDENCE={evidence}");
        if let Ok(output) = std::env::var("CODEX_PRODUCT_DECISION_LIVE_EVIDENCE_PATH") {
            std::fs::write(output, evidence.to_string()).expect("write live evidence");
        }
        shutdown.expect("shutdown live runtime");
        if let Some(error) = evidence["unobserved"]["reason"].as_str() {
            panic!("Product Decision live correction proof: {error}");
        }
    }
}
