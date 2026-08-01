use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// A fresh baseline; the incompatible active-v2 file is intentionally never opened or migrated.
pub(crate) const ACTIVE_DATABASE_FILE_NAME: &str = "codex-orchestrator-active-v3.sqlite";
const ACTIVE_SCHEMA_VERSION: i64 = 11;

pub(crate) fn active_database_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(ACTIVE_DATABASE_FILE_NAME)
}

pub(crate) fn open_active_database(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path)
        .map_err(|error| format!("Unable to open active SQLite database: {error}"))?;
    configure_sqlite_connection(&connection)
        .map_err(|error| format!("Unable to configure active SQLite database: {error}"))?;
    initialize_active_database(&connection)?;
    Ok(connection)
}

pub(crate) fn initialize_active_database(connection: &Connection) -> Result<(), String> {
    let current_version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(|error| format!("Unable to read active schema version: {error}"))?;
    if current_version == ACTIVE_SCHEMA_VERSION {
        return Ok(());
    }
    if (1..=10).contains(&current_version) {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| format!("Unable to begin active schema migration: {error}"))?;
        if current_version == 1 {
            transaction
                .execute_batch(crate::orchestration::repository::ORCHESTRATION_INITIATION_SCHEMA)
                .map_err(|error| {
                    format!("Unable to migrate orchestration initiation schema: {error}")
                })?;
        }
        if current_version < 3 {
            transaction
                .execute_batch(crate::orchestration::bootstrap_transition::POST_CONFIRMATION_SCHEMA)
                .map_err(|error| format!("Unable to migrate post-confirmation schema: {error}"))?;
        }
        if current_version <= 5 {
            transaction
                .execute_batch(
                    crate::orchestration::bootstrap_transition::POST_CONFIRMATION_ATTEMPT_SCHEMA,
                )
                .map_err(|error| format!("Unable to migrate bootstrap attempt schema: {error}"))?;
            if current_version < 5 {
                transaction
                    .execute_batch(
                        crate::orchestration::repository::PLAN_BUILDER_CONTEXT_DELIVERY_SCHEMA,
                    )
                    .map_err(|error| {
                        format!("Unable to migrate Plan Builder context schema: {error}")
                    })?;
            } else {
                transaction
                    .execute_batch(
                        crate::orchestration::repository::PLAN_BUILDER_CONTEXT_RECONCILIATION_SCHEMA,
                    )
                    .map_err(|error| {
                        format!("Unable to migrate Plan Builder context reconciliation schema: {error}")
                    })?;
            }
        }
        transaction
            .execute_batch(
                crate::agent_sessions::repository::AGENT_SESSION_LAUNCH_ACCEPTANCE_SCHEMA,
            )
            .map_err(|error| {
                format!("Unable to migrate Agent Session launch acceptance schema: {error}")
            })?;
        transaction
            .execute_batch(crate::orchestration::repository::FILE_REVIEW_FACTS_SCHEMA)
            .map_err(|error| format!("Unable to migrate File Review facts schema: {error}"))?;
        if current_version == 8 {
            transaction
                .execute_batch(
                    crate::orchestration::repository::FILE_REVIEW_FACTS_IDEMPOTENCY_SCHEMA,
                )
                .map_err(|error| {
                    format!("Unable to migrate File Review idempotency schema: {error}")
                })?;
        }
        if current_version <= 9 {
            transaction
                .execute_batch(
                    crate::orchestration::repository::FILE_REVIEW_GIT_CAPTURE_AUTHORIZATION_SCHEMA,
                )
                .map_err(|error| {
                    format!(
                        "Unable to migrate File Review Git-capture authorization schema: {error}"
                    )
                })?;
        }
        transaction
            .execute_batch(crate::orchestration::repository::INITIATED_SPRINT_GIT_AUTHORITY_SCHEMA)
            .map_err(|error| {
                format!("Unable to migrate initiated Sprint Git authority schema: {error}")
            })?;
        transaction
            .pragma_update(None, "user_version", ACTIVE_SCHEMA_VERSION)
            .map_err(|error| format!("Unable to record active schema version: {error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("Unable to commit active schema migration: {error}"))?;
        return Ok(());
    }
    if current_version != 0 {
        return Err(format!(
            "Unsupported active database schema version {current_version}; expected {ACTIVE_SCHEMA_VERSION}"
        ));
    }
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("Unable to begin active schema initialization: {error}"))?;
    transaction
        .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
        .map_err(|error| format!("Unable to initialize Agent Session schema: {error}"))?;
    transaction
        .execute_batch(crate::orchestration::repository::ORCHESTRATION_SCHEMA)
        .map_err(|error| format!("Unable to initialize orchestration schema: {error}"))?;
    transaction
        .execute_batch(crate::orchestration::repository::ORCHESTRATION_INITIATION_SCHEMA)
        .map_err(|error| {
            format!("Unable to initialize orchestration initiation schema: {error}")
        })?;
    transaction
        .execute_batch(crate::orchestration::bootstrap_transition::POST_CONFIRMATION_SCHEMA)
        .map_err(|error| format!("Unable to initialize post-confirmation schema: {error}"))?;
    transaction
        .execute_batch(crate::orchestration::bootstrap_transition::POST_CONFIRMATION_ATTEMPT_SCHEMA)
        .map_err(|error| format!("Unable to initialize bootstrap attempt schema: {error}"))?;
    transaction
        .execute_batch(crate::orchestration::repository::PLAN_BUILDER_CONTEXT_DELIVERY_SCHEMA)
        .map_err(|error| format!("Unable to initialize Plan Builder context schema: {error}"))?;
    transaction
        .execute_batch(crate::orchestration::repository::FILE_REVIEW_FACTS_SCHEMA)
        .map_err(|error| format!("Unable to initialize File Review facts schema: {error}"))?;
    transaction
        .execute_batch(crate::orchestration::repository::INITIATED_SPRINT_GIT_AUTHORITY_SCHEMA)
        .map_err(|error| {
            format!("Unable to initialize initiated Sprint Git authority schema: {error}")
        })?;
    transaction
        .pragma_update(None, "user_version", ACTIVE_SCHEMA_VERSION)
        .map_err(|error| format!("Unable to record active schema version: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("Unable to commit active schema initialization: {error}"))?;
    Ok(())
}
use std::time::Duration;

const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Applies the app-wide policy to every SQLite connection before it is used. WAL permits readers
/// while a writer commits, the bounded busy timeout handles brief contention deliberately, and FULL
/// synchronous mode favors durable commits over write throughput.
pub(crate) fn configure_sqlite_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_exists(connection: &Connection, name: &str) -> bool {
        connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                [name],
                |_| Ok(()),
            )
            .is_ok()
    }

    fn seed_file_review_predecessor(
        connection: &Connection,
        version: i64,
        fingerprint: bool,
        first_kind: &str,
    ) {
        let document_fingerprint = if fingerprint {
            ", payload_fingerprint TEXT NOT NULL"
        } else {
            ""
        };
        connection.execute_batch(&format!("CREATE TABLE epic_initiations (epic_id TEXT PRIMARY KEY); CREATE TABLE initiated_sprints (id TEXT PRIMARY KEY); CREATE TABLE epic_initiation_provenance (id TEXT PRIMARY KEY); INSERT INTO epic_initiations VALUES ('epic'); INSERT INTO initiated_sprints VALUES ('sprint'); INSERT INTO epic_initiation_provenance VALUES ('provenance'); CREATE TABLE file_review_documents (document_ref_id TEXT PRIMARY KEY, epic_id TEXT NOT NULL, sprint_id TEXT NOT NULL, provenance_id TEXT NOT NULL, opaque_reference TEXT NOT NULL UNIQUE, title TEXT NOT NULL, summary TEXT, idempotency_key TEXT NOT NULL UNIQUE{document_fingerprint}, recorded_at TEXT NOT NULL, FOREIGN KEY(epic_id) REFERENCES epic_initiations(epic_id), FOREIGN KEY(sprint_id) REFERENCES initiated_sprints(id), FOREIGN KEY(provenance_id) REFERENCES epic_initiation_provenance(id)); CREATE TABLE file_review_changed_files (document_ref_id TEXT NOT NULL, changed_file_reference_id TEXT NOT NULL, display_name TEXT NOT NULL, change_kind TEXT NOT NULL CHECK(change_kind IN ('added','modified','deleted','renamed')), ordinal INTEGER NOT NULL CHECK(ordinal >= 0), PRIMARY KEY(document_ref_id,changed_file_reference_id), UNIQUE(document_ref_id,ordinal), FOREIGN KEY(document_ref_id) REFERENCES file_review_documents(document_ref_id)); CREATE TABLE stored_file_review_artifacts (artifact_id TEXT PRIMARY KEY, document_ref_id TEXT NOT NULL UNIQUE, contract_version TEXT NOT NULL CHECK(contract_version='stored-file-review-artifact/v1'), payload BLOB NOT NULL, payload_bytes INTEGER NOT NULL, provenance_id TEXT NOT NULL, FOREIGN KEY(document_ref_id) REFERENCES file_review_documents(document_ref_id), FOREIGN KEY(provenance_id) REFERENCES epic_initiation_provenance(id)); INSERT INTO file_review_documents (document_ref_id,epic_id,sprint_id,provenance_id,opaque_reference,title,idempotency_key{fingerprint_column},recorded_at) VALUES ('doc','epic','sprint','provenance','opaque','Changed files','key'{fingerprint_value},'t'); INSERT INTO file_review_changed_files VALUES ('doc','file-2','src/b.ts','modified',1),('doc','file-1','src/a.ts','{first_kind}',0); INSERT INTO stored_file_review_artifacts VALUES ('artifact','doc','stored-file-review-artifact/v1',x'0102',2,'provenance'); PRAGMA user_version={version};", fingerprint_column = if fingerprint { ",payload_fingerprint" } else { "" }, fingerprint_value = if fingerprint { ",''" } else { "" })).expect("seed predecessor");
    }

    #[test]
    fn applies_explicit_connection_policy() {
        let connection = Connection::open_in_memory().expect("memory database");

        configure_sqlite_connection(&connection).expect("configure connection");

        assert_eq!(pragma_i64(&connection, "foreign_keys"), 1);
        assert_eq!(pragma_i64(&connection, "busy_timeout"), 5_000);
        assert_eq!(pragma_i64(&connection, "synchronous"), 2);
    }

    #[test]
    fn selects_wal_for_file_backed_connections() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let connection = Connection::open(directory.path().join("app.sqlite")).expect("database");

        configure_sqlite_connection(&connection).expect("configure connection");

        let journal_mode = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
            .expect("journal mode");
        assert_eq!(journal_mode, "wal");
    }

    #[test]
    fn active_database_contains_the_unified_current_schema() {
        let connection = Connection::open_in_memory().expect("memory database");
        configure_sqlite_connection(&connection).expect("configure connection");
        initialize_active_database(&connection).expect("initialize active database");

        let tables = connection
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .expect("prepare table query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query tables")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect tables");
        assert_eq!(
            tables,
            vec![
                "agent_session_invocation_diagnostics",
                "agent_session_invocation_launch_acceptances",
                "agent_session_invocations",
                "agent_session_runtime_events",
                "agent_sessions",
                "capability_profiles",
                "effect_provenance",
                "epic_bootstrap_attempt_completion_commands",
                "epic_bootstrap_attempt_completion_facts",
                "epic_bootstrap_attempt_completion_results",
                "epic_bootstrap_attempts",
                "epic_bootstrap_completion_commands",
                "epic_bootstrap_completion_facts",
                "epic_bootstrap_completion_results",
                "epic_bootstrap_transitions",
                "epic_initiation_commands",
                "epic_initiation_events",
                "epic_initiation_material_snapshots",
                "epic_initiation_provenance",
                "epic_initiation_results",
                "epic_initiations",
                "epic_planning_drafts",
                "file_review_changed_files",
                "file_review_documents",
                "file_review_git_capture_authorizations",
                "initiated_planning_drafts",
                "initiated_sprint_git_authorities",
                "initiated_sprints",
                "plan_builder_context_deliveries",
                "planning_draft_agent_session_associations",
                "planning_draft_lifecycle_events",
                "planning_draft_profile_assignments",
                "proposal_command_results",
                "proposal_commands",
                "proposal_events",
                "proposal_revisions",
                "stored_file_review_artifacts",
            ]
        );
        assert_eq!(
            pragma_i64(&connection, "user_version"),
            ACTIVE_SCHEMA_VERSION
        );
        let invocation_columns = connection
            .prepare("PRAGMA table_info(agent_session_invocations)")
            .expect("prepare invocation schema")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query invocation schema")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect invocation schema");
        assert!(invocation_columns.contains(&"input_provenance".to_string()));
        let context_columns = connection
            .prepare("PRAGMA table_info(plan_builder_context_deliveries)")
            .expect("prepare context schema")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query context schema")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect context schema");
        assert!(context_columns.contains(&"target_invocation_id".to_string()));
        let file_review_columns = connection
            .prepare("PRAGMA table_info(file_review_documents)")
            .expect("prepare File Review schema")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query File Review schema")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect File Review schema");
        assert!(file_review_columns.contains(&"payload_fingerprint".to_string()));
        let membership_columns = connection
            .prepare("PRAGMA table_info(file_review_changed_files)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(membership_columns.contains(&"previous_display_name".to_string()));
        let authorization_columns = connection
            .prepare("PRAGMA table_info(file_review_git_capture_authorizations)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        for column in [
            "capture_authorization_id",
            "idempotency_key",
            "repository_root",
            "worktree_root",
            "baseline_object_id",
            "current_object_id",
            "recorded_at",
        ] {
            assert!(authorization_columns.contains(&column.to_string()));
        }
        assert_eq!(connection.query_row("SELECT count(*) FROM pragma_index_list('file_review_git_capture_authorizations') WHERE [unique] = 1", [], |row| row.get::<_, i64>(0)).unwrap(), 2);
        assert_eq!(connection.query_row("SELECT count(*) FROM pragma_foreign_key_list('file_review_git_capture_authorizations')", [], |row| row.get::<_, i64>(0)).unwrap(), 3);
        let git_authority_columns = connection
            .prepare("PRAGMA table_info(initiated_sprint_git_authorities)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        for column in [
            "repository_common_dir",
            "runtime_instance_ref",
            "runtime_source_ref",
            "source_fingerprint",
            "baseline_object_id",
            "current_object_id",
            "recorded_at",
        ] {
            assert!(git_authority_columns.contains(&column.to_string()));
        }
    }

    #[test]
    fn migrates_genuine_v10_schema_without_losing_private_capture_authority() {
        let connection = Connection::open_in_memory().expect("database");
        configure_sqlite_connection(&connection).expect("policy");
        connection
            .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
            .expect("v10 Agent Session schema");
        connection
            .execute_batch(crate::orchestration::repository::ORCHESTRATION_SCHEMA)
            .expect("v10 orchestration schema");
        connection
            .execute_batch(crate::orchestration::repository::ORCHESTRATION_INITIATION_SCHEMA)
            .expect("v10 initiation schema");
        connection
            .execute_batch(crate::orchestration::bootstrap_transition::POST_CONFIRMATION_SCHEMA)
            .expect("v10 bootstrap schema");
        connection
            .execute_batch(
                crate::orchestration::bootstrap_transition::POST_CONFIRMATION_ATTEMPT_SCHEMA,
            )
            .expect("v10 attempt schema");
        connection
            .execute_batch(crate::orchestration::repository::PLAN_BUILDER_CONTEXT_DELIVERY_SCHEMA)
            .expect("v10 context schema");
        connection
            .execute_batch(crate::orchestration::repository::FILE_REVIEW_FACTS_SCHEMA)
            .expect("v10 File Review schema");
        connection
            .pragma_update(None, "foreign_keys", false)
            .expect("seed predecessor");
        connection.execute_batch("INSERT INTO epic_initiation_provenance (id,command_id,result_id,event_id,recorded_at) VALUES ('provenance-v10','command-v10','result-v10','event-v10','t'); INSERT INTO epic_initiations (id,command_id,result_id,event_id,provenance_id,draft_id,proposal_revision_id,material_snapshot_id,epic_id,recorded_at) VALUES ('initiation-v10','command-v10','result-v10','event-v10','provenance-v10','draft-v10','revision-v10','snapshot-v10','epic-v10','t'); INSERT INTO initiated_sprints (id,epic_id,ordinal,title,intended_movement,concern_summaries_json,sprint_plan_id,sprint_plan_revision_id) VALUES ('sprint-v10','epic-v10',0,'Sprint','Move','[]','plan-v10','plan-revision-v10'); INSERT INTO file_review_git_capture_authorizations (capture_authorization_id,idempotency_key,payload_fingerprint,epic_id,sprint_id,provenance_id,repository_id,repository_root,worktree_id,worktree_root,baseline_object_id,current_object_id,recorded_at) VALUES ('capture-v10','capture-key-v10','capture-fingerprint-v10','epic-v10','sprint-v10','provenance-v10','repository-v10','C:\\repo','worktree-v10','C:\\repo\\worktree','aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb','t'); PRAGMA user_version=10;").expect("seed v10 facts");
        connection
            .pragma_update(None, "foreign_keys", true)
            .expect("restore foreign keys");

        initialize_active_database(&connection).expect("migrate v10");

        assert!(table_exists(
            &connection,
            "initiated_sprint_git_authorities"
        ));
        assert_eq!(
            connection
                .query_row(
                    "SELECT payload_fingerprint FROM file_review_git_capture_authorizations WHERE capture_authorization_id='capture-v10'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("preserved Batch 11 authority"),
            "capture-fingerprint-v10"
        );
        assert_eq!(pragma_i64(&connection, "user_version"), 11);
        initialize_active_database(&connection).expect("reopen v11");
    }

    #[test]
    fn active_database_reopens_without_touching_legacy_database() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let legacy_path = directory.path().join("codex-orchestrator.sqlite");
        let prior_v2_path = directory.path().join("codex-orchestrator-active-v2.sqlite");
        std::fs::write(&legacy_path, b"obsolete database bytes").expect("seed legacy file");
        std::fs::write(&prior_v2_path, b"prior v2 database bytes").expect("seed v2 file");
        let active_path = active_database_path(directory.path());

        drop(open_active_database(&active_path).expect("create active database"));
        drop(open_active_database(&active_path).expect("reopen active database"));

        assert_eq!(
            std::fs::read(&legacy_path).expect("legacy file"),
            b"obsolete database bytes"
        );
        assert_eq!(
            std::fs::read(&prior_v2_path).expect("prior v2 file"),
            b"prior v2 database bytes"
        );
        assert_eq!(active_path.file_name().unwrap(), ACTIVE_DATABASE_FILE_NAME);
        assert_eq!(
            active_path.file_name().unwrap(),
            "codex-orchestrator-active-v3.sqlite"
        );
    }

    #[test]
    fn migrates_v8_file_review_rows_to_fingerprint_schema_without_losing_data() {
        let connection = Connection::open_in_memory().expect("database");
        configure_sqlite_connection(&connection).expect("policy");
        seed_file_review_predecessor(&connection, 8, false, "added");
        assert!(!table_exists(
            &connection,
            "file_review_git_capture_authorizations"
        ));
        initialize_active_database(&connection).expect("migrate v8");
        let columns = connection
            .prepare("PRAGMA table_info(file_review_documents)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(columns.contains(&"payload_fingerprint".to_string()));
        assert_eq!(connection.query_row("SELECT payload_fingerprint FROM file_review_documents WHERE document_ref_id='doc'", [], |r| r.get::<_, String>(0)).unwrap(), "");
        assert_eq!(
            connection
                .query_row(
                    "SELECT payload FROM stored_file_review_artifacts WHERE artifact_id='artifact'",
                    [],
                    |r| r.get::<_, Vec<u8>>(0)
                )
                .unwrap(),
            vec![1, 2]
        );
        assert_eq!(connection.query_row("SELECT previous_display_name FROM file_review_changed_files WHERE changed_file_reference_id='file-1'", [], |r| r.get::<_, Option<String>>(0)).unwrap(), None);
        let files = connection.prepare("SELECT changed_file_reference_id FROM file_review_changed_files WHERE document_ref_id='doc' ORDER BY ordinal").unwrap().query_map([], |r| r.get::<_, String>(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(files, vec!["file-1", "file-2"]);
        assert!(table_exists(
            &connection,
            "file_review_git_capture_authorizations"
        ));
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM pragma_foreign_key_list('file_review_changed_files')",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            pragma_i64(&connection, "user_version"),
            ACTIVE_SCHEMA_VERSION
        );
    }

    #[test]
    fn migrates_v9_file_review_rows_and_preserves_historical_rename_null() {
        let connection = Connection::open_in_memory().expect("database");
        configure_sqlite_connection(&connection).expect("policy");
        seed_file_review_predecessor(&connection, 9, true, "renamed");
        assert!(!table_exists(
            &connection,
            "file_review_git_capture_authorizations"
        ));
        initialize_active_database(&connection).expect("migrate v9");
        assert_eq!(connection.query_row("SELECT previous_display_name FROM file_review_changed_files WHERE changed_file_reference_id='file-1'", [], |r| r.get::<_, Option<String>>(0)).unwrap(), None);
        assert!(connection.execute("INSERT INTO file_review_changed_files (document_ref_id,changed_file_reference_id,display_name,change_kind,previous_display_name,ordinal) VALUES ('doc','bad','src/a.ts','modified','src/b.ts',1)", []).is_err());
        initialize_active_database(&connection).expect("reopen current schema");
    }

    #[test]
    fn migrates_v5_context_claims_to_stable_target_invocation_identity() {
        let connection = Connection::open_in_memory().expect("database");
        configure_sqlite_connection(&connection).expect("policy");
        connection
            .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
            .expect("Agent Session schema");
        connection
            .execute_batch(crate::orchestration::repository::ORCHESTRATION_SCHEMA)
            .expect("orchestration schema");
        connection
            .execute_batch(crate::orchestration::repository::ORCHESTRATION_INITIATION_SCHEMA)
            .expect("initiation schema");
        connection
            .execute_batch(crate::orchestration::bootstrap_transition::POST_CONFIRMATION_SCHEMA)
            .expect("post-confirmation schema");
        connection
            .execute_batch(
                crate::orchestration::bootstrap_transition::POST_CONFIRMATION_ATTEMPT_SCHEMA,
            )
            .expect("attempt schema");
        connection
            .execute_batch(
                "CREATE TABLE plan_builder_context_deliveries (
                   id TEXT PRIMARY KEY, initiation_id TEXT NOT NULL UNIQUE, epic_id TEXT NOT NULL,
                   agent_session_id TEXT NOT NULL, source_kind TEXT NOT NULL,
                   requested_at TEXT NOT NULL, pending_at TEXT NOT NULL,
                   delivery_claim_id TEXT, delivery_claimed_at TEXT,
                   delivered_to_invocation_id TEXT UNIQUE, delivered_at TEXT, consumed_at TEXT
                 );
                 INSERT INTO plan_builder_context_deliveries
                   (id,initiation_id,epic_id,agent_session_id,source_kind,requested_at,pending_at)
                 VALUES ('delivery','initiation','epic','session','button_initiation','t','t');",
            )
            .expect("v5 context schema");
        connection
            .pragma_update(None, "user_version", 5)
            .expect("v5");

        initialize_active_database(&connection).expect("migrate v5");

        let columns = connection
            .prepare("PRAGMA table_info(plan_builder_context_deliveries)")
            .expect("prepare columns")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query columns")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect columns");
        assert!(columns.contains(&"target_invocation_id".to_string()));
        assert_eq!(
            connection
                .query_row(
                    "SELECT initiation_id FROM plan_builder_context_deliveries WHERE id='delivery'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("preserved delivery"),
            "initiation"
        );
        assert_eq!(
            pragma_i64(&connection, "user_version"),
            ACTIVE_SCHEMA_VERSION
        );
    }

    #[test]
    fn migrates_v6_history_without_inventing_launch_acceptance() {
        let connection = Connection::open_in_memory().expect("database");
        configure_sqlite_connection(&connection).expect("policy");
        initialize_active_database(&connection).expect("initialize current schema");
        connection
            .execute(
                "INSERT INTO agent_sessions (id,title,availability,requested_options_json,created_at,updated_at) VALUES ('session-v6','Preserved session','available','{}','2026-07-16T12:00:00Z','2026-07-16T12:00:01Z')",
                [],
            )
            .expect("session history");
        connection
            .execute(
                "INSERT INTO agent_session_invocations (id,session_id,submitted_text,input_provenance,status,requested_options_json,effective_options_json,started_at,created_at,updated_at) VALUES ('invocation-v6','session-v6','preserve exact submitted text','application','running','{}','{}','2026-07-16T12:00:01Z','2026-07-16T12:00:00Z','2026-07-16T12:00:01Z')",
                [],
            )
            .expect("running invocation history");
        connection
            .execute_batch(
                "DROP TABLE agent_session_invocation_launch_acceptances; PRAGMA user_version = 6;",
            )
            .expect("represent v6 schema");

        initialize_active_database(&connection).expect("migrate v6");

        let preserved: (String, String, String) = connection
            .query_row(
                "SELECT submitted_text,input_provenance,started_at FROM agent_session_invocations WHERE id='invocation-v6'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("preserved invocation");
        assert_eq!(
            preserved,
            (
                "preserve exact submitted text".to_string(),
                "application".to_string(),
                "2026-07-16T12:00:01Z".to_string()
            )
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM agent_session_invocation_launch_acceptances",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("conservative acceptance count"),
            0
        );
        assert_eq!(
            pragma_i64(&connection, "user_version"),
            ACTIVE_SCHEMA_VERSION
        );
    }

    #[test]
    fn migrates_v1_proposal_rows_without_changing_their_identity_or_bytes() {
        let connection = Connection::open_in_memory().expect("database");
        configure_sqlite_connection(&connection).expect("policy");
        connection
            .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
            .expect("session v1");
        connection
            .execute_batch(crate::orchestration::repository::ORCHESTRATION_SCHEMA)
            .expect("orchestration v1");
        connection
            .pragma_update(None, "foreign_keys", false)
            .expect("seed legacy rows");
        connection.execute("INSERT INTO epic_planning_drafts (id,status,created_at,updated_at) VALUES ('draft','active','t','t')", []).expect("draft");
        connection.execute("INSERT INTO proposal_commands (id,idempotency_key,draft_id,capability_profile_id,agent_session_association_id,actor_id,proposal_json,payload_fingerprint,recorded_at) VALUES ('command','key','draft','profile','association','actor','{}','fingerprint','t')", []).expect("command");
        connection.execute("INSERT INTO effect_provenance (id,source_kind,recorded_at,actor_id,agent_session_association_id,capability_profile_id,causal_command_id,causal_result_id) VALUES ('provenance','managed_plan_builder','t','actor','association','profile','command','result')", []).expect("provenance");
        let bytes = r#"{"suggestedEpicName":"Epic","sprints":[{"title":"Sprint","intendedMovement":"Move","concernSummaries":[]}]}"#;
        connection.execute("INSERT INTO proposal_revisions (id,draft_id,revision_token,proposal_json,command_id,provenance_id,recorded_at) VALUES ('revision','draft','token',?1,'command','provenance','t')", [bytes]).expect("revision");
        connection
            .pragma_update(None, "user_version", 1)
            .expect("v1");
        connection
            .pragma_update(None, "foreign_keys", true)
            .expect("restore foreign keys");
        initialize_active_database(&connection).expect("migrate");
        assert_eq!(
            connection
                .query_row(
                    "SELECT proposal_json FROM proposal_revisions WHERE id='revision'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            bytes
        );
        assert_eq!(
            pragma_i64(&connection, "user_version"),
            ACTIVE_SCHEMA_VERSION
        );
        assert!(connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='epic_initiation_results'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .is_ok());
    }

    #[test]
    fn migrates_v2_to_post_confirmation_schema_without_changing_existing_tables() {
        let connection = Connection::open_in_memory().expect("database");
        configure_sqlite_connection(&connection).expect("policy");
        connection
            .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
            .expect("session schema");
        connection
            .execute_batch(crate::orchestration::repository::ORCHESTRATION_SCHEMA)
            .expect("orchestration schema");
        connection
            .execute_batch(crate::orchestration::repository::ORCHESTRATION_INITIATION_SCHEMA)
            .expect("initiation schema");
        connection
            .execute("INSERT INTO epic_planning_drafts (id,status,created_at,updated_at) VALUES ('preserved-v2','active','t','t')", [])
            .expect("preserved v2 row");
        connection
            .pragma_update(None, "user_version", 2)
            .expect("v2");

        initialize_active_database(&connection).expect("migrate v2");

        assert_eq!(
            pragma_i64(&connection, "user_version"),
            ACTIVE_SCHEMA_VERSION
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT status FROM epic_planning_drafts WHERE id='preserved-v2'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "active"
        );
        for table in [
            "epic_bootstrap_transitions",
            "epic_bootstrap_attempts",
            "epic_bootstrap_attempt_completion_commands",
            "epic_bootstrap_attempt_completion_results",
            "epic_bootstrap_attempt_completion_facts",
            "epic_bootstrap_completion_commands",
            "epic_bootstrap_completion_results",
            "epic_bootstrap_completion_facts",
        ] {
            assert!(
                connection
                    .query_row(
                        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                        [table],
                        |_| Ok(()),
                    )
                    .is_ok(),
                "missing {table}"
            );
        }
    }

    #[test]
    fn migrates_v3_single_bootstrap_fact_into_attempt_zero_without_changing_bytes() {
        let connection = Connection::open_in_memory().expect("database");
        configure_sqlite_connection(&connection).expect("policy");
        connection
            .execute_batch(crate::orchestration::bootstrap_transition::POST_CONFIRMATION_SCHEMA)
            .expect("v3 transition schema");
        connection
            .pragma_update(None, "foreign_keys", false)
            .expect("seed v3 rows");
        connection.execute("INSERT INTO epic_bootstrap_transitions (initiation_id,epic_id,proposal_revision_id,material_snapshot_hash,proposal_json,preparation_id,prepared_root,approved_plan_path,manifest_path,overview_path,runner_brief_path,bootstrap_session_id,bootstrap_invocation_id,runner_session_id,runner_invocation_id,prepared_at,bootstrap_session_created_at,bootstrap_launched_at,bootstrap_lifecycle_status,bootstrap_lifecycle_observed_at,semantic_completion_fact_id,semantic_completed_at,created_at,updated_at) VALUES ('initiation-v3','epic-v3','revision-v3','snapshot-v3','{}','preparation-v3','root-v3','plan-v3','manifest-v3','overview-v3','brief-v3','bootstrap-session-v3','bootstrap-invocation-v3','runner-session-v3','runner-invocation-v3','t','t','t','interrupted','t','fact-v3','t','t','t')", []).expect("transition");
        let payload = r#"{"epicOverviewMarkdown":"exact","runnerBriefMarkdown":"bytes"}"#;
        let inventory =
            r#"[{"kind":"epic_overview","path":"overview-v3","sha256":"hash","sizeBytes":5}]"#;
        connection.execute("INSERT INTO epic_bootstrap_completion_commands (id,transition_id,agent_session_id,agent_invocation_id,payload_hash,payload_json,recorded_at) VALUES ('command-v3','initiation-v3','bootstrap-session-v3','bootstrap-invocation-v3','payload-hash',?1,'t')", [payload]).expect("command");
        connection.execute("INSERT INTO epic_bootstrap_completion_results (id,command_id,inventory_json,recorded_at) VALUES ('result-v3','command-v3',?1,'t')", [inventory]).expect("result");
        connection.execute("INSERT INTO epic_bootstrap_completion_facts (id,transition_id,command_id,result_id,inventory_json,recorded_at) VALUES ('fact-v3','initiation-v3','command-v3','result-v3',?1,'t')", [inventory]).expect("fact");
        connection
            .pragma_update(None, "user_version", 3)
            .expect("v3");
        connection
            .pragma_update(None, "foreign_keys", true)
            .expect("foreign keys");

        initialize_active_database(&connection).expect("migrate v3");

        assert_eq!(
            pragma_i64(&connection, "user_version"),
            ACTIVE_SCHEMA_VERSION
        );
        let (ordinal, invocation, disposition, fact): (i64, String, String, Option<String>) = connection
            .query_row("SELECT ordinal,agent_invocation_id,retry_disposition,semantic_completion_fact_id FROM epic_bootstrap_attempts WHERE transition_id='initiation-v3'", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .expect("migrated attempt");
        assert_eq!(
            (
                ordinal,
                invocation.as_str(),
                disposition.as_str(),
                fact.as_deref()
            ),
            (0, "bootstrap-invocation-v3", "retryable", Some("fact-v3"))
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT payload_json FROM epic_bootstrap_attempt_completion_commands WHERE attempt_id='epic-bootstrap-attempt-0-initiation-v3'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("migrated payload"),
            payload
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT inventory_json FROM epic_bootstrap_attempt_completion_facts WHERE attempt_id='epic-bootstrap-attempt-0-initiation-v3'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("migrated inventory"),
            inventory
        );
    }

    #[test]
    fn v1_to_current_migration_reopens_with_exact_agent_session_history_and_proposal_bytes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("active-v1.sqlite");
        let connection = Connection::open(&path).expect("database");
        configure_sqlite_connection(&connection).expect("policy");
        connection
            .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
            .expect("session v1");
        connection
            .execute_batch(crate::orchestration::repository::ORCHESTRATION_SCHEMA)
            .expect("orchestration v1");
        connection
            .pragma_update(None, "foreign_keys", false)
            .expect("seed");
        connection.execute("INSERT INTO agent_sessions (id,title,availability,requested_options_json,created_at,updated_at) VALUES ('session-v1','Session','available','{\"model\":\"test\"}','t','t')", []).expect("session");
        connection.execute("INSERT INTO agent_session_invocations (id,session_id,submitted_text,input_provenance,status,requested_options_json,created_at,updated_at) VALUES ('invocation-v1','session-v1','preserve exact history','user','completed','{}','t','t')", []).expect("invocation");
        let event_bytes = r#"{"type":"item.completed","payload":"bytes stay exact"}"#;
        connection.execute("INSERT INTO agent_session_runtime_events (id,invocation_id,sequence,source,raw_payload_json,recorded_at) VALUES ('event-v1','invocation-v1',0,'runtime',?1,'t')", [event_bytes]).expect("runtime event");
        connection.execute("INSERT INTO epic_planning_drafts (id,status,created_at,updated_at) VALUES ('draft-v1','active','t','t')", []).expect("draft");
        connection.execute("INSERT INTO proposal_commands (id,idempotency_key,draft_id,capability_profile_id,agent_session_association_id,actor_id,proposal_json,payload_fingerprint,recorded_at) VALUES ('command-v1','key-v1','draft-v1','profile-v1','association-v1','actor','{}','fingerprint','t')", []).expect("command");
        connection.execute("INSERT INTO effect_provenance (id,source_kind,recorded_at,actor_id,agent_session_association_id,capability_profile_id,causal_command_id,causal_result_id) VALUES ('provenance-v1','managed_plan_builder','t','actor','association-v1','profile-v1','command-v1','result-v1')", []).expect("provenance");
        let proposal_bytes = r#"{"suggestedEpicName":"Preserved","sprints":[{"title":"Sprint","intendedMovement":"Move","concernSummaries":[]}]} "#;
        connection.execute("INSERT INTO proposal_revisions (id,draft_id,revision_token,proposal_json,command_id,provenance_id,recorded_at) VALUES ('revision-v1','draft-v1','token-v1',?1,'command-v1','provenance-v1','t')", [proposal_bytes]).expect("revision");
        connection
            .pragma_update(None, "user_version", 1)
            .expect("v1");
        connection
            .pragma_update(None, "foreign_keys", true)
            .expect("foreign keys");
        initialize_active_database(&connection).expect("migrate");
        drop(connection);

        let reopened = open_active_database(&path).expect("reopen");
        assert_eq!(pragma_i64(&reopened, "user_version"), ACTIVE_SCHEMA_VERSION);
        assert_eq!(
            reopened
                .query_row(
                    "SELECT raw_payload_json FROM agent_session_runtime_events WHERE id='event-v1'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .expect("event"),
            event_bytes
        );
        assert_eq!(
            reopened
                .query_row(
                    "SELECT proposal_json FROM proposal_revisions WHERE id='revision-v1'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .expect("proposal"),
            proposal_bytes
        );
        assert_eq!(
            reopened
                .query_row(
                    "SELECT session_id FROM agent_session_invocations WHERE id='invocation-v1'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .expect("invocation"),
            "session-v1"
        );
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
            assert!(
                reopened
                    .query_row(
                        &format!(
                            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='{table}'"
                        ),
                        [],
                        |_| Ok(())
                    )
                    .is_ok(),
                "missing {table}"
            );
        }
    }

    fn pragma_i64(connection: &Connection, name: &str) -> i64 {
        connection
            .query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
            .expect("pragma value")
    }
}
