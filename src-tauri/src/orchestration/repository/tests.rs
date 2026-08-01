use super::*;
use crate::orchestration::{
    application::OrchestrationApplication,
    domain::{
        CapabilityProfileId, EpicPlanningDraftId, PlanBuilderProposal,
        PlanningDraftAgentSessionAssociationId, ProposedSprint,
    },
};
use chrono::{TimeZone, Utc};
use std::sync::{Arc, Mutex};

const SESSION_ID: &str = "agent-session-plan-builder";
const ASSOCIATION_ID: &str = "planning-draft-agent-session-association-1";
const ACTOR_ID: &str = "managed-plan-builder";

#[test]
fn file_review_store_replays_exact_facts_and_reauthorizes_on_load() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("review"), "review-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "review-init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    let epic = &query.initiated_epics[0];
    let facts = StoreFileReviewFacts {
        document_ref_id: "review-document".into(),
        epic_id: epic.epic_id.clone(),
        sprint_id: query.initiated_sprints[0].sprint_id.clone(),
        provenance_id: epic.provenance_id.clone(),
        opaque_reference: "opaque-review".into(),
        title: "Changed files".into(),
        summary: None,
        artifact_id: "review-artifact".into(),
        payload: b"{}".to_vec(),
        idempotency_key: "review-store".into(),
        changed_files: vec![FileReviewChangedFileWrite {
            changed_file_reference_id: "changed-1".into(),
            display_name: "src/a.ts".into(),
            change_kind: "modified".into(),
            previous_display_name: None,
        }],
    };
    assert_eq!(
        repository.store_file_review_facts(facts.clone()).unwrap(),
        StoreFileReviewFactsResult::Stored
    );
    assert_eq!(
        repository.store_file_review_facts(facts).unwrap(),
        StoreFileReviewFactsResult::IdempotentReplay
    );
    match repository.load_scoped_file_review("opaque-review").unwrap() {
        ScopedFileReviewLoad::Available { document } => {
            assert_eq!(document.artifact_id, "review-artifact");
            assert_eq!(
                document.changed_files[0].changed_file_reference_id,
                "changed-1"
            );
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn git_capture_authorization_is_private_replay_safe_and_reauthorized() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("review"), "save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    let epic = &query.initiated_epics[0];
    let value = FileReviewGitCaptureAuthorizationWrite {
        capture_authorization_id: "capture-1".into(),
        idempotency_key: "capture-key".into(),
        epic_id: epic.epic_id.clone(),
        sprint_id: query.initiated_sprints[0].sprint_id.clone(),
        provenance_id: epic.provenance_id.clone(),
        repository_id: "repository-1".into(),
        repository_root: "C:\\repo".into(),
        worktree_id: "worktree-1".into(),
        worktree_root: "C:\\repo\\worktree".into(),
        baseline_object_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        current_object_id: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
    };
    assert_eq!(
        repository
            .store_file_review_git_capture_authorization(value.clone())
            .unwrap(),
        StoreFileReviewGitCaptureAuthorizationResult::Stored
    );
    assert_eq!(
        repository
            .store_file_review_git_capture_authorization(value.clone())
            .unwrap(),
        StoreFileReviewGitCaptureAuthorizationResult::IdempotentReplay
    );
    let loaded = repository
        .load_file_review_git_capture_authorization("capture-1")
        .unwrap()
        .unwrap();
    assert_eq!(loaded.worktree_root, "C:\\repo\\worktree");
    assert!(!serde_json::to_string(&repository.native_query().unwrap())
        .unwrap()
        .contains("C:\\repo"));
    let mut conflict = value;
    conflict.current_object_id = "cccccccccccccccccccccccccccccccccccccccc".into();
    assert!(matches!(
        repository.store_file_review_git_capture_authorization(conflict),
        Err(FileReviewGitCaptureAuthorizationError::Conflict)
    ));
}

#[test]
fn file_review_store_rejects_wrong_provenance_without_writing() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("review"), "review-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "review-init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    let result = repository.store_file_review_facts(StoreFileReviewFacts {
        document_ref_id: "bad-document".into(),
        epic_id: query.initiated_epics[0].epic_id.clone(),
        sprint_id: query.initiated_sprints[0].sprint_id.clone(),
        provenance_id: "wrong-provenance".into(),
        opaque_reference: "opaque-bad".into(),
        title: "Changed files".into(),
        summary: None,
        artifact_id: "bad-artifact".into(),
        payload: b"{}".to_vec(),
        idempotency_key: "bad-store".into(),
        changed_files: vec![FileReviewChangedFileWrite {
            changed_file_reference_id: "changed-1".into(),
            display_name: "src/a.ts".into(),
            change_kind: "modified".into(),
            previous_display_name: None,
        }],
    });
    assert!(matches!(result, Err(FileReviewFactsError::Forbidden)));
    assert!(matches!(
        repository.load_scoped_file_review("opaque-bad").unwrap(),
        ScopedFileReviewLoad::Unavailable
    ));
}

#[test]
fn scoped_file_review_distinguishes_unknown_invalid_and_broken_membership() {
    let repository = repository_at(time());
    assert!(matches!(
        repository.load_scoped_file_review("missing").unwrap(),
        ScopedFileReviewLoad::Unavailable
    ));
    assert!(matches!(
        repository.load_scoped_file_review(" ").unwrap(),
        ScopedFileReviewLoad::Invalid
    ));
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("review"), "status-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "status-init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    repository
        .store_file_review_facts(StoreFileReviewFacts {
            document_ref_id: "status-document".into(),
            epic_id: query.initiated_epics[0].epic_id.clone(),
            sprint_id: query.initiated_sprints[0].sprint_id.clone(),
            provenance_id: query.initiated_epics[0].provenance_id.clone(),
            opaque_reference: "opaque-status".into(),
            title: "Changed files".into(),
            summary: None,
            artifact_id: "status-artifact".into(),
            payload: b"{}".to_vec(),
            idempotency_key: "status-store".into(),
            changed_files: vec![FileReviewChangedFileWrite {
                changed_file_reference_id: "changed-1".into(),
                display_name: "src/a.ts".into(),
                change_kind: "modified".into(),
                previous_display_name: None,
            }],
        })
        .unwrap();
    repository
        .lock()
        .unwrap()
        .execute(
            "DELETE FROM stored_file_review_artifacts WHERE artifact_id='status-artifact'",
            [],
        )
        .unwrap();
    assert!(matches!(
        repository.load_scoped_file_review("opaque-status").unwrap(),
        ScopedFileReviewLoad::Unauthorized
    ));
}

#[test]
fn file_review_rejects_a_valid_other_epic_provenance_and_omits_it_from_query() {
    let repository = repository_at(time());
    let first = repository
        .save_epic_plan_proposal(command(None, proposal("one"), "one-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: first.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "one-init".into(),
        })
        .unwrap();
    let draft = EpicPlanningDraftId::new("epic-planning-draft-2").unwrap();
    let profile = CapabilityProfileId::new("capability-profile-2").unwrap();
    let association =
        PlanningDraftAgentSessionAssociationId::new("planning-draft-agent-session-association-2")
            .unwrap();
    repository.create_planning_draft(&draft, time()).unwrap();
    repository
        .create_capability_profile(&profile, "active", time())
        .unwrap();
    repository
        .associate_agent_session(&association, &draft, SESSION_ID, ACTOR_ID, time())
        .unwrap();
    repository
        .assign_profile(&draft, &profile, &association, at(2030, 1, 1), time())
        .unwrap();
    let second = repository
        .save_epic_plan_proposal(SaveEpicPlanProposalCommand {
            epic_planning_draft_id: draft.clone(),
            capability_profile_id: profile,
            agent_session_association_id: association,
            agent_session_id: SESSION_ID.into(),
            actor_id: ACTOR_ID.into(),
            expected_revision: None,
            proposal: proposal("two"),
            idempotency_key: "two-save".into(),
        })
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: draft,
            expected_revision_token: second.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "two-init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    let first_epic = &query.initiated_epics[0];
    repository
        .store_file_review_facts(StoreFileReviewFacts {
            document_ref_id: "tamper-document".into(),
            epic_id: first_epic.epic_id.clone(),
            sprint_id: query
                .initiated_sprints
                .iter()
                .find(|x| x.epic_id == first_epic.epic_id)
                .unwrap()
                .sprint_id
                .clone(),
            provenance_id: first_epic.provenance_id.clone(),
            opaque_reference: "opaque-tamper".into(),
            title: "Changed files".into(),
            summary: None,
            artifact_id: "tamper-artifact".into(),
            payload: b"{}".to_vec(),
            idempotency_key: "tamper-store".into(),
            changed_files: vec![FileReviewChangedFileWrite {
                changed_file_reference_id: "changed".into(),
                display_name: "src/a.ts".into(),
                change_kind: "modified".into(),
                previous_display_name: None,
            }],
        })
        .unwrap();
    let other = query
        .initiated_epics
        .iter()
        .find(|x| x.epic_id != first_epic.epic_id)
        .unwrap()
        .provenance_id
        .clone();
    let connection = repository.lock().unwrap();
    connection.execute("UPDATE file_review_documents SET provenance_id=?1 WHERE document_ref_id='tamper-document'", [&other]).unwrap();
    connection.execute("UPDATE stored_file_review_artifacts SET provenance_id=?1 WHERE artifact_id='tamper-artifact'", [&other]).unwrap();
    drop(connection);
    assert!(matches!(
        repository.load_scoped_file_review("opaque-tamper").unwrap(),
        ScopedFileReviewLoad::Unauthorized
    ));
    assert!(!serde_json::to_string(&repository.native_query().unwrap())
        .unwrap()
        .contains("tamper-document"));
}

#[test]
fn initiation_is_atomic_idempotent_and_preserves_the_consumed_revision() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("init"), "save"))
        .unwrap();
    let initiate = |key: &str, token: String| super::super::domain::InitiateEpicCommand {
        epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
        expected_revision_token: token,
        actor_id: "application-user".into(),
        idempotency_key: key.into(),
    };
    let result = repository
        .initiate_epic(initiate("init", saved.revision_token.clone()))
        .unwrap();
    let retry = repository
        .initiate_epic(initiate("init", saved.revision_token.clone()))
        .unwrap();
    assert_eq!(result.epic_id, retry.epic_id);
    assert!(retry.idempotent_replay);
    assert!(matches!(
        repository.initiate_epic(initiate("init", "different-revision-token".into())),
        Err(super::super::domain::InitiateEpicError::IdempotencyConflict)
    ));
    assert!(matches!(
        repository.initiate_epic(initiate("other", saved.revision_token)),
        Err(super::super::domain::InitiateEpicError::AlreadyInitiated)
    ));
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("after"), "after")),
        Err(SaveProposalError::DraftNotFound)
    ));
    let query = repository.native_query().expect("query");
    assert_eq!(query.planning_drafts[0].status, "initiated");
    let connection = repository.lock().unwrap();
    for table in [
        "epic_initiation_commands",
        "epic_initiation_results",
        "epic_initiation_events",
        "epic_initiation_provenance",
        "epic_initiation_material_snapshots",
        "epic_initiations",
        "initiated_sprints",
    ] {
        assert_eq!(
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            if table == "initiated_sprints" { 1 } else { 1 }
        );
    }
}

#[test]
fn button_context_claim_keeps_stable_invocation_identity_until_explicit_reconciliation() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("context"), "context-save"))
        .unwrap();
    let initiation = repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "context-initiation".into(),
        })
        .unwrap();
    repository
        .schedule_button_initiation_context(&initiation)
        .unwrap();
    repository
        .schedule_button_initiation_context(&initiation)
        .unwrap();

    let first = repository
        .claim_pending_plan_builder_context(
            SESSION_ID,
            "claim-before-restart",
            "context-invocation-before-restart",
        )
        .unwrap()
        .unwrap();
    assert!(repository
        .claim_pending_plan_builder_context(
            SESSION_ID,
            "claim-after-restart",
            "must-not-replace-target",
        )
        .is_err());
    let recovered = repository
        .load_claimed_plan_builder_context(SESSION_ID)
        .unwrap()
        .unwrap();
    assert_eq!(first, recovered);
    assert_eq!(recovered.initiation_id, initiation.initiation_id.as_str());
    assert_eq!(recovered.epic_id, initiation.epic_id.as_str());

    repository.release_plan_builder_context(&recovered).unwrap();
    let delivery = repository
        .claim_pending_plan_builder_context(SESSION_ID, "delivery-claim", "context-invocation")
        .unwrap()
        .unwrap();
    repository
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO agent_session_invocations
             (id,session_id,submitted_text,input_provenance,status,requested_options_json,started_at,created_at,updated_at)
             VALUES ('context-invocation',?1,'original application text','application','running','{}','t','t','t')",
            params![SESSION_ID],
        )
        .unwrap();
    repository.consume_plan_builder_context(&delivery).unwrap();
    assert!(repository
        .claim_pending_plan_builder_context(
            SESSION_ID,
            "must-not-redeliver",
            "must-not-redeliver-invocation",
        )
        .unwrap()
        .is_none());
    let facts: (String, String, String, String, String) = repository
        .lock()
        .unwrap()
        .query_row(
            "SELECT source_kind,requested_at,pending_at,delivered_at,consumed_at
             FROM plan_builder_context_deliveries",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(facts.0, "button_initiation");
    assert!(facts.1 <= facts.2 && facts.2 <= facts.3 && facts.3 <= facts.4);
}

#[test]
fn initiated_draft_is_terminal_for_managed_draft_mutations() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("terminal"), "terminal-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "terminal-initiation".into(),
        })
        .unwrap();
    let draft = EpicPlanningDraftId::new("epic-planning-draft-1").unwrap();

    assert!(matches!(
        repository.update_planning_draft_title(
            &draft,
            SESSION_ID,
            Some("Late title"),
            "late-title"
        ),
        Err(SaveProposalError::Forbidden)
    ));
    assert!(matches!(
        repository.cancel_planning_draft(&draft, SESSION_ID, "late-cancel"),
        Err(SaveProposalError::Forbidden)
    ));
    assert!(repository
        .bootstrap_managed_plan_builder(SESSION_ID)
        .is_ok());
    let query = repository.native_query_at(time()).expect("query");
    assert_eq!(query.planning_drafts[0].status, "initiated");
    assert!(query.planning_drafts[0].canceled_at.is_none());
}

#[test]
fn initiation_failure_matrix_leaves_no_false_initiation_root() {
    let initiate =
        |token: String, actor: &str, key: &str| super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: token,
            actor_id: actor.into(),
            idempotency_key: key.into(),
        };
    let repository = repository_at(time());
    let mut missing_draft = initiate("missing".into(), "application-user", "missing-draft");
    missing_draft.epic_planning_draft_id = EpicPlanningDraftId::new("missing-draft").unwrap();
    assert!(matches!(
        repository.initiate_epic(missing_draft),
        Err(super::super::domain::InitiateEpicError::DraftNotFound)
    ));
    assert!(matches!(
        repository.initiate_epic(initiate(
            "missing".into(),
            "application-user",
            "missing-proposal"
        )),
        Err(super::super::domain::InitiateEpicError::ProposalMissing)
    ));
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("first"), "first"))
        .unwrap();
    assert!(matches!(
        repository.initiate_epic(initiate("stale".into(), "application-user", "stale")),
        Err(super::super::domain::InitiateEpicError::RevisionConflict)
    ));
    assert!(matches!(
        repository.initiate_epic(initiate(
            saved.revision_token.clone(),
            "caller",
            "forbidden"
        )),
        Err(super::super::domain::InitiateEpicError::Forbidden)
    ));
    let draft = EpicPlanningDraftId::new("epic-planning-draft-1").unwrap();
    repository
        .cancel_planning_draft(&draft, SESSION_ID, "cancel-before-initiation")
        .unwrap();
    assert!(matches!(
        repository.initiate_epic(initiate(
            saved.revision_token,
            "application-user",
            "canceled"
        )),
        Err(super::super::domain::InitiateEpicError::Canceled)
    ));

    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("rollback"), "rollback-save"))
        .unwrap();
    repository.lock().unwrap().execute_batch("CREATE TRIGGER reject_initiated_sprint BEFORE INSERT ON initiated_sprints BEGIN SELECT RAISE(ABORT, 'induced rollback'); END;").unwrap();
    assert!(
        matches!(repository.initiate_epic(initiate(saved.revision_token, "application-user", "rollback")), Err(super::super::domain::InitiateEpicError::Unavailable(message)) if message.contains("induced rollback"))
    );
    let connection = repository.lock().unwrap();
    for table in [
        "epic_initiation_commands",
        "epic_initiation_results",
        "epic_initiation_events",
        "epic_initiation_provenance",
        "epic_initiation_material_snapshots",
        "epic_initiations",
        "initiated_planning_drafts",
        "initiated_sprints",
    ] {
        assert_eq!(
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0,
            "{table} must roll back"
        );
    }
}

#[test]
fn draft_title_and_cancellation_are_durable_idempotent_and_block_further_effects() {
    let repository = repository_at(time());
    let draft = EpicPlanningDraftId::new("epic-planning-draft-1").expect("draft");
    repository
        .update_planning_draft_title(&draft, SESSION_ID, Some("Separate draft title"), "title-1")
        .expect("title");
    repository
        .update_planning_draft_title(&draft, SESSION_ID, Some("Separate draft title"), "title-1")
        .expect("title retry");
    repository
        .cancel_planning_draft(&draft, SESSION_ID, "cancel-1")
        .expect("cancel");
    repository
        .cancel_planning_draft(&draft, SESSION_ID, "cancel-1")
        .expect("cancel retry");
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("denied"), "save-after-cancel")),
        Err(SaveProposalError::DraftNotFound)
    ));
    assert!(matches!(
        repository.bootstrap_managed_plan_builder(SESSION_ID),
        Err(SaveProposalError::Forbidden)
    ));
    let query = repository.native_query().expect("query");
    assert_eq!(
        query.planning_drafts[0].title.as_deref(),
        Some("Separate draft title")
    );
    assert_eq!(query.planning_drafts[0].status, "canceled");
    assert!(query.planning_drafts[0].canceled_at.is_some());
}

#[test]
fn bootstrap_is_idempotent_per_session_and_allocates_distinct_drafts() {
    let repository = repository_at(time());
    {
        let connection = repository.lock().expect("connection");
        connection.execute("INSERT INTO agent_sessions (id, title, availability, requested_options_json, created_at, updated_at) VALUES ('second-session', 'Second', 'available', '{}', ?1, ?1)", params![timestamp(time())]).expect("second session");
    }
    let first = repository
        .bootstrap_managed_plan_builder(SESSION_ID)
        .expect("first binding");
    let first_retry = repository
        .bootstrap_managed_plan_builder(SESSION_ID)
        .expect("first retry");
    let second = repository
        .bootstrap_managed_plan_builder("second-session")
        .expect("second binding");
    assert_eq!(first, first_retry);
    assert_ne!(first.0, second.0);
    assert_ne!(first.2, second.2);
}

#[test]
fn saves_typed_proposals_revises_and_retries_without_duplicate_effects() {
    let repository = Arc::new(repository_at(time()));
    let application = OrchestrationApplication::new(repository.clone());
    let first = application
        .save_epic_plan_proposal(command(None, proposal("one"), "key-1"))
        .expect("first save");
    let duplicate = application
        .save_epic_plan_proposal(command(None, proposal("one"), "key-1"))
        .expect("duplicate retry");
    assert_eq!(duplicate.revision_id, first.revision_id);
    assert_eq!(duplicate.revision_token, first.revision_token);
    assert!(!first.idempotent_replay);
    assert!(duplicate.idempotent_replay);
    let second = application
        .save_epic_plan_proposal(command(
            Some(first.revision_token.clone()),
            proposal("two"),
            "key-2",
        ))
        .expect("revision save");
    assert_ne!(second.revision_id, first.revision_id);
    let query = repository.native_query_at(time()).expect("query");
    assert_eq!(query.proposal_revisions.len(), 2);
    assert!(query
        .proposal_revisions
        .iter()
        .any(|revision| revision.proposal == proposal("one")));
    assert!(query
        .proposal_revisions
        .iter()
        .any(|revision| revision.proposal == proposal("two")));
    assert_eq!(query.recorded_proposal_events.len(), 2);
    assert_eq!(query.provenance_links.len(), 2);
}

#[test]
fn rejects_collisions_stale_malformed_and_unauthorized_requests_without_writes() {
    let repository = repository_at(time());
    let first = repository
        .save_epic_plan_proposal(command(None, proposal("one"), "key-1"))
        .expect("first save");
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("changed"), "key-1")),
        Err(SaveProposalError::IdempotencyConflict)
    ));
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("two"), "key-2")),
        Err(SaveProposalError::RevisionConflict)
    ));
    let mut forbidden = command(Some(first.revision_token.clone()), proposal("two"), "key-3");
    forbidden.agent_session_association_id =
        PlanningDraftAgentSessionAssociationId::new("wrong-association").expect("id");
    assert!(matches!(
        repository.save_epic_plan_proposal(forbidden),
        Err(SaveProposalError::Forbidden)
    ));
    let mut cross_session = command(
        Some(first.revision_token.clone()),
        proposal("cross-session"),
        "key-cross-session",
    );
    cross_session.agent_session_id = "another-session".into();
    assert!(matches!(
        repository.save_epic_plan_proposal(cross_session),
        Err(SaveProposalError::Forbidden)
    ));
    let mut malformed = command(
        Some(first.revision_token.clone()),
        PlanBuilderProposal {
            suggested_epic_name: Some(" ".into()),
            sprints: vec![],
        },
        "key-4",
    );
    malformed.proposal.suggested_epic_name = Some(" ".into());
    assert!(matches!(
        repository.save_epic_plan_proposal(malformed),
        Err(SaveProposalError::InvalidInput(_))
    ));
    let mut oversized = proposal("oversized");
    oversized.sprints[0].title = "x".repeat(241);
    assert!(matches!(
        repository.save_epic_plan_proposal(command(Some(first.revision_token), oversized, "key-5")),
        Err(SaveProposalError::InvalidInput(_))
    ));
    let query = repository.native_query_at(time()).expect("query");
    assert_eq!(query.proposal_revisions.len(), 1);
    assert_eq!(query.recorded_proposal_events.len(), 1);
}

#[test]
fn trusted_time_blocks_backdating_and_denies_expired_duplicate_retries() {
    let clock = Arc::new(MutableClock::new(at(2030, 1, 1)));
    let repository = repository_with_clock(clock.clone(), at(2029, 1, 1));
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("backdated"), "backdate")),
        Err(SaveProposalError::Forbidden)
    ));

    let clock = Arc::new(MutableClock::new(time()));
    let repository = repository_with_clock(clock.clone(), at(2030, 1, 1));
    let first = repository
        .save_epic_plan_proposal(command(None, proposal("one"), "key-1"))
        .expect("save while authorized");
    clock.set(at(2031, 1, 1));
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("one"), "key-1")),
        Err(SaveProposalError::Forbidden)
    ));
    let query = repository.native_query_at(at(2031, 1, 1)).expect("query");
    assert_eq!(query.proposal_revisions.len(), 1);
    assert_eq!(
        query.provenance_links[0].recorded_at,
        "2026-07-15T12:00:00.000Z"
    );
    assert_eq!(
        query.provenance_links[0].causal_command_id,
        first.command_id.as_str()
    );

    let repository = repository_at(time());
    repository
        .save_epic_plan_proposal(command(None, proposal("disabled"), "disabled-key"))
        .expect("save while active");
    repository
        .connection
        .lock()
        .expect("database")
        .execute(
            "UPDATE capability_profiles SET status = 'disabled' WHERE id = 'capability-profile-1'",
            [],
        )
        .expect("disable profile");
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("disabled"), "disabled-key")),
        Err(SaveProposalError::Forbidden)
    ));
}

#[test]
fn association_identity_is_independent_from_the_provider_neutral_agent_session_id() {
    let repository = repository_at(time());
    let query = repository.native_query_at(time()).expect("query");
    assert_eq!(query.agent_session_associations.len(), 1);
    let association = &query.agent_session_associations[0];
    assert_eq!(association.agent_session_association_id, ASSOCIATION_ID);
    assert_eq!(association.agent_session_id, SESSION_ID);
    assert_ne!(
        association.agent_session_association_id,
        association.agent_session_id
    );
}

#[test]
fn query_has_explicit_empty_state_and_matches_canonical_empty_fixture() {
    let json = serde_json::to_value(
        &repository_at(time())
            .native_query_at(time())
            .expect("query"),
    )
    .expect("json");
    assert_eq!(
        json,
        current_native_fixture(include_str!(
            "../fixtures/orchestration-native-query-v2/valid-empty.json"
        ))
        .expect("fixture")
    );
}

#[test]
fn populated_native_query_and_rejected_boundaries_match_rust_golden_fixtures() {
    let json = serde_json::to_value(&canonical_populated_query()).expect("json");
    assert_eq!(
        json,
        current_native_fixture(include_str!(
            "../fixtures/orchestration-native-query-v2/valid-proposal.json"
        ))
        .expect("fixture")
    );
    for (error, fixture) in [
        (
            SaveProposalError::RevisionConflict,
            include_str!("../fixtures/orchestration-native-query-v2/rejected-stale-revision.json"),
        ),
        (
            SaveProposalError::IdempotencyConflict,
            include_str!(
                "../fixtures/orchestration-native-query-v2/rejected-idempotency-conflict.json"
            ),
        ),
        (
            SaveProposalError::Forbidden,
            include_str!("../fixtures/orchestration-native-query-v2/rejected-forbidden.json"),
        ),
    ] {
        assert_eq!(
            serde_json::to_value(GoldenBoundaryDto::from(error)).expect("json"),
            serde_json::from_str::<serde_json::Value>(fixture).expect("fixture")
        );
    }
}

#[test]
fn initiated_native_query_serialization_matches_the_frontend_golden_fixture() {
    assert_eq!(
        serde_json::to_value(&canonical_initiated_query()).expect("json"),
        current_native_fixture(include_str!(
            "../fixtures/orchestration-native-query-v2/valid-initiated-epic.json"
        ))
        .expect("fixture")
    );
}

#[test]
fn restart_preserves_query_without_product_acceptance_or_initiated_identity() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("unified.sqlite");
    let connection = initialized_connection(&path);
    let clock = Arc::new(MutableClock::new(time()));
    let repository =
        SqliteOrchestrationRepository::new_with_clock(connection, clock).expect("repository");
    seed(&repository, at(2030, 1, 1));
    repository
        .save_epic_plan_proposal(command(None, proposal("one"), "key-1"))
        .expect("save");
    let before =
        serde_json::to_string(&repository.native_query_at(time()).expect("query")).expect("json");
    drop(repository);
    let after = serde_json::to_string(
        &SqliteOrchestrationRepository::open(&path)
            .expect("reopen")
            .native_query_at(time())
            .expect("query"),
    )
    .expect("json");
    assert_eq!(before, after);
    assert!(!after.contains("accepted"));
    assert!(after.contains("\"initiatedEpics\":[]"));
    assert!(!after.contains("sprintId"));
}

#[test]
fn restart_preserves_initiated_epic_and_ordered_preparatory_sprints() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("unified.sqlite");
    let connection = crate::storage::open_active_database(&path).expect("active database");
    seed_session(&connection);
    let clock = Arc::new(MutableClock::new(time()));
    let repository =
        SqliteOrchestrationRepository::new_with_clock(connection, clock).expect("repository");
    seed(&repository, at(2030, 1, 1));
    let mut submitted = proposal("restart");
    submitted.sprints.push(ProposedSprint {
        title: "Second preparatory Sprint".into(),
        intended_movement: "Keep the persisted proposal order.".into(),
        concern_summaries: vec!["No execution was initiated.".into()],
    });
    let saved = repository
        .save_epic_plan_proposal(command(None, submitted, "restart-save"))
        .expect("save proposal");
    let initiated = repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "restart-initiation".into(),
        })
        .expect("initiate");
    let before = repository.native_query_at(time()).expect("query");
    drop(repository);

    drop(crate::storage::open_active_database(&path).expect("reopen active database"));
    let after = SqliteOrchestrationRepository::open(&path)
        .expect("reopen repository")
        .native_query_at(time())
        .expect("query");
    assert_eq!(
        serde_json::to_value(&after).expect("json"),
        serde_json::to_value(&before).expect("json")
    );
    assert_eq!(after.planning_drafts[0].status, "initiated");
    assert_eq!(after.initiated_epics[0].epic_id, initiated.epic_id.as_str());
    assert_eq!(after.material_snapshots.len(), 1);
    assert_eq!(after.initiated_sprints.len(), 2);
    assert_eq!(after.initiated_sprints[0].ordinal, 0);
    assert_eq!(after.initiated_sprints[1].ordinal, 1);
    assert_eq!(after.initiated_sprints[0].title, "Sprint restart");
    assert_eq!(
        after.initiated_sprints[1].title,
        "Second preparatory Sprint"
    );
}

fn repository_at(now: chrono::DateTime<Utc>) -> SqliteOrchestrationRepository {
    repository_with_clock(Arc::new(MutableClock::new(now)), at(2030, 1, 1))
}
fn repository_with_clock(
    clock: Arc<MutableClock>,
    expiry: chrono::DateTime<Utc>,
) -> SqliteOrchestrationRepository {
    let connection = initialized_connection_in_memory();
    let repository =
        SqliteOrchestrationRepository::new_with_clock(connection, clock).expect("repository");
    seed(&repository, expiry);
    repository
}
fn initialized_connection(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).expect("open");
    initialize_connection(&connection);
    connection
}
fn initialized_connection_in_memory() -> Connection {
    let connection = Connection::open_in_memory().expect("database");
    initialize_connection(&connection);
    connection
}
fn initialize_connection(connection: &Connection) {
    crate::storage::configure_sqlite_connection(connection).expect("policy");
    connection
        .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
        .expect("session schema");
    connection
        .execute_batch(ORCHESTRATION_SCHEMA)
        .expect("orchestration schema");
    connection
        .execute_batch(ORCHESTRATION_INITIATION_SCHEMA)
        .expect("initiation schema");
    connection
        .execute_batch(PLAN_BUILDER_CONTEXT_DELIVERY_SCHEMA)
        .expect("context delivery schema");
    connection
        .execute_batch(FILE_REVIEW_FACTS_SCHEMA)
        .expect("File Review facts schema");
    seed_session(connection);
}
fn seed(repository: &SqliteOrchestrationRepository, expiry: chrono::DateTime<Utc>) {
    let draft = EpicPlanningDraftId::new("epic-planning-draft-1").expect("draft");
    let profile = CapabilityProfileId::new("capability-profile-1").expect("profile");
    let association =
        PlanningDraftAgentSessionAssociationId::new(ASSOCIATION_ID).expect("association");
    repository
        .create_planning_draft(&draft, time())
        .expect("draft");
    repository
        .create_capability_profile(&profile, "active", time())
        .expect("profile");
    repository
        .associate_agent_session(&association, &draft, SESSION_ID, ACTOR_ID, time())
        .expect("association");
    repository
        .assign_profile(&draft, &profile, &association, expiry, time())
        .expect("assignment");
}
fn seed_session(connection: &Connection) {
    connection.execute("INSERT INTO agent_sessions (id, title, availability, requested_options_json, created_at, updated_at) VALUES (?1, 'Plan Builder session', 'available', '{}', ?2, ?2)", params![SESSION_ID, timestamp(time())]).expect("session");
}
fn command(
    expected_revision: Option<String>,
    proposal: PlanBuilderProposal,
    key: &str,
) -> SaveEpicPlanProposalCommand {
    SaveEpicPlanProposalCommand {
        epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").expect("draft"),
        capability_profile_id: CapabilityProfileId::new("capability-profile-1").expect("profile"),
        agent_session_association_id: PlanningDraftAgentSessionAssociationId::new(ASSOCIATION_ID)
            .expect("association"),
        agent_session_id: SESSION_ID.into(),
        actor_id: ACTOR_ID.into(),
        expected_revision,
        proposal,
        idempotency_key: key.into(),
    }
}
fn proposal(suffix: &str) -> PlanBuilderProposal {
    PlanBuilderProposal {
        suggested_epic_name: Some(format!("Suggested Epic {suffix}")),
        sprints: vec![ProposedSprint {
            title: format!("Sprint {suffix}"),
            intended_movement: format!("Move {suffix} forward."),
            concern_summaries: vec![format!("Concern {suffix}.")],
        }],
    }
}
fn time() -> chrono::DateTime<Utc> {
    at(2026, 7, 15)
}
fn at(year: i32, month: u32, day: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 12, 0, 0)
        .single()
        .expect("time")
}

struct MutableClock(Mutex<chrono::DateTime<Utc>>);
impl MutableClock {
    fn new(now: chrono::DateTime<Utc>) -> Self {
        Self(Mutex::new(now))
    }
    fn set(&self, now: chrono::DateTime<Utc>) {
        *self.0.lock().expect("clock") = now;
    }
}
impl OrchestrationClock for MutableClock {
    fn now(&self) -> chrono::DateTime<Utc> {
        *self.0.lock().expect("clock")
    }
}

fn canonical_populated_query() -> NativeQueryV2 {
    NativeQueryV2 {
        contract_version: NATIVE_QUERY_VERSION,
        generated_at: "2026-07-15T12:00:00.000Z".into(),
        planning_drafts: vec![PlanningDraftDto {
            epic_planning_draft_id: "epic-planning-draft-fixture".into(),
            title: None,
            status: "active".into(),
            created_at: "2026-07-15T12:00:00.000Z".into(),
            updated_at: "2026-07-15T12:00:00.000Z".into(),
            canceled_at: None,
            current_proposal: CurrentProposalDto::Available {
                proposal_revision_id: "proposal-revision-fixture".into(),
            },
        }],
        agent_session_associations: vec![AgentSessionAssociationDto {
            agent_session_association_id: "association-fixture".into(),
            epic_planning_draft_id: "epic-planning-draft-fixture".into(),
            agent_session_id: "agent-session-fixture".into(),
            actor_id: "managed-plan-builder".into(),
            associated_at: "2026-07-15T12:00:00.000Z".into(),
        }],
        proposal_revisions: vec![ProposalRevisionDto {
            proposal_revision_id: "proposal-revision-fixture".into(),
            epic_planning_draft_id: "epic-planning-draft-fixture".into(),
            parent_proposal_revision_id: None,
            revision_token: "proposal-token-fixture".into(),
            proposal: proposal("fixture"),
            command_id: "proposal-command-fixture".into(),
            provenance_id: "provenance-fixture".into(),
            recorded_at: "2026-07-15T12:00:00.000Z".into(),
        }],
        recorded_proposal_events: vec![RecordedProposalEventDto {
            proposal_event_id: "proposal-event-fixture".into(),
            epic_planning_draft_id: "epic-planning-draft-fixture".into(),
            proposal_revision_id: "proposal-revision-fixture".into(),
            command_id: "proposal-command-fixture".into(),
            provenance_id: "provenance-fixture".into(),
            event_kind: "proposal_saved".into(),
            recorded_at: "2026-07-15T12:00:00.000Z".into(),
        }],
        provenance_links: vec![ProvenanceLinkDto {
            provenance_id: "provenance-fixture".into(),
            source_kind: "managed_plan_builder".into(),
            recorded_at: "2026-07-15T12:00:00.000Z".into(),
            actor_id: "managed-plan-builder".into(),
            agent_session_association_id: "association-fixture".into(),
            capability_profile_id: "capability-profile-fixture".into(),
            causal_command_id: "proposal-command-fixture".into(),
            causal_result_id: "proposal-result-fixture".into(),
        }],
        initiation_commands: vec![],
        initiation_results: vec![],
        initiation_events: vec![],
        initiation_provenance: vec![],
        material_snapshots: vec![],
        initiated_epics: vec![],
        initiated_sprints: vec![],
        file_review_documents: vec![],
    }
}

fn current_native_fixture(value: &str) -> Result<serde_json::Value, serde_json::Error> {
    let mut fixture = serde_json::from_str::<serde_json::Value>(value)?;
    fixture
        .as_object_mut()
        .unwrap()
        .insert("fileReviewDocuments".into(), serde_json::json!([]));
    Ok(fixture)
}

fn canonical_initiated_query() -> NativeQueryV2 {
    let mut query = canonical_populated_query();
    let mut proposal = proposal("fixture");
    proposal.sprints.push(ProposedSprint {
        title: "Second Sprint fixture".into(),
        intended_movement: "Move second fixture forward.".into(),
        concern_summaries: vec!["Second concern fixture.".into()],
    });
    query.planning_drafts[0].status = "initiated".into();
    query.proposal_revisions[0].proposal = proposal.clone();
    query.initiation_commands = vec![InitiationCommandDto {
        command_id: "init-command-fixture".into(),
        epic_planning_draft_id: "epic-planning-draft-fixture".into(),
        expected_revision_token: "proposal-token-fixture".into(),
        actor_id: "application-user".into(),
        idempotency_key: "initiate:epic-planning-draft-fixture:proposal-revision-fixture".into(),
        payload_fingerprint: "epic-planning-draft-fixture:proposal-token-fixture:application-user"
            .into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
    }];
    query.initiation_results = vec![InitiationResultDto {
        result_id: "init-result-fixture".into(),
        command_id: "init-command-fixture".into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
    }];
    query.initiation_events = vec![InitiationEventDto {
        event_id: "init-event-fixture".into(),
        command_id: "init-command-fixture".into(),
        result_id: "init-result-fixture".into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
    }];
    query.initiation_provenance = vec![InitiationProvenanceDto {
        provenance_id: "init-provenance-fixture".into(),
        command_id: "init-command-fixture".into(),
        result_id: "init-result-fixture".into(),
        event_id: "init-event-fixture".into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
    }];
    query.material_snapshots = vec![MaterialSnapshotDto {
        material_snapshot_id: "snapshot-fixture".into(),
        epic_planning_draft_id: "epic-planning-draft-fixture".into(),
        proposal_revision_id: "proposal-revision-fixture".into(),
        version: 1,
        proposal,
        content_hash: "24a39e80f2f30ceb15c9488d9e9e48f6cf462b44ee1b69e6133342495d94dae8".into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
    }];
    query.initiated_epics = vec![InitiatedEpicDto {
        initiation_id: "initiation-fixture".into(),
        epic_planning_draft_id: "epic-planning-draft-fixture".into(),
        proposal_revision_id: "proposal-revision-fixture".into(),
        material_snapshot_id: "snapshot-fixture".into(),
        epic_id: "epic-fixture".into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
        command_id: "init-command-fixture".into(),
        result_id: "init-result-fixture".into(),
        event_id: "init-event-fixture".into(),
        provenance_id: "init-provenance-fixture".into(),
    }];
    query.initiated_sprints = vec![
        InitiatedSprintDto {
            sprint_id: "sprint-fixture".into(),
            epic_id: "epic-fixture".into(),
            ordinal: 0,
            title: "Sprint fixture".into(),
            intended_movement: "Move fixture forward.".into(),
            concern_summaries: vec!["Concern fixture.".into()],
            sprint_plan_id: "sprint-plan-fixture".into(),
            sprint_plan_revision_id: "sprint-plan-revision-fixture".into(),
        },
        InitiatedSprintDto {
            sprint_id: "sprint-fixture-2".into(),
            epic_id: "epic-fixture".into(),
            ordinal: 1,
            title: "Second Sprint fixture".into(),
            intended_movement: "Move second fixture forward.".into(),
            concern_summaries: vec!["Second concern fixture.".into()],
            sprint_plan_id: "sprint-plan-fixture-2".into(),
            sprint_plan_revision_id: "sprint-plan-revision-fixture-2".into(),
        },
    ];
    query
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GoldenBoundaryDto {
    code: &'static str,
    is_error: bool,
    state_authority: &'static str,
}
impl From<SaveProposalError> for GoldenBoundaryDto {
    fn from(error: SaveProposalError) -> Self {
        let code = match error {
            SaveProposalError::RevisionConflict => "revision_conflict",
            SaveProposalError::IdempotencyConflict => "idempotency_conflict",
            SaveProposalError::Forbidden => "forbidden",
            _ => unreachable!("fixture error"),
        };
        Self {
            code,
            is_error: true,
            state_authority: "none",
        }
    }
}
