//! Historical desktop database migrations remain tested by their owning package.
use crate::runtime::providers::codex::profiles::*;
use rusqlite::Connection;
#[test]
fn historical_profile_tables_are_restored() {
    let directory = tempfile::tempdir().unwrap();
    let connection =
        crate::storage::open_active_database(&directory.path().join("migration.sqlite")).unwrap();
    connection
            .execute_batch(
                "DROP TABLE native_codex_profile_readiness; DROP TABLE native_codex_profiles; PRAGMA user_version=20;",
            )
            .unwrap();
    crate::storage::initialize_active_database(&connection).unwrap();
    assert!(connection
        .query_row("SELECT 1 FROM native_codex_profiles", [], |_| Ok(()))
        .is_err());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        crate::storage::ACTIVE_SCHEMA_VERSION
    );
}
#[test]
fn v31_and_v32_migrations_add_external_adoption_storage_without_setup_backfill() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let connection = crate::storage::open_active_database(&database).unwrap();
    connection.execute_batch("DROP TABLE native_codex_profile_sandbox_adoption_confirmations; DROP TABLE native_codex_profile_sandbox_adoptions; PRAGMA user_version=30;").unwrap();
    crate::storage::initialize_active_database(&connection).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        crate::storage::ACTIVE_SCHEMA_VERSION
    );
    assert_eq!(connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='native_codex_profile_sandbox_adoptions')", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
    assert_eq!(connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='native_codex_profile_sandbox_adoption_confirmations')", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
}

#[test]
fn v34_migration_preserves_old_danger_authorization_as_insufficient_and_rejects_old_canary_pass() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE native_codex_profiles (id TEXT PRIMARY KEY, updated_at TEXT NOT NULL);",
        )
        .unwrap();
    connection
            .execute("INSERT INTO native_codex_profiles (id,updated_at) VALUES ('p1','2026-08-07T12:00:00Z')", [])
            .unwrap();
    connection.execute_batch("CREATE TABLE native_codex_profile_readiness (profile_id TEXT PRIMARY KEY, danger_full_access_canary TEXT NOT NULL);").unwrap();
    connection.execute("INSERT INTO native_codex_profile_readiness (profile_id,danger_full_access_canary) VALUES ('p1','passed')", []).unwrap();
    connection
        .execute_batch(NATIVE_PROFILE_V25_MIGRATION)
        .unwrap();
    connection
        .execute_batch(NATIVE_PROFILE_V26_MIGRATION)
        .unwrap();
    connection.execute(
            "INSERT INTO native_codex_profile_mode_authorizations (profile_id,mode,filesystem_identity,authorized_at) VALUES ('p1','danger_full_access','identity','2026-08-07T12:00:00Z')",
            [],
        ).unwrap();
    connection.execute(
            "INSERT INTO native_codex_profile_full_access_canaries (attempt_id,profile_id,filesystem_identity,mode,executable,version,sentinel_path,state,started_at,completed_at) VALUES ('a1','p1','identity','danger_full_access','C:/codex.exe','codex-cli 0.144.0','C:/owned.txt','passed','2026-08-07T12:00:00Z','2026-08-07T12:00:01Z')",
            [],
        ).unwrap();
    connection
        .execute_batch(NATIVE_PROFILE_V34_MIGRATION)
        .unwrap();
    connection
        .execute_batch(NATIVE_PROFILE_V34_FULL_ACCESS_CANARY_MIGRATION)
        .unwrap();
    let authorization: (String, String, Option<String>) = connection.query_row(
            "SELECT authority_scope,authority_version,authorization_correlation_id FROM native_codex_profile_mode_authorizations WHERE profile_id='p1'",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
    assert_eq!(
        authorization,
        (
            "filesystem_only".into(),
            "danger-full-access/filesystem-only/v1".into(),
            None
        )
    );
    let canary: (String, String, String) = connection.query_row(
            "SELECT state,terminal_classification,cleanup_disposition FROM native_codex_profile_full_access_canaries WHERE attempt_id='a1'",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
    assert_eq!(
        canary,
        (
            "legacy_unverified".into(),
            "legacy_unverified".into(),
            "not_observed".into()
        )
    );
}

#[test]
fn v21_readiness_migration_preserves_facts_and_maps_retired_states() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let connection = crate::storage::open_active_database(&database).unwrap();
    connection.execute_batch("DROP TABLE native_codex_profile_readiness; CREATE TABLE native_codex_profile_readiness (profile_id TEXT PRIMARY KEY, authentication TEXT NOT NULL, sandbox_initialization TEXT NOT NULL, workspace_write_canary TEXT NOT NULL, mcp_reporting TEXT NOT NULL, attention TEXT, login_requested_at TEXT, observed_at TEXT NOT NULL); INSERT INTO native_codex_profiles (id,canonical_home_path,filesystem_identity,ownership,lifecycle,created_at,updated_at) VALUES ('profile','C:\\profile','identity','registered_existing','active','t','t'); INSERT INTO native_codex_profile_readiness VALUES ('profile','authenticated','unsupported','blocked','not_configured','legacy','t','t'); PRAGMA user_version=21;").unwrap();
    crate::storage::initialize_active_database(&connection).unwrap();
    let row: (String, String) = connection.query_row("SELECT sandbox_initialization,mcp_reporting FROM native_codex_profile_readiness WHERE profile_id='profile'", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    assert_eq!(row, ("attention_required".into(), "not_assessed".into()));
    let attention: String = connection
            .query_row(
                "SELECT detail FROM native_codex_profile_attentions WHERE profile_id='profile' AND concern='continuity'",
                [],
                |row| row.get(0),
            )
            .unwrap();
    assert_eq!(attention, "legacy");
}

#[test]
fn v28_setup_attempt_migration_preserves_each_legacy_state_without_fabricating_provenance() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let connection = Connection::open(&database).unwrap();
    connection.execute_batch(NATIVE_PROFILE_SCHEMA).unwrap();
    connection.execute_batch("DROP INDEX ux_native_codex_profile_setup_attempt_pending; DROP TABLE native_codex_profile_setup_attempts; CREATE TABLE native_codex_profile_setup_attempts (attempt_id TEXT PRIMARY KEY, profile_id TEXT NOT NULL, phase TEXT NOT NULL, state TEXT NOT NULL, started_at TEXT NOT NULL, deadline_at TEXT NOT NULL, completed_at TEXT); INSERT INTO native_codex_profiles (id,canonical_home_path,filesystem_identity,ownership,lifecycle,created_at,updated_at) VALUES ('profile','C:\\profile','identity','registered_existing','active','t','t'); INSERT INTO native_codex_profile_setup_attempts VALUES ('failed','profile','sandbox_initialization','failed','2026-08-06T22:41:12Z','2026-08-06T22:43:12Z','2026-08-06T22:41:13Z'),('pending','profile','sandbox_initialization','pending','2026-08-06T22:42:12Z','2026-08-06T22:44:12Z',NULL),('completed','profile','sandbox_initialization','completed','2026-08-06T22:43:12Z','2026-08-06T22:45:12Z','2026-08-06T22:43:13Z'),('timed_out','profile','sandbox_initialization','timed_out','2026-08-06T22:44:12Z','2026-08-06T22:46:12Z','2026-08-06T22:46:12Z'),('cancelled','profile','sandbox_initialization','cancelled','2026-08-06T22:45:12Z','2026-08-06T22:47:12Z','2026-08-06T22:45:13Z'); PRAGMA user_version=27;").unwrap();
    crate::storage::initialize_active_database(&connection).unwrap();
    let mut statement = connection.prepare("SELECT attempt_id,state,terminal_classification,executable,version,workspace_sandbox_supported,launch_accepted_at,terminal_exit_code FROM native_codex_profile_setup_attempts ORDER BY requested_at").unwrap();
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<i32>>(7)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        rows,
        vec![
            (
                "failed".into(),
                "legacy_unclassified_failed".into(),
                "legacy_unclassified_failed".into(),
                None,
                None,
                None,
                None,
                None
            ),
            (
                "pending".into(),
                "recovered_unobserved".into(),
                "recovered_unobserved".into(),
                None,
                None,
                None,
                None,
                None
            ),
            (
                "completed".into(),
                "terminal_succeeded".into(),
                "not_observed".into(),
                None,
                None,
                None,
                None,
                None
            ),
            (
                "timed_out".into(),
                "timed_out".into(),
                "timed_out".into(),
                None,
                None,
                None,
                None,
                None
            ),
            (
                "cancelled".into(),
                "cancelled".into(),
                "cancelled".into(),
                None,
                None,
                None,
                None,
                None
            ),
        ]
    );
}

#[test]
fn v30_policy_invariant_migration_preserves_existing_v28_attempt_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let connection = Connection::open(&database).unwrap();
    connection.execute_batch(NATIVE_PROFILE_SCHEMA).unwrap();
    connection.execute_batch("DROP INDEX ux_native_codex_profile_setup_attempt_pending; DROP TABLE native_codex_profile_setup_attempts; CREATE TABLE native_codex_profile_setup_attempts (attempt_id TEXT PRIMARY KEY, profile_id TEXT NOT NULL, filesystem_identity TEXT NOT NULL, phase TEXT NOT NULL CHECK (phase IN ('sandbox_initialization','workspace_write_canary')), state TEXT NOT NULL CHECK (state IN ('pending','launch_failed','terminal_succeeded','terminal_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed')), executable TEXT, version TEXT, workspace_sandbox_supported INTEGER, correlation_id TEXT NOT NULL UNIQUE, requested_at TEXT NOT NULL, launch_accepted_at TEXT, deadline_at TEXT NOT NULL, settled_at TEXT, terminal_classification TEXT NOT NULL CHECK (terminal_classification IN ('not_observed','exit_code','launch_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed')), terminal_exit_code INTEGER, FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT); INSERT INTO native_codex_profiles (id,canonical_home_path,filesystem_identity,ownership,lifecycle,created_at,updated_at) VALUES ('profile','C:\\profile','identity','registered_existing','active','t','t'); INSERT INTO native_codex_profile_setup_attempts VALUES ('attempt','profile','identity','sandbox_initialization','terminal_failed','C:/application-owned/codex.exe','codex-cli test',1,'correlation','2026-08-07T12:00:00Z','2026-08-07T12:00:01Z','2026-08-07T12:02:00Z','2026-08-07T12:00:02Z','exit_code',1); PRAGMA user_version=28;").unwrap();

    crate::storage::initialize_active_database(&connection).unwrap();

    let row: (String, String, String, String, Option<i32>) = connection
            .query_row(
                "SELECT state,executable,version,terminal_classification,terminal_exit_code FROM native_codex_profile_setup_attempts WHERE attempt_id='attempt'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
    assert_eq!(
        row,
        (
            "terminal_failed".into(),
            "C:/application-owned/codex.exe".into(),
            "codex-cli test".into(),
            "exit_code".into(),
            Some(1),
        )
    );
    connection
            .execute("INSERT INTO native_codex_profile_setup_attempts (attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,deadline_at,settled_at,terminal_classification) VALUES ('unsupported','profile','identity','sandbox_initialization','policy_unsupported','C:/application-owned/codex.exe','codex-cli test',0,'correlation-unsupported','2026-08-07T12:03:00Z','2026-08-07T12:05:00Z','2026-08-07T12:03:00Z','policy_unsupported')", [])
            .unwrap();
}

#[test]
fn v33_canary_receipt_classification_migration_preserves_existing_terminal_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let connection = Connection::open(&database).unwrap();
    connection.execute_batch(NATIVE_PROFILE_SCHEMA).unwrap();
    connection.execute_batch("DROP INDEX ux_native_codex_profile_setup_attempt_pending; DROP TABLE native_codex_profile_setup_attempts; CREATE TABLE native_codex_profile_setup_attempts (attempt_id TEXT PRIMARY KEY, profile_id TEXT NOT NULL, filesystem_identity TEXT NOT NULL, phase TEXT NOT NULL CHECK (phase IN ('sandbox_initialization','workspace_write_canary')), state TEXT NOT NULL CHECK (state IN ('pending','launch_failed','terminal_succeeded','terminal_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')), executable TEXT, version TEXT, workspace_sandbox_supported INTEGER, correlation_id TEXT NOT NULL UNIQUE, requested_at TEXT NOT NULL, launch_accepted_at TEXT, deadline_at TEXT NOT NULL, settled_at TEXT, terminal_classification TEXT NOT NULL CHECK (terminal_classification IN ('not_observed','exit_code','launch_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')), terminal_exit_code INTEGER, CHECK (state <> 'policy_unsupported' OR (phase IN ('sandbox_initialization','workspace_write_canary') AND terminal_classification='policy_unsupported' AND workspace_sandbox_supported=0 AND executable IS NOT NULL AND length(trim(executable))>0 AND version IS NOT NULL AND length(trim(version))>0 AND length(trim(correlation_id))>0 AND length(trim(requested_at))>0 AND length(trim(deadline_at))>0 AND settled_at IS NOT NULL AND length(trim(settled_at))>0 AND launch_accepted_at IS NULL AND terminal_exit_code IS NULL)), FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT); INSERT INTO native_codex_profiles (id,canonical_home_path,filesystem_identity,ownership,lifecycle,created_at,updated_at) VALUES ('profile','C:\\profile','identity','registered_existing','active','t','t'); INSERT INTO native_codex_profile_setup_attempts VALUES ('attempt','profile','identity','workspace_write_canary','terminal_failed','C:/application-owned/codex.exe','codex-cli test',1,'correlation','2026-08-07T12:00:00Z','2026-08-07T12:00:01Z','2026-08-07T12:02:00Z','2026-08-07T12:00:02Z','exit_code',1); PRAGMA user_version=32;").unwrap();

    crate::storage::initialize_active_database(&connection).unwrap();

    let row: (String, String, Option<i32>) = connection
            .query_row(
                "SELECT state,terminal_classification,terminal_exit_code FROM native_codex_profile_setup_attempts WHERE attempt_id='attempt'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
    assert_eq!(row, ("terminal_failed".into(), "exit_code".into(), Some(1)));
    connection
            .execute(
                "INSERT INTO native_codex_profile_setup_attempts (attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,launch_accepted_at,deadline_at,settled_at,terminal_classification,terminal_exit_code) VALUES ('receipt-missing','profile','identity','workspace_write_canary','terminal_failed','C:/application-owned/codex.exe','codex-cli test',1,'correlation-receipt','2026-08-07T12:03:00Z','2026-08-07T12:03:01Z','2026-08-07T12:05:00Z','2026-08-07T12:03:02Z','receipt_missing',1)",
                [],
            )
            .unwrap();
}
