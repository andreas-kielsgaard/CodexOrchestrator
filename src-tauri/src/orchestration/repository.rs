use super::domain::{
    CapabilityProfileId, EffectProvenanceId, EpicPlanningDraftId, PlanBuilderProposal,
    PlanningDraftAgentSessionAssociationId, ProposalCommandId, ProposalEventId, ProposalResultId,
    ProposalRevisionId, SaveEpicPlanProposalCommand, SaveProposalError, SaveProposalResult,
    NATIVE_QUERY_VERSION,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

pub(crate) const ORCHESTRATION_SCHEMA: &str = r#"
CREATE TABLE epic_planning_drafts (
  id TEXT PRIMARY KEY,
  title TEXT,
  status TEXT NOT NULL CHECK (status IN ('active', 'canceled')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  canceled_at TEXT
);
CREATE TABLE planning_draft_lifecycle_events (
  id TEXT PRIMARY KEY,
  draft_id TEXT NOT NULL,
  event_kind TEXT NOT NULL CHECK (event_kind IN ('draft_begun', 'draft_title_updated', 'draft_canceled')),
  idempotency_key TEXT NOT NULL UNIQUE,
  actor_id TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT
);
CREATE TABLE capability_profiles (
  id TEXT PRIMARY KEY,
  status TEXT NOT NULL CHECK (status IN ('active', 'disabled', 'expired')),
  created_at TEXT NOT NULL
);
CREATE TABLE planning_draft_agent_session_associations (
  id TEXT PRIMARY KEY,
  draft_id TEXT NOT NULL,
  agent_session_id TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  associated_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT,
  FOREIGN KEY (agent_session_id) REFERENCES agent_sessions(id) ON DELETE RESTRICT
);
CREATE TABLE planning_draft_profile_assignments (
  draft_id TEXT NOT NULL,
  capability_profile_id TEXT NOT NULL,
  agent_session_association_id TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  assigned_at TEXT NOT NULL,
  PRIMARY KEY (draft_id, capability_profile_id, agent_session_association_id),
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT,
  FOREIGN KEY (capability_profile_id) REFERENCES capability_profiles(id) ON DELETE RESTRICT,
  FOREIGN KEY (agent_session_association_id) REFERENCES planning_draft_agent_session_associations(id) ON DELETE RESTRICT
);
CREATE TABLE proposal_commands (
  id TEXT PRIMARY KEY,
  idempotency_key TEXT NOT NULL UNIQUE,
  draft_id TEXT NOT NULL,
  capability_profile_id TEXT NOT NULL,
  agent_session_association_id TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  expected_revision_token TEXT,
  proposal_json TEXT NOT NULL CHECK (json_valid(proposal_json)),
  payload_fingerprint TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT
);
CREATE TABLE effect_provenance (
  id TEXT PRIMARY KEY,
  source_kind TEXT NOT NULL CHECK (source_kind = 'managed_plan_builder'),
  recorded_at TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  agent_session_association_id TEXT NOT NULL,
  capability_profile_id TEXT NOT NULL,
  causal_command_id TEXT NOT NULL UNIQUE,
  causal_result_id TEXT NOT NULL UNIQUE,
  FOREIGN KEY (causal_command_id) REFERENCES proposal_commands(id) ON DELETE RESTRICT
);
CREATE TABLE proposal_revisions (
  id TEXT PRIMARY KEY,
  draft_id TEXT NOT NULL,
  parent_revision_id TEXT,
  revision_token TEXT NOT NULL UNIQUE,
  proposal_json TEXT NOT NULL CHECK (json_valid(proposal_json)),
  command_id TEXT NOT NULL UNIQUE,
  provenance_id TEXT NOT NULL UNIQUE,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT,
  FOREIGN KEY (parent_revision_id) REFERENCES proposal_revisions(id) ON DELETE RESTRICT,
  FOREIGN KEY (command_id) REFERENCES proposal_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (provenance_id) REFERENCES effect_provenance(id) ON DELETE RESTRICT
);
CREATE INDEX proposal_revisions_by_draft ON proposal_revisions(draft_id, recorded_at, id);
CREATE TABLE proposal_events (
  id TEXT PRIMARY KEY,
  draft_id TEXT NOT NULL,
  revision_id TEXT NOT NULL,
  command_id TEXT NOT NULL UNIQUE,
  provenance_id TEXT NOT NULL,
  event_kind TEXT NOT NULL CHECK (event_kind = 'proposal_saved'),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT,
  FOREIGN KEY (revision_id) REFERENCES proposal_revisions(id) ON DELETE RESTRICT,
  FOREIGN KEY (command_id) REFERENCES proposal_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (provenance_id) REFERENCES effect_provenance(id) ON DELETE RESTRICT
);
CREATE INDEX proposal_events_by_draft ON proposal_events(draft_id, recorded_at, id);
CREATE TABLE proposal_command_results (
  id TEXT PRIMARY KEY,
  command_id TEXT NOT NULL UNIQUE,
  revision_id TEXT NOT NULL UNIQUE,
  event_id TEXT NOT NULL UNIQUE,
  provenance_id TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES proposal_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (revision_id) REFERENCES proposal_revisions(id) ON DELETE RESTRICT,
  FOREIGN KEY (event_id) REFERENCES proposal_events(id) ON DELETE RESTRICT,
  FOREIGN KEY (provenance_id) REFERENCES effect_provenance(id) ON DELETE RESTRICT
);
"#;

/// Additive active-v3 migration. A snapshot is the exact proposal bytes consumed by initiation;
/// it deliberately is not a generated document or filesystem artifact.
pub(crate) const ORCHESTRATION_INITIATION_SCHEMA: &str = r#"
CREATE TABLE epic_initiation_commands (
  id TEXT PRIMARY KEY, idempotency_key TEXT NOT NULL UNIQUE, draft_id TEXT NOT NULL,
  expected_revision_token TEXT NOT NULL, actor_id TEXT NOT NULL, payload_fingerprint TEXT NOT NULL,
  recorded_at TEXT NOT NULL, FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT
);
CREATE TABLE epic_initiation_results (
  id TEXT PRIMARY KEY, command_id TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES epic_initiation_commands(id) ON DELETE RESTRICT
);
CREATE TABLE epic_initiation_events (
  id TEXT PRIMARY KEY, command_id TEXT NOT NULL UNIQUE, result_id TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES epic_initiation_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (result_id) REFERENCES epic_initiation_results(id) ON DELETE RESTRICT
);
CREATE TABLE epic_initiation_provenance (
  id TEXT PRIMARY KEY, command_id TEXT NOT NULL UNIQUE, result_id TEXT NOT NULL UNIQUE, event_id TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES epic_initiation_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (result_id) REFERENCES epic_initiation_results(id) ON DELETE RESTRICT,
  FOREIGN KEY (event_id) REFERENCES epic_initiation_events(id) ON DELETE RESTRICT
);
CREATE TABLE epic_initiation_material_snapshots (
  id TEXT PRIMARY KEY, draft_id TEXT NOT NULL, proposal_revision_id TEXT NOT NULL,
  version INTEGER NOT NULL CHECK (version = 1), proposal_json TEXT NOT NULL CHECK (json_valid(proposal_json)),
  content_hash TEXT NOT NULL, recorded_at TEXT NOT NULL,
  UNIQUE(draft_id, proposal_revision_id), FOREIGN KEY (proposal_revision_id) REFERENCES proposal_revisions(id) ON DELETE RESTRICT
);
CREATE TABLE epic_initiations (
  id TEXT PRIMARY KEY, command_id TEXT NOT NULL UNIQUE, result_id TEXT NOT NULL UNIQUE, event_id TEXT NOT NULL UNIQUE,
  provenance_id TEXT NOT NULL UNIQUE, draft_id TEXT NOT NULL UNIQUE, proposal_revision_id TEXT NOT NULL UNIQUE,
  material_snapshot_id TEXT NOT NULL UNIQUE, epic_id TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES epic_initiation_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (proposal_revision_id) REFERENCES proposal_revisions(id) ON DELETE RESTRICT,
  FOREIGN KEY (material_snapshot_id) REFERENCES epic_initiation_material_snapshots(id) ON DELETE RESTRICT
);
CREATE TABLE initiated_planning_drafts (
  draft_id TEXT PRIMARY KEY, initiation_id TEXT NOT NULL UNIQUE, initiated_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT,
  FOREIGN KEY (initiation_id) REFERENCES epic_initiations(id) ON DELETE RESTRICT
);
CREATE TABLE initiated_sprints (
  id TEXT PRIMARY KEY, epic_id TEXT NOT NULL, ordinal INTEGER NOT NULL, title TEXT NOT NULL,
  intended_movement TEXT NOT NULL, concern_summaries_json TEXT NOT NULL CHECK (json_valid(concern_summaries_json)),
  sprint_plan_id TEXT NOT NULL UNIQUE, sprint_plan_revision_id TEXT NOT NULL UNIQUE,
  UNIQUE(epic_id, ordinal), FOREIGN KEY (epic_id) REFERENCES epic_initiations(epic_id) ON DELETE RESTRICT
);
"#;

/// Durable one-shot application context scheduled only by a confirmed button initiation.
pub(crate) const PLAN_BUILDER_CONTEXT_DELIVERY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS plan_builder_context_deliveries (
  id TEXT PRIMARY KEY,
  initiation_id TEXT NOT NULL UNIQUE,
  epic_id TEXT NOT NULL,
  agent_session_id TEXT NOT NULL,
  source_kind TEXT NOT NULL CHECK (source_kind = 'button_initiation'),
  requested_at TEXT NOT NULL,
  pending_at TEXT NOT NULL,
  delivery_claim_id TEXT,
  delivery_claimed_at TEXT,
  target_invocation_id TEXT UNIQUE,
  delivered_to_invocation_id TEXT UNIQUE,
  delivered_at TEXT,
  consumed_at TEXT,
  FOREIGN KEY (initiation_id) REFERENCES epic_initiations(id) ON DELETE RESTRICT,
  FOREIGN KEY (agent_session_id) REFERENCES agent_sessions(id) ON DELETE RESTRICT,
  FOREIGN KEY (delivered_to_invocation_id) REFERENCES agent_session_invocations(id) ON DELETE RESTRICT,
  CHECK ((delivery_claim_id IS NULL) = (delivery_claimed_at IS NULL)),
  CHECK ((delivery_claim_id IS NULL) = (target_invocation_id IS NULL)),
  CHECK ((delivered_to_invocation_id IS NULL) = (delivered_at IS NULL)),
  CHECK ((delivered_at IS NULL) = (consumed_at IS NULL))
);
CREATE INDEX IF NOT EXISTS plan_builder_context_pending_by_session
  ON plan_builder_context_deliveries(agent_session_id, pending_at)
  WHERE consumed_at IS NULL;
"#;

pub(crate) const PLAN_BUILDER_CONTEXT_RECONCILIATION_SCHEMA: &str = r#"
ALTER TABLE plan_builder_context_deliveries ADD COLUMN target_invocation_id TEXT;
CREATE UNIQUE INDEX plan_builder_context_target_invocation
  ON plan_builder_context_deliveries(target_invocation_id)
  WHERE target_invocation_id IS NOT NULL;
"#;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingPlanBuilderContextDelivery {
    pub(crate) delivery_id: String,
    pub(crate) initiation_id: String,
    pub(crate) epic_id: String,
    pub(crate) claim_id: String,
    pub(crate) target_invocation_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManagedPlanBuilderBinding {
    pub(crate) associated_at: DateTime<Utc>,
}

pub(crate) struct SqliteOrchestrationRepository {
    connection: Mutex<Connection>,
    clock: Arc<dyn OrchestrationClock>,
    #[cfg(test)]
    fail_next_context_consume: std::sync::atomic::AtomicBool,
}

pub(crate) trait OrchestrationClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

struct SystemOrchestrationClock;
impl OrchestrationClock for SystemOrchestrationClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

impl SqliteOrchestrationRepository {
    pub(crate) fn new(connection: Connection) -> Result<Self, SaveProposalError> {
        Self::new_with_clock(connection, Arc::new(SystemOrchestrationClock))
    }

    pub(crate) fn new_with_clock(
        connection: Connection,
        clock: Arc<dyn OrchestrationClock>,
    ) -> Result<Self, SaveProposalError> {
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(sql_error("configure orchestration database"))?;
        Ok(Self {
            connection: Mutex::new(connection),
            clock,
            #[cfg(test)]
            fail_next_context_consume: std::sync::atomic::AtomicBool::new(false),
        })
    }

    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, SaveProposalError> {
        let connection =
            Connection::open(path).map_err(sql_error("open orchestration database"))?;
        Self::new(connection)
    }

    pub(crate) fn create_planning_draft(
        &self,
        id: &EpicPlanningDraftId,
        created_at: DateTime<Utc>,
    ) -> Result<(), SaveProposalError> {
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO epic_planning_drafts (id, title, status, created_at, updated_at) VALUES (?1, NULL, 'active', ?2, ?2)",
                params![id.as_str(), timestamp(created_at)],
            )
            .map_err(sql_error("create planning draft"))?;
        Ok(())
    }

    pub(crate) fn schedule_button_initiation_context(
        &self,
        initiation: &super::domain::InitiateEpicResult,
    ) -> Result<(), SaveProposalError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_error("begin button initiation context scheduling"))?;
        let (session_id, epic_id): (String, String) = transaction
            .query_row(
                "SELECT association.agent_session_id, initiation.epic_id
                 FROM epic_initiations initiation
                 JOIN planning_draft_agent_session_associations association
                   ON association.draft_id=initiation.draft_id
                 WHERE initiation.id=?1",
                params![initiation.initiation_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sql_error(
                "derive managed Plan Builder session for initiation context",
            ))?;
        if epic_id != initiation.epic_id.as_str() {
            return Err(SaveProposalError::Unavailable(
                "button initiation context identity does not match durable initiation".into(),
            ));
        }
        let now = timestamp(self.clock.now());
        let delivery_id = format!("plan-builder-context-{}", initiation.initiation_id.as_str());
        transaction
            .execute(
                "INSERT OR IGNORE INTO plan_builder_context_deliveries
                 (id,initiation_id,epic_id,agent_session_id,source_kind,requested_at,pending_at)
                 VALUES (?1,?2,?3,?4,'button_initiation',?5,?5)",
                params![
                    delivery_id,
                    initiation.initiation_id.as_str(),
                    epic_id,
                    session_id,
                    now
                ],
            )
            .map_err(sql_error("record pending button initiation context"))?;
        let existing: (String, String, String) = transaction
            .query_row(
                "SELECT initiation_id,epic_id,agent_session_id
                 FROM plan_builder_context_deliveries WHERE id=?1",
                params![delivery_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(sql_error("verify pending button initiation context"))?;
        if existing
            != (
                initiation.initiation_id.as_str().to_string(),
                initiation.epic_id.as_str().to_string(),
                session_id,
            )
        {
            return Err(SaveProposalError::Unavailable(
                "button initiation context identity was already used for different semantics"
                    .into(),
            ));
        }
        transaction
            .commit()
            .map_err(sql_error("commit button initiation context scheduling"))
    }

    pub(crate) fn claim_pending_plan_builder_context(
        &self,
        session_id: &str,
        claim_id: &str,
        target_invocation_id: &str,
    ) -> Result<Option<PendingPlanBuilderContextDelivery>, SaveProposalError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_error("begin Plan Builder context claim"))?;
        let unresolved_claims: i64 = transaction
            .query_row(
                "SELECT count(*) FROM plan_builder_context_deliveries
                 WHERE agent_session_id=?1 AND consumed_at IS NULL AND delivery_claim_id IS NOT NULL",
                params![session_id],
                |row| row.get(0),
            )
            .map_err(sql_error("check unresolved Plan Builder context claims"))?;
        if unresolved_claims != 0 {
            return Err(SaveProposalError::Unavailable(
                "an earlier Plan Builder context claim requires launch reconciliation".into(),
            ));
        }
        let delivery = transaction
            .query_row(
                "SELECT id,initiation_id,epic_id FROM plan_builder_context_deliveries
                 WHERE agent_session_id=?1 AND consumed_at IS NULL AND delivery_claim_id IS NULL
                 ORDER BY pending_at,id LIMIT 1",
                params![session_id],
                |row| {
                    Ok(PendingPlanBuilderContextDelivery {
                        delivery_id: row.get(0)?,
                        initiation_id: row.get(1)?,
                        epic_id: row.get(2)?,
                        claim_id: claim_id.to_string(),
                        target_invocation_id: target_invocation_id.to_string(),
                    })
                },
            )
            .optional()
            .map_err(sql_error("read pending Plan Builder context"))?;
        if let Some(delivery) = delivery.as_ref() {
            transaction
                .execute(
                    "UPDATE plan_builder_context_deliveries
                     SET delivery_claim_id=?2,delivery_claimed_at=?3,target_invocation_id=?4
                     WHERE id=?1 AND consumed_at IS NULL",
                    params![
                        delivery.delivery_id,
                        claim_id,
                        timestamp(self.clock.now()),
                        target_invocation_id
                    ],
                )
                .map_err(sql_error("claim pending Plan Builder context"))?;
        }
        transaction
            .commit()
            .map_err(sql_error("commit Plan Builder context claim"))?;
        Ok(delivery)
    }

    pub(crate) fn load_claimed_plan_builder_context(
        &self,
        session_id: &str,
    ) -> Result<Option<PendingPlanBuilderContextDelivery>, SaveProposalError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT id,initiation_id,epic_id,delivery_claim_id,target_invocation_id
                 FROM plan_builder_context_deliveries
                 WHERE agent_session_id=?1 AND consumed_at IS NULL AND delivery_claim_id IS NOT NULL",
                params![session_id],
                |row| {
                    Ok(PendingPlanBuilderContextDelivery {
                        delivery_id: row.get(0)?,
                        initiation_id: row.get(1)?,
                        epic_id: row.get(2)?,
                        claim_id: row.get(3)?,
                        target_invocation_id: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(sql_error("load claimed Plan Builder context"))
    }

    pub(crate) fn consume_plan_builder_context(
        &self,
        delivery: &PendingPlanBuilderContextDelivery,
    ) -> Result<(), SaveProposalError> {
        #[cfg(test)]
        if self
            .fail_next_context_consume
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(SaveProposalError::Unavailable(
                "injected Plan Builder context consume failure".into(),
            ));
        }
        let connection = self.lock()?;
        let now = timestamp(self.clock.now());
        let changed = connection
            .execute(
                "UPDATE plan_builder_context_deliveries
                 SET delivered_to_invocation_id=?3,delivered_at=?4,consumed_at=?4
                 WHERE id=?1 AND delivery_claim_id=?2 AND target_invocation_id=?3 AND consumed_at IS NULL",
                params![
                    delivery.delivery_id,
                    delivery.claim_id,
                    delivery.target_invocation_id,
                    now
                ],
            )
            .map_err(sql_error("consume Plan Builder context"))?;
        if changed != 1 {
            return Err(SaveProposalError::Unavailable(
                "pending Plan Builder context claim is stale".into(),
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn fail_next_plan_builder_context_consume(&self) {
        self.fail_next_context_consume
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub(crate) fn release_plan_builder_context(
        &self,
        delivery: &PendingPlanBuilderContextDelivery,
    ) -> Result<(), SaveProposalError> {
        let connection = self.lock()?;
        connection
            .execute(
                "UPDATE plan_builder_context_deliveries
                 SET delivery_claim_id=NULL,delivery_claimed_at=NULL,target_invocation_id=NULL
                 WHERE id=?1 AND delivery_claim_id=?2 AND target_invocation_id=?3 AND consumed_at IS NULL",
                params![
                    delivery.delivery_id,
                    delivery.claim_id,
                    delivery.target_invocation_id
                ],
            )
            .map_err(sql_error("release Plan Builder context claim"))?;
        Ok(())
    }

    pub(crate) fn create_capability_profile(
        &self,
        id: &CapabilityProfileId,
        status: &str,
        created_at: DateTime<Utc>,
    ) -> Result<(), SaveProposalError> {
        if !matches!(status, "active" | "disabled" | "expired") {
            return Err(SaveProposalError::InvalidInput(
                "capability profile status is invalid".into(),
            ));
        }
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO capability_profiles (id, status, created_at) VALUES (?1, ?2, ?3)",
                params![id.as_str(), status, timestamp(created_at)],
            )
            .map_err(sql_error("create capability profile"))?;
        Ok(())
    }

    pub(crate) fn assign_profile(
        &self,
        draft_id: &EpicPlanningDraftId,
        profile_id: &CapabilityProfileId,
        association_id: &PlanningDraftAgentSessionAssociationId,
        expires_at: DateTime<Utc>,
        assigned_at: DateTime<Utc>,
    ) -> Result<(), SaveProposalError> {
        let connection = self.lock()?;
        connection.execute("INSERT INTO planning_draft_profile_assignments (draft_id, capability_profile_id, agent_session_association_id, expires_at, assigned_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![draft_id.as_str(), profile_id.as_str(), association_id.as_str(), timestamp(expires_at), timestamp(assigned_at)])
            .map_err(sql_error("assign capability profile"))?;
        Ok(())
    }

    pub(crate) fn associate_agent_session(
        &self,
        association_id: &PlanningDraftAgentSessionAssociationId,
        draft_id: &EpicPlanningDraftId,
        session_id: &str,
        actor_id: &str,
        associated_at: DateTime<Utc>,
    ) -> Result<(), SaveProposalError> {
        let connection = self.lock()?;
        connection.execute("INSERT INTO planning_draft_agent_session_associations (id, draft_id, agent_session_id, actor_id, associated_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![association_id.as_str(), draft_id.as_str(), session_id, actor_id, timestamp(associated_at)])
            .map_err(sql_error("associate Agent Session"))?;
        Ok(())
    }

    /// Resolves the calling session's managed Plan Builder binding, or atomically creates its
    /// pre-initiation draft, profile, association, and assignment. None of these rows imply an
    /// initiated Epic.
    pub(crate) fn bootstrap_managed_plan_builder(
        &self,
        session_id: &str,
    ) -> Result<
        (
            EpicPlanningDraftId,
            CapabilityProfileId,
            PlanningDraftAgentSessionAssociationId,
        ),
        SaveProposalError,
    > {
        let profile = CapabilityProfileId::new("plan-builder-capability-profile-v1")
            .map_err(SaveProposalError::InvalidInput)?;
        let now = self.clock.now();
        let connection = self.lock()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(sql_error("begin managed Plan Builder bootstrap"))?;
        if let Some((draft, profile, association, status, _initiated)) = transaction
            .query_row(
                "SELECT assignment.draft_id, assignment.capability_profile_id, association.id, draft.status, EXISTS(SELECT 1 FROM initiated_planning_drafts initiated WHERE initiated.draft_id = draft.id) FROM planning_draft_profile_assignments assignment JOIN planning_draft_agent_session_associations association ON association.id = assignment.agent_session_association_id JOIN epic_planning_drafts draft ON draft.id = assignment.draft_id WHERE association.agent_session_id = ?1 AND association.actor_id = 'managed-plan-builder' ORDER BY association.associated_at ASC LIMIT 1",
                params![session_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, bool>(4)?)),
            )
            .optional()
            .map_err(sql_error("resolve managed Plan Builder binding"))?
        {
            if status == "canceled" {
                return Err(SaveProposalError::Forbidden);
            }
            transaction
                .commit()
                .map_err(sql_error("commit managed Plan Builder binding read"))?;
            return Ok((
                EpicPlanningDraftId::new(draft).map_err(SaveProposalError::InvalidInput)?,
                CapabilityProfileId::new(profile).map_err(SaveProposalError::InvalidInput)?,
                PlanningDraftAgentSessionAssociationId::new(association)
                    .map_err(SaveProposalError::InvalidInput)?,
            ));
        }
        let draft = EpicPlanningDraftId::new(format!(
            "epic-planning-draft-{}",
            uuid::Uuid::new_v4().simple()
        ))
        .map_err(SaveProposalError::InvalidInput)?;
        let association = PlanningDraftAgentSessionAssociationId::new(format!(
            "plan-builder-association-{}",
            uuid::Uuid::new_v4().simple()
        ))
        .map_err(SaveProposalError::InvalidInput)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO epic_planning_drafts (id, title, status, created_at, updated_at) VALUES (?1, NULL, 'active', ?2, ?2)",
                params![draft.as_str(), timestamp(now)],
            )
            .map_err(sql_error("bootstrap planning draft"))?;
        transaction.execute("INSERT OR IGNORE INTO planning_draft_lifecycle_events (id, draft_id, event_kind, idempotency_key, actor_id, recorded_at) VALUES (?1, ?2, 'draft_begun', ?3, 'application-user', ?4)", params![new_id("draft-event"), draft.as_str(), format!("begin:{session_id}"), timestamp(now)]).map_err(sql_error("record draft begin"))?;
        transaction.execute("INSERT OR IGNORE INTO capability_profiles (id, status, created_at) VALUES (?1, 'active', ?2)", params![profile.as_str(), timestamp(now)]).map_err(sql_error("bootstrap capability profile"))?;
        transaction.execute("INSERT OR IGNORE INTO planning_draft_agent_session_associations (id, draft_id, agent_session_id, actor_id, associated_at) VALUES (?1, ?2, ?3, 'managed-plan-builder', ?4)", params![association.as_str(), draft.as_str(), session_id, timestamp(now)]).map_err(sql_error("bootstrap Agent Session association"))?;
        transaction.execute("INSERT OR IGNORE INTO planning_draft_profile_assignments (draft_id, capability_profile_id, agent_session_association_id, expires_at, assigned_at) VALUES (?1, ?2, ?3, '2100-01-01T00:00:00.000Z', ?4)", params![draft.as_str(), profile.as_str(), association.as_str(), timestamp(now)]).map_err(sql_error("bootstrap capability assignment"))?;
        transaction
            .commit()
            .map_err(sql_error("commit managed Plan Builder bootstrap"))?;
        Ok((draft, profile, association))
    }

    pub(crate) fn load_managed_plan_builder_binding(
        &self,
        session_id: &str,
    ) -> Result<Option<ManagedPlanBuilderBinding>, SaveProposalError> {
        let connection = self.lock()?;
        let associated_at = connection
            .query_row(
                "SELECT association.associated_at
                 FROM planning_draft_agent_session_associations association
                 JOIN planning_draft_profile_assignments assignment
                   ON assignment.agent_session_association_id=association.id
                  AND assignment.draft_id=association.draft_id
                 WHERE association.agent_session_id=?1
                   AND association.actor_id='managed-plan-builder'
                 ORDER BY association.associated_at,association.id
                 LIMIT 1",
                params![session_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_error("load managed Plan Builder binding"))?;
        associated_at
            .map(|value| {
                DateTime::parse_from_rfc3339(&value)
                    .map(|associated_at| associated_at.with_timezone(&Utc))
                    .map(|associated_at| ManagedPlanBuilderBinding { associated_at })
                    .map_err(|error| {
                        SaveProposalError::Unavailable(format!(
                            "load managed Plan Builder binding timestamp: {error}"
                        ))
                    })
            })
            .transpose()
    }

    pub(crate) fn update_planning_draft_title(
        &self,
        draft_id: &EpicPlanningDraftId,
        session_id: &str,
        title: Option<&str>,
        idempotency_key: &str,
    ) -> Result<(), SaveProposalError> {
        let title = title.map(str::trim).filter(|value| !value.is_empty());
        if title.is_some_and(|value| value.len() > 240) || idempotency_key.trim().is_empty() {
            return Err(SaveProposalError::InvalidInput(
                "draft title or idempotency key is invalid".into(),
            ));
        }
        let now = timestamp(self.clock.now());
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_error("begin title update"))?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT draft_id FROM planning_draft_lifecycle_events WHERE idempotency_key = ?1",
                params![idempotency_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_error("read title idempotency"))?;
        if let Some(existing) = existing {
            return if existing == draft_id.as_str() {
                Ok(())
            } else {
                Err(SaveProposalError::IdempotencyConflict)
            };
        }
        let changed = transaction.execute("UPDATE epic_planning_drafts SET title = ?1, updated_at = ?2 WHERE id = ?3 AND status = 'active' AND NOT EXISTS (SELECT 1 FROM initiated_planning_drafts WHERE draft_id = ?3) AND EXISTS (SELECT 1 FROM planning_draft_agent_session_associations WHERE draft_id = ?3 AND agent_session_id = ?4 AND actor_id = 'managed-plan-builder')", params![title, now, draft_id.as_str(), session_id]).map_err(sql_error("update draft title"))?;
        if changed == 0 {
            return Err(SaveProposalError::Forbidden);
        }
        transaction.execute("INSERT INTO planning_draft_lifecycle_events (id, draft_id, event_kind, idempotency_key, actor_id, recorded_at) VALUES (?1, ?2, 'draft_title_updated', ?3, 'application-user', ?4)", params![new_id("draft-event"), draft_id.as_str(), idempotency_key, now]).map_err(sql_error("record title update"))?;
        transaction
            .commit()
            .map_err(sql_error("commit title update"))?;
        Ok(())
    }

    pub(crate) fn cancel_planning_draft(
        &self,
        draft_id: &EpicPlanningDraftId,
        session_id: &str,
        idempotency_key: &str,
    ) -> Result<(), SaveProposalError> {
        if idempotency_key.trim().is_empty() {
            return Err(SaveProposalError::InvalidInput(
                "idempotency key is required".into(),
            ));
        }
        let now = timestamp(self.clock.now());
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_error("begin draft cancellation"))?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT draft_id FROM planning_draft_lifecycle_events WHERE idempotency_key = ?1",
                params![idempotency_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_error("read cancel idempotency"))?;
        if let Some(existing) = existing {
            return if existing == draft_id.as_str() {
                Ok(())
            } else {
                Err(SaveProposalError::IdempotencyConflict)
            };
        }
        let associated: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM planning_draft_agent_session_associations WHERE draft_id = ?1 AND agent_session_id = ?2 AND actor_id = 'managed-plan-builder')", params![draft_id.as_str(), session_id], |row| row.get(0)).map_err(sql_error("authorize cancellation"))?;
        if !associated {
            return Err(SaveProposalError::Forbidden);
        }
        let changed = transaction.execute("UPDATE epic_planning_drafts SET status = 'canceled', canceled_at = COALESCE(canceled_at, ?1), updated_at = ?1 WHERE id = ?2 AND status = 'active' AND NOT EXISTS (SELECT 1 FROM initiated_planning_drafts WHERE draft_id = ?2)", params![now, draft_id.as_str()]).map_err(sql_error("cancel draft"))?;
        if changed == 0 {
            return Err(SaveProposalError::Forbidden);
        }
        transaction.execute("INSERT INTO planning_draft_lifecycle_events (id, draft_id, event_kind, idempotency_key, actor_id, recorded_at) VALUES (?1, ?2, 'draft_canceled', ?3, 'application-user', ?4)", params![new_id("draft-event"), draft_id.as_str(), idempotency_key, now]).map_err(sql_error("record cancellation"))?;
        transaction
            .commit()
            .map_err(sql_error("commit cancellation"))?;
        Ok(())
    }

    pub(crate) fn save_epic_plan_proposal(
        &self,
        command: SaveEpicPlanProposalCommand,
    ) -> Result<SaveProposalResult, SaveProposalError> {
        command
            .validate()
            .map_err(SaveProposalError::InvalidInput)?;
        let fingerprint = fingerprint(&command)?;
        let effect_time = self.clock.now();
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error("begin proposal save"))?;
        let existing = find_command_result(&transaction, &command.idempotency_key)?;
        if let Some((stored_fingerprint, _)) = &existing {
            if stored_fingerprint != &fingerprint {
                return Err(SaveProposalError::IdempotencyConflict);
            }
        }
        let draft_exists = transaction
            .query_row(
                "SELECT 1 FROM epic_planning_drafts WHERE id = ?1 AND status = 'active' AND NOT EXISTS (SELECT 1 FROM initiated_planning_drafts WHERE draft_id = epic_planning_drafts.id)",
                params![command.epic_planning_draft_id.as_str()],
                |_| Ok(()),
            )
            .optional()
            .map_err(sql_error("read planning draft"))?
            .is_some();
        if !draft_exists {
            return Err(SaveProposalError::DraftNotFound);
        }
        let authorized = transaction.query_row(
            "SELECT 1 FROM planning_draft_profile_assignments assignment JOIN planning_draft_agent_session_associations association ON association.id = assignment.agent_session_association_id JOIN capability_profiles profile ON profile.id = assignment.capability_profile_id WHERE assignment.draft_id = ?1 AND assignment.capability_profile_id = ?2 AND assignment.agent_session_association_id = ?3 AND association.actor_id = ?4 AND association.agent_session_id = ?5 AND association.draft_id = ?1 AND assignment.expires_at >= ?6 AND profile.status = 'active'",
            params![command.epic_planning_draft_id.as_str(), command.capability_profile_id.as_str(), command.agent_session_association_id.as_str(), command.actor_id, command.agent_session_id, timestamp(effect_time)], |_| Ok(())
        ).optional().map_err(sql_error("authorize proposal save"))?.is_some();
        if !authorized {
            return Err(SaveProposalError::Forbidden);
        }
        if let Some((_, mut result)) = existing {
            transaction
                .commit()
                .map_err(sql_error("commit authorized idempotent proposal save"))?;
            result.idempotent_replay = true;
            return Ok(result);
        }
        let latest: Option<(String, String)> = transaction.query_row(
            "SELECT id, revision_token FROM proposal_revisions WHERE draft_id = ?1 ORDER BY recorded_at DESC, id DESC LIMIT 1", params![command.epic_planning_draft_id.as_str()], |row| Ok((row.get(0)?, row.get(1)?))
        ).optional().map_err(sql_error("read current proposal revision"))?;
        if latest.as_ref().map(|(_, token)| token.as_str()) != command.expected_revision.as_deref()
        {
            return Err(SaveProposalError::RevisionConflict);
        }

        let command_id = ProposalCommandId::new(new_id("proposal-command"))
            .map_err(SaveProposalError::Unavailable)?;
        let result_id = ProposalResultId::new(new_id("proposal-result"))
            .map_err(SaveProposalError::Unavailable)?;
        let revision_id = ProposalRevisionId::new(new_id("proposal-revision"))
            .map_err(SaveProposalError::Unavailable)?;
        let event_id = ProposalEventId::new(new_id("proposal-event"))
            .map_err(SaveProposalError::Unavailable)?;
        let provenance_id = EffectProvenanceId::new(new_id("effect-provenance"))
            .map_err(SaveProposalError::Unavailable)?;
        let revision_token = new_id("proposal-revision-token");
        let proposal_json = serde_json::to_string(&command.proposal)
            .map_err(|error| SaveProposalError::Unavailable(error.to_string()))?;
        transaction.execute("INSERT INTO proposal_commands (id, idempotency_key, draft_id, capability_profile_id, agent_session_association_id, actor_id, expected_revision_token, proposal_json, payload_fingerprint, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", params![command_id.as_str(), command.idempotency_key, command.epic_planning_draft_id.as_str(), command.capability_profile_id.as_str(), command.agent_session_association_id.as_str(), command.actor_id, command.expected_revision, proposal_json, fingerprint, timestamp(effect_time)]).map_err(sql_error("record applied proposal command"))?;
        transaction.execute("INSERT INTO effect_provenance (id, source_kind, recorded_at, actor_id, agent_session_association_id, capability_profile_id, causal_command_id, causal_result_id) VALUES (?1, 'managed_plan_builder', ?2, ?3, ?4, ?5, ?6, ?7)", params![provenance_id.as_str(), timestamp(effect_time), command.actor_id, command.agent_session_association_id.as_str(), command.capability_profile_id.as_str(), command_id.as_str(), result_id.as_str()]).map_err(sql_error("record effect provenance"))?;
        let proposal_json = serde_json::to_string(&command.proposal)
            .map_err(|error| SaveProposalError::Unavailable(error.to_string()))?;
        transaction.execute("INSERT INTO proposal_revisions (id, draft_id, parent_revision_id, revision_token, proposal_json, command_id, provenance_id, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", params![revision_id.as_str(), command.epic_planning_draft_id.as_str(), latest.map(|(id, _)| id), revision_token, proposal_json, command_id.as_str(), provenance_id.as_str(), timestamp(effect_time)]).map_err(sql_error("record proposal revision"))?;
        transaction.execute("INSERT INTO proposal_events (id, draft_id, revision_id, command_id, provenance_id, event_kind, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, 'proposal_saved', ?6)", params![event_id.as_str(), command.epic_planning_draft_id.as_str(), revision_id.as_str(), command_id.as_str(), provenance_id.as_str(), timestamp(effect_time)]).map_err(sql_error("append proposal event"))?;
        transaction.execute("INSERT INTO proposal_command_results (id, command_id, revision_id, event_id, provenance_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![result_id.as_str(), command_id.as_str(), revision_id.as_str(), event_id.as_str(), provenance_id.as_str(), timestamp(effect_time)]).map_err(sql_error("record proposal command result"))?;
        let result = SaveProposalResult {
            command_id,
            result_id,
            revision_id,
            revision_token,
            event_id,
            provenance_id,
            idempotent_replay: false,
        };
        transaction
            .commit()
            .map_err(sql_error("commit proposal save"))?;
        Ok(result)
    }

    pub(crate) fn native_query_at(
        &self,
        generated_at: DateTime<Utc>,
    ) -> Result<NativeQueryV2, String> {
        let connection = self.lock().map_err(|error| error.to_string())?;
        let mut draft_statement = connection.prepare("SELECT draft.id, draft.title, CASE WHEN initiated.draft_id IS NOT NULL THEN 'initiated' ELSE draft.status END, draft.created_at, draft.updated_at, draft.canceled_at, latest.id FROM epic_planning_drafts draft LEFT JOIN initiated_planning_drafts initiated ON initiated.draft_id = draft.id LEFT JOIN proposal_revisions latest ON latest.id = (SELECT revision.id FROM proposal_revisions revision WHERE revision.draft_id = draft.id ORDER BY revision.recorded_at DESC, revision.id DESC LIMIT 1) ORDER BY draft.created_at, draft.id").map_err(|error| error.to_string())?;
        let planning_drafts = draft_statement
            .query_map([], |row| {
                Ok(PlanningDraftDto {
                    epic_planning_draft_id: row.get(0)?,
                    title: row.get(1)?,
                    status: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    canceled_at: row.get(5)?,
                    current_proposal: match row.get::<_, Option<String>>(6)? {
                        Some(revision_id) => CurrentProposalDto::Available {
                            proposal_revision_id: revision_id,
                        },
                        None => CurrentProposalDto::Empty {},
                    },
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        let agent_session_associations = collect(&connection, "SELECT id, draft_id, agent_session_id, actor_id, associated_at FROM planning_draft_agent_session_associations ORDER BY associated_at, id", |row| Ok(AgentSessionAssociationDto { agent_session_association_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, agent_session_id: row.get(2)?, actor_id: row.get(3)?, associated_at: row.get(4)? }))?;
        let proposal_revisions = collect(&connection, "SELECT id, draft_id, parent_revision_id, revision_token, proposal_json, command_id, provenance_id, recorded_at FROM proposal_revisions ORDER BY recorded_at, id", |row| Ok(ProposalRevisionDto { proposal_revision_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, parent_proposal_revision_id: row.get(2)?, revision_token: row.get(3)?, proposal: parse_proposal_json(row.get::<_, String>(4)?)?, command_id: row.get(5)?, provenance_id: row.get(6)?, recorded_at: row.get(7)? }))?;
        let recorded_proposal_events = collect(&connection, "SELECT id, draft_id, revision_id, command_id, provenance_id, event_kind, recorded_at FROM proposal_events ORDER BY recorded_at, id", |row| Ok(RecordedProposalEventDto { proposal_event_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, proposal_revision_id: row.get(2)?, command_id: row.get(3)?, provenance_id: row.get(4)?, event_kind: row.get(5)?, recorded_at: row.get(6)? }))?;
        let provenance_links = collect(&connection, "SELECT id, source_kind, recorded_at, actor_id, agent_session_association_id, capability_profile_id, causal_command_id, causal_result_id FROM effect_provenance ORDER BY recorded_at, id", |row| Ok(ProvenanceLinkDto { provenance_id: row.get(0)?, source_kind: row.get(1)?, recorded_at: row.get(2)?, actor_id: row.get(3)?, agent_session_association_id: row.get(4)?, capability_profile_id: row.get(5)?, causal_command_id: row.get(6)?, causal_result_id: row.get(7)? }))?;
        let initiation_commands = collect(&connection, "SELECT id, draft_id, expected_revision_token, actor_id, idempotency_key, payload_fingerprint, recorded_at FROM epic_initiation_commands ORDER BY recorded_at, id", |row| Ok(InitiationCommandDto { command_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, expected_revision_token: row.get(2)?, actor_id: row.get(3)?, idempotency_key: row.get(4)?, payload_fingerprint: row.get(5)?, recorded_at: row.get(6)? }))?;
        let initiation_results = collect(&connection, "SELECT id, command_id, recorded_at FROM epic_initiation_results ORDER BY recorded_at, id", |row| Ok(InitiationResultDto { result_id: row.get(0)?, command_id: row.get(1)?, recorded_at: row.get(2)? }))?;
        let initiation_events = collect(&connection, "SELECT id, command_id, result_id, recorded_at FROM epic_initiation_events ORDER BY recorded_at, id", |row| Ok(InitiationEventDto { event_id: row.get(0)?, command_id: row.get(1)?, result_id: row.get(2)?, recorded_at: row.get(3)? }))?;
        let initiation_provenance = collect(&connection, "SELECT id, command_id, result_id, event_id, recorded_at FROM epic_initiation_provenance ORDER BY recorded_at, id", |row| Ok(InitiationProvenanceDto { provenance_id: row.get(0)?, command_id: row.get(1)?, result_id: row.get(2)?, event_id: row.get(3)?, recorded_at: row.get(4)? }))?;
        let material_snapshots = collect(&connection, "SELECT id, draft_id, proposal_revision_id, version, proposal_json, content_hash, recorded_at FROM epic_initiation_material_snapshots ORDER BY recorded_at, id", |row| Ok(MaterialSnapshotDto { material_snapshot_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, proposal_revision_id: row.get(2)?, version: row.get(3)?, proposal: parse_proposal_json(row.get::<_, String>(4)?)?, content_hash: row.get(5)?, recorded_at: row.get(6)? }))?;
        let initiated_epics = collect(&connection, "SELECT id, draft_id, proposal_revision_id, material_snapshot_id, epic_id, recorded_at, command_id, result_id, event_id, provenance_id FROM epic_initiations ORDER BY recorded_at, id", |row| Ok(InitiatedEpicDto { initiation_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, proposal_revision_id: row.get(2)?, material_snapshot_id: row.get(3)?, epic_id: row.get(4)?, recorded_at: row.get(5)?, command_id: row.get(6)?, result_id: row.get(7)?, event_id: row.get(8)?, provenance_id: row.get(9)? }))?;
        let initiated_sprints = collect(&connection, "SELECT id, epic_id, ordinal, title, intended_movement, concern_summaries_json, sprint_plan_id, sprint_plan_revision_id FROM initiated_sprints ORDER BY epic_id, ordinal", |row| Ok(InitiatedSprintDto { sprint_id: row.get(0)?, epic_id: row.get(1)?, ordinal: row.get(2)?, title: row.get(3)?, intended_movement: row.get(4)?, concern_summaries: serde_json::from_str(&row.get::<_, String>(5)?).map_err(|e| to_sql_error(e.to_string()))?, sprint_plan_id: row.get(6)?, sprint_plan_revision_id: row.get(7)? }))?;
        Ok(NativeQueryV2 {
            contract_version: NATIVE_QUERY_VERSION,
            generated_at: timestamp(generated_at),
            planning_drafts,
            agent_session_associations,
            proposal_revisions,
            recorded_proposal_events,
            provenance_links,
            initiation_commands,
            initiation_results,
            initiation_events,
            initiation_provenance,
            material_snapshots,
            initiated_epics,
            initiated_sprints,
        })
    }

    /// Product-owned semantic transition. It consumes the current saved proposal atomically and
    /// creates only an Epic and its ordered preparatory Sprints.
    pub(crate) fn initiate_epic(
        &self,
        command: super::domain::InitiateEpicCommand,
    ) -> Result<super::domain::InitiateEpicResult, super::domain::InitiateEpicError> {
        use super::domain::{
            EpicId, EpicInitiationId, InitiateEpicError, InitiateEpicResult, ProposalRevisionId,
        };
        command.validate()?;
        if command.actor_id != "application-user" {
            return Err(InitiateEpicError::Forbidden);
        }
        let fingerprint = format!(
            "{}:{}:{}",
            command.epic_planning_draft_id.as_str(),
            command.expected_revision_token,
            command.actor_id
        );
        let now = timestamp(self.clock.now());
        let mut connection = self.connection.lock().map_err(|_| {
            InitiateEpicError::Unavailable("orchestration database lock is poisoned".into())
        })?;
        let tx = connection
            .transaction()
            .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        let existing: Option<(String, String, String, String, String)> = tx.query_row(
            "SELECT command.payload_fingerprint, initiation.id, initiation.epic_id, initiation.proposal_revision_id, snapshot.content_hash FROM epic_initiation_commands command JOIN epic_initiations initiation ON initiation.command_id = command.id JOIN epic_initiation_material_snapshots snapshot ON snapshot.id = initiation.material_snapshot_id WHERE command.idempotency_key = ?1",
            params![command.idempotency_key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))).optional().map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        if let Some((stored, initiation, epic, revision, hash)) = existing {
            if stored != fingerprint {
                return Err(InitiateEpicError::IdempotencyConflict);
            }
            return Ok(InitiateEpicResult {
                initiation_id: EpicInitiationId::new(initiation)
                    .map_err(InitiateEpicError::InvalidInput)?,
                epic_id: EpicId::new(epic).map_err(InitiateEpicError::InvalidInput)?,
                proposal_revision_id: ProposalRevisionId::new(revision)
                    .map_err(InitiateEpicError::InvalidInput)?,
                material_snapshot_hash: hash,
                idempotent_replay: true,
            });
        }
        let status: Option<String> = tx
            .query_row(
                "SELECT status FROM epic_planning_drafts WHERE id=?1",
                params![command.epic_planning_draft_id.as_str()],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        if tx
            .query_row(
                "SELECT 1 FROM initiated_planning_drafts WHERE draft_id=?1",
                params![command.epic_planning_draft_id.as_str()],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?
            .is_some()
        {
            return Err(InitiateEpicError::AlreadyInitiated);
        }
        match status.as_deref() {
            Some("active") => {}
            Some("canceled") => return Err(InitiateEpicError::Canceled),
            Some(_) => return Err(InitiateEpicError::AlreadyInitiated),
            None => return Err(InitiateEpicError::DraftNotFound),
        }
        let latest: Option<(String, String, String)> = tx.query_row("SELECT id, revision_token, proposal_json FROM proposal_revisions WHERE draft_id=?1 ORDER BY recorded_at DESC, id DESC LIMIT 1", params![command.epic_planning_draft_id.as_str()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        let (revision_id, token, proposal_json) =
            latest.ok_or(InitiateEpicError::ProposalMissing)?;
        if token != command.expected_revision_token {
            return Err(InitiateEpicError::RevisionConflict);
        }
        let proposal = parse_proposal_json(proposal_json.clone())
            .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        let command_id = new_id("epic-initiation-command");
        let result_id = new_id("epic-initiation-result");
        let event_id = new_id("epic-initiation-event");
        let provenance_id = new_id("epic-initiation-provenance");
        let snapshot_id = new_id("epic-material-snapshot");
        let initiation_id = new_id("epic-initiation");
        let epic_id = new_id("epic");
        let hash = format!("{:x}", Sha256::digest(proposal_json.as_bytes()));
        tx.execute("INSERT INTO epic_initiation_commands (id,idempotency_key,draft_id,expected_revision_token,actor_id,payload_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![command_id,command.idempotency_key,command.epic_planning_draft_id.as_str(),command.expected_revision_token,command.actor_id,fingerprint,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.execute(
            "INSERT INTO epic_initiation_results (id,command_id,recorded_at) VALUES (?1,?2,?3)",
            params![result_id, command_id, now],
        )
        .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.execute("INSERT INTO epic_initiation_events (id,command_id,result_id,recorded_at) VALUES (?1,?2,?3,?4)", params![event_id,command_id,result_id,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.execute("INSERT INTO epic_initiation_provenance (id,command_id,result_id,event_id,recorded_at) VALUES (?1,?2,?3,?4,?5)", params![provenance_id,command_id,result_id,event_id,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.execute("INSERT INTO epic_initiation_material_snapshots (id,draft_id,proposal_revision_id,version,proposal_json,content_hash,recorded_at) VALUES (?1,?2,?3,1,?4,?5,?6)", params![snapshot_id,command.epic_planning_draft_id.as_str(),revision_id,proposal_json,hash,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.execute("INSERT INTO epic_initiations (id,command_id,result_id,event_id,provenance_id,draft_id,proposal_revision_id,material_snapshot_id,epic_id,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![initiation_id,command_id,result_id,event_id,provenance_id,command.epic_planning_draft_id.as_str(),revision_id,snapshot_id,epic_id,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        for (ordinal, sprint) in proposal.sprints.iter().enumerate() {
            let sprint_id = new_id("sprint");
            tx.execute("INSERT INTO initiated_sprints (id,epic_id,ordinal,title,intended_movement,concern_summaries_json,sprint_plan_id,sprint_plan_revision_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", params![sprint_id, epic_id, ordinal as i64, sprint.title, sprint.intended_movement, serde_json::to_string(&sprint.concern_summaries).unwrap(), new_id("sprint-plan"), new_id("sprint-plan-revision")]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        }
        tx.execute("INSERT INTO initiated_planning_drafts (draft_id,initiation_id,initiated_at) VALUES (?1,?2,?3)", params![command.epic_planning_draft_id.as_str(),initiation_id,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.commit()
            .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        Ok(InitiateEpicResult {
            initiation_id: EpicInitiationId::new(initiation_id)
                .map_err(InitiateEpicError::InvalidInput)?,
            epic_id: EpicId::new(epic_id).map_err(InitiateEpicError::InvalidInput)?,
            proposal_revision_id: ProposalRevisionId::new(revision_id)
                .map_err(InitiateEpicError::InvalidInput)?,
            material_snapshot_hash: hash,
            idempotent_replay: false,
        })
    }

    pub(crate) fn native_query(&self) -> Result<NativeQueryV2, String> {
        self.native_query_at(self.clock.now())
    }

    /// Captures the authorized optimistic precondition for one managed Agent Invocation. The
    /// captured value is never exposed to the agent; save rechecks it transactionally.
    pub(crate) fn capture_plan_builder_precondition(
        &self,
        draft_id: &EpicPlanningDraftId,
        profile_id: &CapabilityProfileId,
        association_id: &PlanningDraftAgentSessionAssociationId,
        agent_session_id: &str,
        actor_id: &str,
    ) -> Result<Option<String>, SaveProposalError> {
        let connection = self.lock()?;
        let now = timestamp(self.clock.now());
        let authorized = connection.query_row("SELECT 1 FROM epic_planning_drafts draft JOIN planning_draft_profile_assignments assignment ON assignment.draft_id = draft.id JOIN planning_draft_agent_session_associations association ON association.id = assignment.agent_session_association_id JOIN capability_profiles profile ON profile.id = assignment.capability_profile_id WHERE draft.id = ?1 AND draft.status = 'active' AND assignment.capability_profile_id = ?2 AND assignment.agent_session_association_id = ?3 AND association.agent_session_id = ?4 AND association.actor_id = ?5 AND assignment.expires_at >= ?6 AND profile.status = 'active'", params![draft_id.as_str(), profile_id.as_str(), association_id.as_str(), agent_session_id, actor_id, now], |_| Ok(())).optional().map_err(sql_error("authorize managed proposal precondition"))?.is_some();
        if !authorized {
            return Err(SaveProposalError::Forbidden);
        }
        connection.query_row("SELECT revision_token FROM proposal_revisions WHERE draft_id = ?1 ORDER BY recorded_at DESC, id DESC LIMIT 1", params![draft_id.as_str()], |row| row.get(0)).optional().map_err(sql_error("capture managed proposal precondition"))
    }

    /// Derives the current initiation precondition from the registered managed Agent Session.
    /// No product identity or optimistic token is accepted from the agent tool input.
    pub(crate) fn capture_agent_initiation_precondition(
        &self,
        draft_id: &EpicPlanningDraftId,
        profile_id: &CapabilityProfileId,
        association_id: &PlanningDraftAgentSessionAssociationId,
        agent_session_id: &str,
        actor_id: &str,
    ) -> Result<String, super::domain::InitiateEpicError> {
        use super::domain::InitiateEpicError;
        let connection = self.connection.lock().map_err(|_| {
            InitiateEpicError::Unavailable("orchestration database lock is poisoned".into())
        })?;
        let now = timestamp(self.clock.now());
        let status: Option<String> = connection
            .query_row(
                "SELECT status FROM epic_planning_drafts WHERE id = ?1",
                params![draft_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| InitiateEpicError::Unavailable(error.to_string()))?;
        match status.as_deref() {
            Some("active") => {}
            Some("canceled") => return Err(InitiateEpicError::Canceled),
            Some(_) => return Err(InitiateEpicError::AlreadyInitiated),
            None => return Err(InitiateEpicError::DraftNotFound),
        }
        let authorized = connection
            .query_row(
                "SELECT 1 FROM planning_draft_profile_assignments assignment JOIN planning_draft_agent_session_associations association ON association.id = assignment.agent_session_association_id JOIN capability_profiles profile ON profile.id = assignment.capability_profile_id WHERE assignment.draft_id = ?1 AND assignment.capability_profile_id = ?2 AND assignment.agent_session_association_id = ?3 AND association.draft_id = ?1 AND association.agent_session_id = ?4 AND association.actor_id = ?5 AND assignment.expires_at >= ?6 AND profile.status = 'active'",
                params![draft_id.as_str(), profile_id.as_str(), association_id.as_str(), agent_session_id, actor_id, now],
                |_| Ok(()),
            )
            .optional()
            .map_err(|error| InitiateEpicError::Unavailable(error.to_string()))?
            .is_some();
        if !authorized {
            return Err(InitiateEpicError::Forbidden);
        }
        connection
            .query_row(
                "SELECT revision_token FROM proposal_revisions WHERE draft_id = ?1 ORDER BY recorded_at DESC, id DESC LIMIT 1",
                params![draft_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| InitiateEpicError::Unavailable(error.to_string()))?
            .ok_or(InitiateEpicError::ProposalMissing)
    }

    pub(crate) fn initiation_is_projected(
        &self,
        initiation_id: &super::domain::EpicInitiationId,
    ) -> Result<bool, String> {
        Ok(self
            .native_query()?
            .initiated_epics
            .iter()
            .any(|epic| epic.initiation_id == initiation_id.as_str()))
    }

    /// Returns only the requested draft's semantic context after the same durable profile,
    /// association, actor, expiry, and active-profile checks used for mutations.
    pub(crate) fn plan_builder_context(
        &self,
        draft_id: &EpicPlanningDraftId,
        profile_id: &CapabilityProfileId,
        association_id: &PlanningDraftAgentSessionAssociationId,
        actor_id: &str,
    ) -> Result<serde_json::Value, SaveProposalError> {
        let connection = self.lock()?;
        let now = timestamp(self.clock.now());
        let exists = connection
            .query_row(
                "SELECT 1 FROM epic_planning_drafts WHERE id = ?1",
                params![draft_id.as_str()],
                |_| Ok(()),
            )
            .optional()
            .map_err(sql_error("read planning draft"))?
            .is_some();
        if !exists {
            return Err(SaveProposalError::DraftNotFound);
        }
        let authorized = connection.query_row("SELECT 1 FROM planning_draft_profile_assignments assignment JOIN planning_draft_agent_session_associations association ON association.id = assignment.agent_session_association_id JOIN capability_profiles profile ON profile.id = assignment.capability_profile_id WHERE assignment.draft_id = ?1 AND assignment.capability_profile_id = ?2 AND assignment.agent_session_association_id = ?3 AND association.actor_id = ?4 AND association.draft_id = ?1 AND assignment.expires_at >= ?5 AND profile.status = 'active'", params![draft_id.as_str(), profile_id.as_str(), association_id.as_str(), actor_id, now], |_| Ok(())).optional().map_err(sql_error("authorize planning context"))?.is_some();
        if !authorized {
            return Err(SaveProposalError::Forbidden);
        }
        let latest: Option<(String, String, String)> = connection.query_row("SELECT id, revision_token, proposal_json FROM proposal_revisions WHERE draft_id = ?1 ORDER BY recorded_at DESC, id DESC LIMIT 1", params![draft_id.as_str()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional().map_err(sql_error("read planning context"))?;
        let proposal = latest
            .as_ref()
            .map(|(_, _, json)| serde_json::from_str::<PlanBuilderProposal>(json))
            .transpose()
            .map_err(|error| {
                SaveProposalError::Unavailable(format!("read proposal context: {error}"))
            })?;
        Ok(
            serde_json::json!({ "epicPlanningDraftId": draft_id.as_str(), "currentProposal": latest.map(|(id, token, _)| serde_json::json!({"proposalRevisionId": id, "revisionToken": token, "proposal": proposal})) }),
        )
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, SaveProposalError> {
        self.connection.lock().map_err(|_| {
            SaveProposalError::Unavailable("orchestration database lock is poisoned".into())
        })
    }
}

fn find_command_result(
    transaction: &Transaction<'_>,
    idempotency_key: &str,
) -> Result<Option<(String, SaveProposalResult)>, SaveProposalError> {
    transaction.query_row("SELECT command.payload_fingerprint, command.id, result.id, result.revision_id, revision.revision_token, result.event_id, result.provenance_id FROM proposal_commands command JOIN proposal_command_results result ON result.command_id = command.id JOIN proposal_revisions revision ON revision.id = result.revision_id WHERE command.idempotency_key = ?1", params![idempotency_key], |row| Ok((row.get(0)?, SaveProposalResult { command_id: ProposalCommandId::new(row.get::<_, String>(1)?).map_err(to_sql_error)?, result_id: ProposalResultId::new(row.get::<_, String>(2)?).map_err(to_sql_error)?, revision_id: ProposalRevisionId::new(row.get::<_, String>(3)?).map_err(to_sql_error)?, revision_token: row.get(4)?, event_id: ProposalEventId::new(row.get::<_, String>(5)?).map_err(to_sql_error)?, provenance_id: EffectProvenanceId::new(row.get::<_, String>(6)?).map_err(to_sql_error)?, idempotent_replay: false }))).optional().map_err(sql_error("read idempotent proposal command"))
}

fn collect<T>(
    connection: &Connection,
    sql: &str,
    map: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, String> {
    connection
        .prepare(sql)
        .map_err(|error| error.to_string())?
        .query_map([], map)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}
fn fingerprint(command: &SaveEpicPlanProposalCommand) -> Result<String, SaveProposalError> {
    serde_json::to_string(&(
        command.epic_planning_draft_id.as_str(),
        command.capability_profile_id.as_str(),
        command.agent_session_association_id.as_str(),
        &command.agent_session_id,
        &command.actor_id,
        &command.expected_revision,
        &command.proposal,
    ))
    .map_err(|error| SaveProposalError::Unavailable(error.to_string()))
}
fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}
fn sql_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> SaveProposalError {
    move |error| SaveProposalError::Unavailable(format!("{context}: {error}"))
}
fn to_sql_error(error: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
    )
}

fn parse_proposal_json(value: String) -> rusqlite::Result<PlanBuilderProposal> {
    let proposal: PlanBuilderProposal =
        serde_json::from_str(&value).map_err(|error| to_sql_error(error.to_string()))?;
    proposal.validate().map_err(to_sql_error)?;
    Ok(proposal)
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeQueryV2 {
    contract_version: &'static str,
    generated_at: String,
    planning_drafts: Vec<PlanningDraftDto>,
    agent_session_associations: Vec<AgentSessionAssociationDto>,
    proposal_revisions: Vec<ProposalRevisionDto>,
    recorded_proposal_events: Vec<RecordedProposalEventDto>,
    provenance_links: Vec<ProvenanceLinkDto>,
    initiation_commands: Vec<InitiationCommandDto>,
    initiation_results: Vec<InitiationResultDto>,
    initiation_events: Vec<InitiationEventDto>,
    initiation_provenance: Vec<InitiationProvenanceDto>,
    material_snapshots: Vec<MaterialSnapshotDto>,
    initiated_epics: Vec<InitiatedEpicDto>,
    initiated_sprints: Vec<InitiatedSprintDto>,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanningDraftDto {
    epic_planning_draft_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    canceled_at: Option<String>,
    current_proposal: CurrentProposalDto,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentSessionAssociationDto {
    agent_session_association_id: String,
    epic_planning_draft_id: String,
    agent_session_id: String,
    actor_id: String,
    associated_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum CurrentProposalDto {
    Empty {},
    Available { proposal_revision_id: String },
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProposalRevisionDto {
    proposal_revision_id: String,
    epic_planning_draft_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_proposal_revision_id: Option<String>,
    revision_token: String,
    proposal: PlanBuilderProposal,
    command_id: String,
    provenance_id: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordedProposalEventDto {
    proposal_event_id: String,
    epic_planning_draft_id: String,
    proposal_revision_id: String,
    command_id: String,
    provenance_id: String,
    event_kind: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProvenanceLinkDto {
    provenance_id: String,
    source_kind: String,
    recorded_at: String,
    actor_id: String,
    agent_session_association_id: String,
    capability_profile_id: String,
    causal_command_id: String,
    causal_result_id: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiationCommandDto {
    command_id: String,
    epic_planning_draft_id: String,
    expected_revision_token: String,
    actor_id: String,
    idempotency_key: String,
    payload_fingerprint: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiationResultDto {
    result_id: String,
    command_id: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiationEventDto {
    event_id: String,
    command_id: String,
    result_id: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiationProvenanceDto {
    provenance_id: String,
    command_id: String,
    result_id: String,
    event_id: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MaterialSnapshotDto {
    material_snapshot_id: String,
    epic_planning_draft_id: String,
    proposal_revision_id: String,
    version: i64,
    proposal: PlanBuilderProposal,
    content_hash: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiatedEpicDto {
    initiation_id: String,
    epic_planning_draft_id: String,
    proposal_revision_id: String,
    material_snapshot_id: String,
    epic_id: String,
    recorded_at: String,
    command_id: String,
    result_id: String,
    event_id: String,
    provenance_id: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiatedSprintDto {
    sprint_id: String,
    epic_id: String,
    ordinal: i64,
    title: String,
    intended_movement: String,
    concern_summaries: Vec<String>,
    sprint_plan_id: String,
    sprint_plan_revision_id: String,
}

#[cfg(test)]
mod tests;
