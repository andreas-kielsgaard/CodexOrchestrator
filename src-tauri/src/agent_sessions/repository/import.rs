//! Atomic historical materialization. No runtime notifications or lifecycle dispatch.
use super::{mapping::*, SqliteAgentSessionRepository};
use crate::agent_sessions::{domain::*, imports::*, ports::*};
use rusqlite::{params, OptionalExtension};
use serde_json::json;

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_session_imports (
 request_id TEXT PRIMARY KEY,
 receipt_json TEXT NOT NULL CHECK(json_valid(receipt_json)),
 session_id TEXT UNIQUE REFERENCES agent_sessions(id),
 completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0,1))
);
CREATE TABLE IF NOT EXISTS agent_session_imported_turns (
 invocation_id TEXT PRIMARY KEY REFERENCES agent_session_invocations(id) ON DELETE CASCADE,
 session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
 source_turn_id TEXT NOT NULL,
 ordinal INTEGER NOT NULL,
 source_started_at INTEGER,
 source_completed_at INTEGER,
 UNIQUE(session_id,ordinal)
);
"#;

impl AgentSessionImportStore for SqliteAgentSessionRepository {
    fn receipt(&self, id: &str) -> Result<Option<ImportReceipt>, String> {
        self.read("load import receipt", |tx| {
            let raw: Option<String> = tx
                .query_row(
                    "SELECT receipt_json FROM agent_session_imports WHERE request_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(sql_unavailable("load import receipt"))?;
            raw.map(|s| from_json(&s, "import receipt")).transpose()
        })
        .map_err(|e| e.to_string())
    }
    fn claim(&self, receipt: &ImportReceipt) -> Result<(), String> {
        self.write("claim conversation import", |tx| {
            tx.execute(
                "INSERT INTO agent_session_imports(request_id,receipt_json) VALUES (?1,?2)",
                params![receipt.request_id, to_json(receipt)?],
            )
            .map_err(sql_write("claim import"))?;
            Ok(())
        })
        .map_err(|e| e.to_string())
    }
    fn record_fork(&self, receipt: &ImportReceipt) -> Result<(), String> {
        self.write("record native fork receipt", |tx| {
            tx.execute("UPDATE agent_session_imports SET receipt_json=?2 WHERE request_id=?1 AND completed=0",
                params![receipt.request_id, to_json(receipt)?]).map_err(sql_write("record fork"))?;
            Ok(())
        }).map_err(|e| e.to_string())
    }
    fn materialize(&self, receipt: &ImportReceipt) -> Result<(), String> {
        let fork = receipt
            .fork
            .as_ref()
            .ok_or("No acknowledged fork to import")?;
        self.write("materialize conversation import", |tx| {
            let done: bool = tx.query_row("SELECT completed FROM agent_session_imports WHERE request_id=?1",
                [&receipt.request_id], |r| r.get(0)).map_err(sql_unavailable("read import completion"))?;
            if done { return Ok(()); }
            let session = &receipt.session;
            validate_session(session).map_err(contract_error)?;
            insert_session(tx, session)?;
            crate::native_profiles::session_binding::insert_import_binding(tx, session.id.as_str(), &receipt.home, &timestamp(session.created_at))
                .map_err(|e| RepositoryError::new(RepositoryErrorKind::Conflict, e))?;
            for (ordinal, turn) in fork.turns.iter().enumerate() {
                let id = AgentInvocationId::new(format!("{}-import-{ordinal:08}", session.id.as_str())).map_err(contract_error)?;
                let input = turn.items.iter().find(|i| i.kind == "user").map(|i| i.text.as_str()).unwrap_or("[No user message in imported turn]");
                let invocation = AgentInvocation {
                    id: id.clone(), session_id: session.id.clone(), submitted_text: input.into(),
                    input_provenance: AgentInvocationInputProvenance::User, status: turn.status,
                    requested_options: Default::default(), effective_options: None, started_at: None,
                    completed_at: None, exit_code: None, signal: None, runtime_error: None,
                    diagnostics: vec![], created_at: session.created_at, updated_at: session.created_at,
                };
                // Imported records have no Orchid execution timestamps. Source times live in provenance.
                insert_invocation(tx, &invocation)?;
                tx.execute("INSERT INTO agent_session_imported_turns VALUES (?1,?2,?3,?4,?5,?6)",
                    params![id.as_str(),session.id.as_str(),turn.id,ordinal as i64,turn.started_at,turn.completed_at])
                    .map_err(sql_write("import turn provenance"))?;
                let metadata = json!({"kind":"codex_history_import","sourceThreadId":receipt.source_thread_id,
                    "sourceTurnId":turn.id,"ordinal":ordinal,"sourceStartedAt":turn.started_at,
                    "sourceCompletedAt":turn.completed_at});
                insert_event(tx, &AgentRuntimeEvent {
                    id: AgentRuntimeEventId::new(format!("{}-metadata", id.as_str())).map_err(contract_error)?,
                    invocation_id:id.clone(),sequence:0,source:AgentRuntimeEventSource::Runtime,
                    raw_payload:metadata.clone(), normalized:Some(NormalizedRuntimeEvent {
                        kind:NormalizedRuntimeEventKind::Unknown,text:None,external_context_id:None,
                        usage:None,details:Some(metadata),tool_activity:None }),recorded_at:session.created_at,
                })?;
                for (index, item) in turn.items.iter().enumerate() {
                    let mut normalized = item.normalized.clone().unwrap_or(NormalizedRuntimeEvent {
                        kind:NormalizedRuntimeEventKind::Unknown,text:Some(item.text.clone()),external_context_id:None,
                        usage:None,details:None,tool_activity:None });
                    let mut details = normalized.details.take().unwrap_or(json!({}));
                    if !details.is_object() { details = json!({"sourceDetails":details}); }
                    details["importedContent"] = json!({"kind":item.kind,"text":item.text});
                    normalized.details = Some(details);
                    insert_event(tx, &AgentRuntimeEvent {
                        id:AgentRuntimeEventId::new(format!("{}-{index}", id.as_str())).map_err(contract_error)?,
                        invocation_id:id.clone(), sequence:index as u64+1, source:AgentRuntimeEventSource::Runtime,
                        raw_payload:item.raw.clone(),normalized:Some(normalized),recorded_at:session.created_at,
                    })?;
                }
            }
            let mut complete = receipt.clone(); complete.completed = true; complete.fork = None;
            tx.execute("UPDATE agent_session_imports SET session_id=?2,completed=1,receipt_json=?3 WHERE request_id=?1",
                params![receipt.request_id,session.id.as_str(),to_json(&complete)?]).map_err(sql_write("complete import"))?;
            Ok(())
        }).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    fn prepare(path: &std::path::Path) -> (SqliteAgentSessionRepository, ImportReceipt) {
        let conn = crate::storage::open_active_database(path).unwrap();
        conn.execute("INSERT INTO native_codex_profiles VALUES ('profile','home','identity','registered_existing','active','now','now','now')", []).unwrap();
        let repo = SqliteAgentSessionRepository::new(conn).unwrap();
        let now = Utc::now();
        let session = AgentSession {
            id: AgentSessionId::new("import-session").unwrap(),
            title: "Imported".into(),
            execution_target: None,
            availability: AgentSessionAvailability::Available,
            workspace_origin: Some("allocated".into()),
            working_directory: Some("workspace".into()),
            requested_options: Default::default(),
            session_profile: None,
            harness_version: None,
            assigned_identity: None,
            created_at: now,
            updated_at: now,
            runtime_binding: AgentRuntimeBinding {
                external_context_id: Some(ExternalRuntimeContextId::new("fork").unwrap()),
                runtime_version: None,
            },
        };
        let fork = ImportedThread {
            id: "fork".into(),
            title: "Imported".into(),
            cwd: None,
            version: None,
            turns: vec![ImportedTurn {
                id: "source-turn".into(),
                status: AgentInvocationStatus::Completed,
                started_at: Some(123),
                completed_at: Some(124),
                items: vec![ImportedItem {
                    kind: "user".into(),
                    text: "Remember context".into(),
                    raw: json!({"type":"userMessage"}),
                    normalized: None,
                }],
            }],
        };
        let receipt = ImportReceipt {
            request_id: "request".into(),
            source_thread_id: "source".into(),
            last_turn_id: "source-turn".into(),
            home: ImportHome {
                profile_id: "profile".into(),
                filesystem_identity: "identity".into(),
                path: "home".into(),
            },
            session,
            fork: Some(fork),
            completed: false,
        };
        (repo, receipt)
    }
    #[test]
    fn import_is_atomic_retryable_and_survives_reopen_without_launch_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db.sqlite");
        let (repo, receipt) = prepare(&path);
        repo.claim(&receipt).unwrap();
        repo.record_fork(&receipt).unwrap();
        repo.materialize(&receipt).unwrap();
        repo.materialize(&receipt).unwrap();
        drop(repo);
        let repo = SqliteAgentSessionRepository::open(&path).unwrap();
        let history = repo
            .load_session_history(&receipt.session.id)
            .unwrap()
            .unwrap();
        assert_eq!(history.invocations.len(), 1);
        assert_eq!(
            history.invocations[0].invocation.submitted_text,
            "Remember context"
        );
        assert_eq!(history.invocations[0].events.len(), 2);
        assert!(history.invocations[0].launch_accepted_at.is_none());
        assert!(
            crate::agent_sessions::application::project_invocation_observation(
                &history.invocations[0]
            )
            .process_terminal
            .is_none()
        );
        assert!(repo.receipt("request").unwrap().unwrap().completed);
        repo.read("verify import effects", |tx| {
            for table in [
                "agent_session_native_profile_launch_provenance",
                "agent_session_invocation_launch_acceptances",
                "agent_session_addresses",
                "session_event_deliveries",
            ] {
                let count: i64 = tx
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                    .unwrap();
                assert_eq!(count, 0, "{table}");
            }
            let count: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM agent_session_native_profile_bindings",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
    }
    #[test]
    fn binding_failure_rolls_back_session_and_history_but_retains_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let (repo, mut receipt) = prepare(&dir.path().join("db.sqlite"));
        receipt.home.filesystem_identity = "changed".into();
        repo.claim(&receipt).unwrap();
        assert!(repo.materialize(&receipt).is_err());
        assert!(repo.get_session(&receipt.session.id).unwrap().is_none());
        assert!(!repo.receipt("request").unwrap().unwrap().completed);
    }
}
