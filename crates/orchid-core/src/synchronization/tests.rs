use super::*;
use crate::{
    agent_sessions::{
        domain::*, ports::*, queries::AgentSessionQueries, repository::SqliteAgentSessionRepository,
    },
    persistence::ActiveDatabase,
};
use std::sync::Arc;
fn database() -> Arc<ActiveDatabase> {
    Arc::new(
        ActiveDatabase::from_connection(
            rusqlite::Connection::open_in_memory().unwrap(),
            crate::schema::initialize,
        )
        .unwrap(),
    )
}
fn session(repository: &SqliteAgentSessionRepository) -> AgentSession {
    let now = chrono::Utc::now();
    repository
        .create_session(AgentSession {
            id: AgentSessionId::new("conversation").unwrap(),
            title: "Retained conversation".into(),
            availability: AgentSessionAvailability::Available,
            runtime_binding: AgentRuntimeBinding {
                external_context_id: None,
                runtime_version: None,
            },
            working_directory: None,
            workspace_origin: None,
            execution_target: None,
            requested_options: Default::default(),
            session_profile: None,
            harness_version: None,
            assigned_identity: None,
            created_at: now,
            updated_at: now,
        })
        .unwrap()
}
#[test]
fn committed_history_replicates_without_becoming_executable_and_replays_idempotently() {
    let db = database();
    let repository = SqliteAgentSessionRepository::from_database(db.clone());
    let local = repository::ReplicaStore::new(db.clone());
    let created = session(&repository);
    let now = chrono::Utc::now();
    let invocation = repository
        .create_pending_invocation(AgentInvocation {
            id: AgentInvocationId::new("turn").unwrap(),
            session_id: created.id.clone(),
            submitted_text: "Keep this conversation".into(),
            attachments: vec![RuntimeImageAttachment {
                id: "a".repeat(64),
                name: "note.txt".into(),
                media_type: "text/plain".into(),
                data_base64: "cGVyc2lzdGVudA==".into(),
            }],
            input_provenance: AgentInvocationInputProvenance::User,
            status: AgentInvocationStatus::Pending,
            requested_options: Default::default(),
            effective_options: None,
            started_at: None,
            completed_at: None,
            exit_code: None,
            signal: None,
            runtime_error: None,
            diagnostics: vec![],
            created_at: now,
            updated_at: now,
        })
        .unwrap();
    repository
        .mark_invocation_running(&invocation.id, now, Default::default(), now)
        .unwrap();
    repository
        .append_event(AgentRuntimeEvent {
            id: AgentRuntimeEventId::new("event").unwrap(),
            invocation_id: invocation.id,
            sequence: 0,
            source: AgentRuntimeEventSource::Runtime,
            raw_payload: serde_json::json!({"largeToolDetails":"retained"}),
            normalized: None,
            recorded_at: now,
        })
        .unwrap();
    let pending = local.pending_sessions(0).unwrap();
    assert_eq!(pending.len(), 1);
    let change = repository
        .export_session(&created.id, local.device_id().unwrap(), pending[0].1)
        .unwrap();
    let replica_db = database();
    let replica = repository::ReplicaStore::new(replica_db.clone());
    assert!(replica.accept(&change.device_id, &change).unwrap());
    assert!(!replica.accept(&change.device_id, &change).unwrap());
    assert!(replica.accept("another-device", &change).is_err());
    let execution_repository = Arc::new(SqliteAgentSessionRepository::from_database(replica_db));
    assert!(execution_repository
        .list_session_summaries(Default::default())
        .unwrap()
        .is_empty());
    assert!(replica.pending_sessions(0).unwrap().is_empty());
    let queries = AgentSessionQueries::new(execution_repository).with_replicas(replica);
    assert_eq!(
        queries.load_session(&created.id).unwrap(),
        change.session.history
    );
    assert_eq!(
        queries
            .event_details(&created.id, "event")
            .unwrap()
            .raw_payload["largeToolDetails"],
        "retained"
    );
    assert_eq!(queries.list_sessions(Default::default()).unwrap().len(), 1);
    assert_eq!(local.cursor("offline-server").unwrap(), (0, 0));
}
#[test]
fn journal_is_rolled_back_with_failed_local_mutation() {
    let db = database();
    let repository = SqliteAgentSessionRepository::from_database(db.clone());
    session(&repository);
    let store = repository::ReplicaStore::new(db.clone());
    let before = store.pending_sessions(0).unwrap();
    let result = db.write("rollback probe", |tx| -> Result<(), String> {
        tx.execute(
            "UPDATE agent_sessions SET title='uncommitted' WHERE id='conversation'",
            [],
        )
        .map_err(|e| e.to_string())?;
        Err("reject mutation".into())
    });
    assert!(result.is_err());
    assert_eq!(store.pending_sessions(0).unwrap(), before);
    assert_eq!(
        repository
            .get_session(&AgentSessionId::new("conversation").unwrap())
            .unwrap()
            .unwrap()
            .title,
        "Retained conversation"
    );
}

#[test]
fn former_owner_does_not_present_stale_local_history_after_cache_eviction() {
    struct Source(SessionChange);
    impl SessionReplicaSource for Source {
        fn fetch_session(&self, _: &str) -> Result<SessionChange, String> {
            Ok(self.0.clone())
        }
    }
    let db = database();
    let repository = Arc::new(SqliteAgentSessionRepository::from_database(db.clone()));
    let created = session(&repository);
    let store = repository::ReplicaStore::new(db.clone());
    let local = store.device_id().unwrap();
    let mut change = repository
        .export_current_session(&created.id, "new-owner".into())
        .unwrap();
    change.session.history.session.title = "Updated on the new owner".into();
    change.transfers.push(OwnershipTransfer {
        operation_id: "transfer".into(),
        source: local,
        destination: "new-owner".into(),
    });
    db.write("release fixture ownership", |db| {
        db.execute(
            "INSERT INTO orchid_session_authority VALUES(?1,'new-owner',?2,0)",
            rusqlite::params![
                created.id.as_str(),
                serde_json::to_string(&change.transfers).unwrap()
            ],
        )
    })
    .unwrap();
    store.accept("new-owner", &change).unwrap();
    store
        .set_retention("sessions", RetentionMode::Cache)
        .unwrap();
    store.evict_cached_sessions().unwrap();
    let queries = AgentSessionQueries::new(repository.clone()).with_replicas(store.clone());
    assert!(queries
        .load_session(&created.id)
        .unwrap_err()
        .to_string()
        .contains("cached summary"));
    let queries = queries.with_replica_source(Arc::new(Source(change)));
    assert_eq!(
        queries.load_session(&created.id).unwrap().session.title,
        "Updated on the new owner"
    );
    assert!(store.get(created.id.as_str()).unwrap().is_some());
    assert_eq!(
        repository.get_session(&created.id).unwrap().unwrap().title,
        "Retained conversation"
    );
}

#[test]
fn fork_copies_completed_context_without_handles_queue_or_false_launch_evidence() {
    let db = database();
    let repository = SqliteAgentSessionRepository::from_database(db);
    let created = session(&repository);
    let now = chrono::Utc::now();
    let turn = AgentInvocation {
        id: AgentInvocationId::new("finished-turn").unwrap(),
        session_id: created.id.clone(),
        submitted_text: "Discuss this change".into(),
        attachments: vec![],
        input_provenance: AgentInvocationInputProvenance::User,
        status: AgentInvocationStatus::Pending,
        requested_options: Default::default(),
        effective_options: None,
        started_at: None,
        completed_at: None,
        exit_code: None,
        signal: None,
        runtime_error: None,
        diagnostics: vec![],
        created_at: now,
        updated_at: now,
    };
    repository.create_pending_invocation(turn.clone()).unwrap();
    repository
        .mark_invocation_running(&turn.id, now, Default::default(), now)
        .unwrap();
    repository
        .append_event(AgentRuntimeEvent {
            id: AgentRuntimeEventId::new("answer").unwrap(),
            invocation_id: turn.id.clone(),
            sequence: 0,
            source: AgentRuntimeEventSource::Runtime,
            raw_payload: serde_json::json!({"answer":"Retain this"}),
            normalized: None,
            recorded_at: now,
        })
        .unwrap();
    repository
        .finish_invocation(
            &turn.id,
            InvocationCompletion {
                status: AgentInvocationTerminalStatus::Completed,
                completed_at: now,
                exit_code: Some(0),
                signal: None,
                runtime_error: None,
            },
            now,
        )
        .unwrap();
    let source = repository
        .load_session_history(&created.id)
        .unwrap()
        .unwrap();
    let destination = SqliteAgentSessionRepository::from_database(database());
    let worktree = tempfile::tempdir().unwrap();
    let target = crate::execution_targets::domain::SessionExecutionTarget {
        capability_profile_id: "profile".into(),
        capability_profile_revision: 1,
        execution: Default::default(),
        repository_id: "repository".into(),
        branch_ref: "refs/heads/main".into(),
        worktree_id: "worktree".into(),
        path: worktree.path().to_string_lossy().into_owned(),
        head: None,
    };
    let fork_id = AgentSessionId::new("fork").unwrap();
    let prefix = InitialPromptPrefix {
        source: "workflow".into(),
        version: 1,
        content: "Original conversation instructions".into(),
    };
    let fork = destination
        .fork_conversation_with_context(
            fork_id.clone(),
            &source,
            target.clone(),
            Some(prefix.clone()),
        )
        .unwrap();
    let history = destination.load_session_history(&fork.id).unwrap().unwrap();
    assert_eq!(history.invocations.len(), 1);
    assert!(history
        .session
        .runtime_binding
        .external_context_id
        .is_none());
    assert!(history.invocations[0].launch_accepted_at.is_none());
    assert!(history.invocations[0].import_provenance.is_some());
    assert!(history.invocations[0].invocation.started_at.is_none());
    assert_eq!(
        history.invocations[0].events[0].raw_payload["answer"],
        "Retain this"
    );
    assert!(destination
        .session_queue(&fork.id)
        .unwrap()
        .entries
        .is_empty());
    assert_eq!(
        destination.initial_prompt_prefix(&fork.id).unwrap(),
        Some(prefix.clone())
    );
    assert_eq!(
        destination
            .fork_conversation_with_context(fork_id.clone(), &source, target.clone(), Some(prefix))
            .unwrap()
            .id,
        fork.id
    );
    assert!(destination
        .fork_conversation_with_context(fork_id, &source, target, None)
        .is_err());
}

#[test]
fn cache_eviction_preserves_summary_and_owner_without_touching_local_work() {
    let db = database();
    let repository = SqliteAgentSessionRepository::from_database(db.clone());
    let created = session(&repository);
    let local = repository::ReplicaStore::new(db);
    let change = repository
        .export_current_session(&created.id, local.device_id().unwrap())
        .unwrap();
    let replica = repository::ReplicaStore::new(database());
    replica
        .set_retention("sessions", RetentionMode::Cache)
        .unwrap();
    replica.accept_for_retention(&change).unwrap();
    assert!(replica.get(created.id.as_str()).unwrap().is_none());
    assert_eq!(replica.summaries().unwrap().len(), 1);
    assert_eq!(
        replica.owner(created.id.as_str()).unwrap(),
        Some(change.device_id.clone())
    );
    replica.accept(&change.device_id, &change).unwrap();
    assert!(replica.get(created.id.as_str()).unwrap().is_some());
    replica.evict_cached_sessions().unwrap();
    assert_eq!(replica.completeness().unwrap(), (1, 0));
    assert!(repository.get_session(&created.id).unwrap().is_some());
    replica
        .set_retention("sessions", RetentionMode::Retained)
        .unwrap();
    assert!(replica.evict(created.id.as_str()).is_err());
}

#[test]
fn explicit_handoff_moves_authority_and_rejects_late_source_snapshots() {
    let source_db = database();
    let source = SqliteAgentSessionRepository::from_database(source_db.clone());
    let created = session(&source);
    let now = chrono::Utc::now();
    let pending:AgentInvocation=serde_json::from_value(serde_json::json!({"id":"queued","sessionId":created.id,"submittedText":"original queued input","attachments":[],"inputProvenance":"application","status":"pending","requestedOptions":{},"diagnostics":[],"createdAt":now,"updatedAt":now})).unwrap();
    let preparation:crate::agent_sessions::preparation::SessionPreparation=serde_json::from_value(serde_json::json!({"invocationId":"queued","sessionId":created.id,"pinnedDelivery":true,"phase":"accepted","steps":[],"canRetry":false,"sourceBinding":{"externalContextId":null,"runtimeVersion":null},"deliveryStarted":false})).unwrap();
    source
        .accept_queued_preparation(None, pending, preparation, None, now)
        .unwrap();
    let source_store = repository::ReplicaStore::new(source_db.clone());
    let source_id = source_store.device_id().unwrap();
    let old = source
        .export_current_session(&created.id, source_id.clone())
        .unwrap();
    let server = repository::ReplicaStore::new(database());
    server.accept(&source_id, &old).unwrap();
    let destination_db = database();
    let destination = SqliteAgentSessionRepository::from_database(destination_db.clone());
    let destination_id = repository::ReplicaStore::new(destination_db)
        .device_id()
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let target = crate::execution_targets::domain::SessionExecutionTarget {
        capability_profile_id: "profile".into(),
        capability_profile_revision: 1,
        execution: crate::execution_targets::domain::ExecutionBinding {
            device_id: destination_id.clone(),
            connection: crate::execution_targets::domain::ExecutionConnection::Device,
            ..Default::default()
        },
        repository_id: "repository".into(),
        branch_ref: "refs/heads/main".into(),
        worktree_id: "worktree".into(),
        path: root.path().to_string_lossy().into_owned(),
        head: None,
    };
    let transfers = vec![OwnershipTransfer {
        operation_id: "move-1".into(),
        source: source_id.clone(),
        destination: destination_id.clone(),
    }];
    source_db.write("release source for transfer",|tx|->Result<(),String>{
        tx.execute("UPDATE agent_sessions SET execution_target_json=?1,working_directory=?2 WHERE id='conversation'",rusqlite::params![serde_json::to_string(&target).unwrap(),target.path]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO orchid_session_authority VALUES('conversation',?1,?2,1)",rusqlite::params![destination_id,serde_json::to_string(&transfers).unwrap()]).map_err(|e|e.to_string())?;Ok(())
    }).unwrap();
    assert!(!source.owns_execution(&created.id).unwrap());
    let handoff = source
        .export_current_session(&created.id, source_id.clone())
        .unwrap();
    destination
        .accept_handoff(&destination_id, &handoff)
        .unwrap();
    destination
        .accept_handoff(&destination_id, &handoff)
        .unwrap();
    assert!(destination.owns_execution(&created.id).unwrap());
    assert_eq!(
        destination
            .session_queue(&created.id)
            .unwrap()
            .entries
            .len(),
        1
    );
    assert!(
        !destination
            .preparation(&AgentInvocationId::new("queued").unwrap())
            .unwrap()
            .unwrap()
            .pinned_delivery
    );
    assert_eq!(
        destination
            .get_session(&created.id)
            .unwrap()
            .unwrap()
            .execution_target
            .unwrap()
            .execution
            .device_id,
        "local"
    );
    let changed = destination
        .export_current_session(&created.id, destination_id.clone())
        .unwrap();
    assert!(server.accept(&destination_id, &changed).unwrap());
    assert!(!server.accept(&source_id, &old).unwrap());
    assert!(source_store.accept(&destination_id, &changed).unwrap());
    assert_eq!(
        source.execution_owner(&created.id).unwrap(),
        Some(destination_id.clone())
    );
    assert!(source.pending_handoffs().unwrap().is_empty());
    // Returning to a former owner must bring the current queued payload, not its stale local row.
    let destination_db = destination.database();
    let mut returning = changed.clone();
    returning.transfers.push(OwnershipTransfer {
        operation_id: "move-back".into(),
        source: destination_id.clone(),
        destination: source_id.clone(),
    });
    returning
        .session
        .history
        .session
        .execution_target
        .as_mut()
        .unwrap()
        .execution
        .device_id = source_id.clone();
    returning.session.history.invocations[0]
        .invocation
        .submitted_text = "edited on the other host".into();
    destination_db.write("release return handoff",|db|db.execute("UPDATE orchid_session_authority SET owner=?1,transfers_json=?2 WHERE session_id=?3",rusqlite::params![source_id,serde_json::to_string(&returning.transfers).unwrap(),created.id.as_str()])).unwrap();
    source.accept_handoff(&source_id, &returning).unwrap();
    source.accept_handoff(&source_id, &returning).unwrap();
    assert!(source.owns_execution(&created.id).unwrap());
    assert!(!destination.owns_execution(&created.id).unwrap());
    assert_eq!(
        source
            .get_invocation(&AgentInvocationId::new("queued").unwrap())
            .unwrap()
            .unwrap()
            .submitted_text,
        "edited on the other host"
    );
}
