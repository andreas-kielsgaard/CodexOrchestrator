//! Durable, application-owned Product Decision versions. This module deliberately does not
//! publish decisions into orchestration or infer any application scope.

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
);"#;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentPassage {
    session_id: String,
    invocation_id: String,
    passage: AgentPassageKind,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AgentPassageKind {
    SubmittedInput,
    RuntimeEvent { runtime_event_id: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AcceptanceProvenance {
    /// The command itself is the explicit human acceptance; no reason is required.
    ManualHumanApplication,
    /// Agent material remains proposal-only until this explicit human acceptance command records it.
    AgentAssisted { passage: AgentPassage },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurrentAgentPassageEvidence {
    evidence_id: String,
    passage: AgentPassage,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoricalUnresolvedEvidence {
    evidence_id: String,
    legacy_reference: String,
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
    current_actionable_evidence: Vec<CurrentAgentPassageEvidence>,
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
    current_actionable_evidence: Vec<CurrentAgentPassageEvidence>,
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
}
impl ProductDecisionTauriState {
    pub(crate) fn new(repository: Arc<ProductDecisionRepository>) -> Self {
        Self { repository }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDecisionHistoryInput {
    epic_id: String,
    decision_id: String,
}
#[derive(Serialize)]
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
) -> Result<ProductDecisionQuery, ProductDecisionTransportError> {
    state.repository.query().map_err(Into::into)
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
        validate_agent_references(&transaction, &input.acceptance_provenance)?;
        for evidence in &input.current_actionable_evidence {
            validate_passage(&transaction, &evidence.passage)?;
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

    pub(crate) fn query(&self) -> Result<ProductDecisionQuery, ProductDecisionError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        let mut statement = connection.prepare("SELECT d.decision_id,d.epic_id,v.version_id FROM product_decisions d JOIN product_decision_versions v ON v.decision_id=d.decision_id AND v.version=d.current_version ORDER BY d.decision_id").map_err(|_| ProductDecisionError::Unavailable)?;
        let rows = statement
            .query_map([], |r| {
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
        Ok(ProductDecisionQuery { decisions })
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
            || evidence.legacy_reference.trim().is_empty()
            || evidence.label.trim().is_empty()
            || !ids.insert(&evidence.evidence_id)
        {
            return Err(ProductDecisionError::InvalidInput);
        }
    }
    Ok(())
}
fn validate_agent_references(
    connection: &rusqlite::Transaction<'_>,
    provenance: &AcceptanceProvenance,
) -> Result<(), ProductDecisionError> {
    if let AcceptanceProvenance::AgentAssisted { passage } = provenance {
        validate_passage(connection, passage)?;
    }
    Ok(())
}
fn validate_passage(
    connection: &rusqlite::Transaction<'_>,
    passage: &AgentPassage,
) -> Result<(), ProductDecisionError> {
    if passage.session_id.trim().is_empty() || passage.invocation_id.trim().is_empty() {
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
    if let AgentPassageKind::RuntimeEvent { runtime_event_id } = &passage.passage {
        if runtime_event_id.trim().is_empty() {
            return Err(ProductDecisionError::InvalidInput);
        }
        let event: Option<()> = connection
            .query_row(
                "SELECT 1 FROM agent_session_runtime_events WHERE id=?1 AND invocation_id=?2",
                params![runtime_event_id, passage.invocation_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|_| ProductDecisionError::Unavailable)?;
        if event.is_none() {
            return Err(ProductDecisionError::InvalidInput);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    fn repo() -> (tempfile::TempDir, ProductDecisionRepository) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("active.sqlite");
        let connection = crate::storage::open_active_database(&path).unwrap();
        connection.execute("INSERT INTO agent_sessions(id,title,availability,requested_options_json,created_at,updated_at) VALUES('session','s','available','{}','t','t')",[]).unwrap();
        connection.execute("INSERT INTO agent_session_invocations(id,session_id,submitted_text,input_provenance,status,requested_options_json,started_at,completed_at,created_at,updated_at) VALUES('invocation','session','text','user','completed','{}','t','t','t','t')",[]).unwrap();
        connection.execute("INSERT INTO agent_session_runtime_events(id,invocation_id,sequence,source,raw_payload_json,recorded_at) VALUES('event','invocation',0,'runtime','{}','t')",[]).unwrap();
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
                passage: AgentPassage {
                    session_id: "session".into(),
                    invocation_id: "invocation".into(),
                    passage: AgentPassageKind::RuntimeEvent {
                        runtime_event_id: "event".into(),
                    },
                },
            },
            current_actionable_evidence: vec![CurrentAgentPassageEvidence {
                evidence_id: "current".into(),
                passage: AgentPassage {
                    session_id: "session".into(),
                    invocation_id: "invocation".into(),
                    passage: AgentPassageKind::SubmittedInput,
                },
            }],
            historical_unresolved_evidence: vec![HistoricalUnresolvedEvidence {
                evidence_id: "old".into(),
                legacy_reference: "legacy-ref".into(),
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
            repo.query().unwrap().decisions[0].application_state,
            "not_applied"
        );
        assert_eq!(repo.history("epic", "decision").unwrap().len(), 2);
        drop(repo);
        let reopened = ProductDecisionRepository::open(dir.path().join("active.sqlite")).unwrap();
        assert_eq!(
            reopened.query().unwrap().decisions[0]
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
        manual.acceptance_provenance = AcceptanceProvenance::ManualHumanApplication;
        manual.current_actionable_evidence.clear();
        let accepted = repo.accept(manual).unwrap();
        assert_eq!(
            accepted.acceptance_provenance,
            AcceptanceProvenance::ManualHumanApplication
        );
    }
    #[test]
    fn rejects_foreign_or_mismatched_agent_evidence() {
        let (_dir, repo) = repo();
        let mut foreign = input("foreign", None);
        foreign.acceptance_provenance = AcceptanceProvenance::AgentAssisted {
            passage: AgentPassage {
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
        mismatch.current_actionable_evidence[0].passage.passage = AgentPassageKind::RuntimeEvent {
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
}
