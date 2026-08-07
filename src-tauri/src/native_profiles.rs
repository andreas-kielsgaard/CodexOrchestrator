//! Product-owned Codex home profiles. This module deliberately records only filesystem identity
//! and bounded setup observations; it never reads authentication, sandbox, or provider payloads.

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, OnceLock},
};
use tauri::State;
use uuid::Uuid;

pub(crate) const NATIVE_PROFILE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS native_codex_profiles (
  id TEXT PRIMARY KEY,
  canonical_home_path TEXT NOT NULL UNIQUE,
  filesystem_identity TEXT NOT NULL,
  ownership TEXT NOT NULL CHECK (ownership IN ('registered_existing','application_dedicated')),
  lifecycle TEXT NOT NULL CHECK (lifecycle IN ('active','missing_or_moved','replaced','foreign','malformed')),
  selected_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profiles_selected
ON native_codex_profiles((1)) WHERE selected_at IS NOT NULL;
CREATE TABLE IF NOT EXISTS native_codex_profile_readiness (
  profile_id TEXT PRIMARY KEY,
  authentication TEXT NOT NULL CHECK (authentication IN ('unknown','authenticated','unauthenticated')),
  sandbox_initialization TEXT NOT NULL CHECK (sandbox_initialization IN ('unknown','initialized','failed','attention_required')),
  workspace_write_canary TEXT NOT NULL CHECK (workspace_write_canary IN ('not_run','passed','blocked')),
  danger_full_access_canary TEXT NOT NULL DEFAULT 'not_run' CHECK (danger_full_access_canary IN ('not_run','passed','blocked')),
  mcp_reporting TEXT NOT NULL CHECK (mcp_reporting IN ('not_assessed','ready','probe_failed')),
  attention TEXT,
  login_requested_at TEXT,
  observed_at TEXT NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS native_codex_profile_attentions (
  profile_id TEXT NOT NULL,
  concern TEXT NOT NULL CHECK (concern IN ('authentication','sandbox','canary','mcp_reporting','continuity','cli')),
  detail TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  PRIMARY KEY(profile_id, concern),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS native_codex_profile_setup_attempts (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  phase TEXT NOT NULL CHECK (phase IN ('sandbox_initialization','workspace_write_canary')),
  state TEXT NOT NULL CHECK (state IN ('pending','launch_failed','terminal_succeeded','terminal_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')),
  executable TEXT,
  version TEXT,
  workspace_sandbox_supported INTEGER,
  correlation_id TEXT NOT NULL UNIQUE,
  requested_at TEXT NOT NULL,
  launch_accepted_at TEXT,
  deadline_at TEXT NOT NULL,
  settled_at TEXT,
  terminal_classification TEXT NOT NULL CHECK (terminal_classification IN ('not_observed','exit_code','receipt_missing','launch_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')),
  terminal_exit_code INTEGER,
  CHECK (state <> 'policy_unsupported' OR (phase IN ('sandbox_initialization','workspace_write_canary') AND terminal_classification='policy_unsupported' AND workspace_sandbox_supported=0 AND executable IS NOT NULL AND length(trim(executable))>0 AND version IS NOT NULL AND length(trim(version))>0 AND length(trim(correlation_id))>0 AND length(trim(requested_at))>0 AND length(trim(deadline_at))>0 AND settled_at IS NOT NULL AND length(trim(settled_at))>0 AND launch_accepted_at IS NULL AND terminal_exit_code IS NULL)),
  CHECK (terminal_classification <> 'receipt_missing' OR (phase='workspace_write_canary' AND state='terminal_failed' AND workspace_sandbox_supported=1 AND executable IS NOT NULL AND length(trim(executable))>0 AND version IS NOT NULL AND length(trim(version))>0 AND length(trim(correlation_id))>0 AND length(trim(requested_at))>0 AND launch_accepted_at IS NOT NULL AND length(trim(launch_accepted_at))>0 AND length(trim(deadline_at))>0 AND settled_at IS NOT NULL AND length(trim(settled_at))>0)),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_setup_attempt_pending
ON native_codex_profile_setup_attempts(profile_id,phase) WHERE state='pending';
CREATE TABLE IF NOT EXISTS native_codex_profile_mcp_probes (
  request_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  correlation_id TEXT NOT NULL UNIQUE,
  expected_capability TEXT NOT NULL,
  expected_server TEXT NOT NULL,
  expected_tool TEXT NOT NULL,
  expected_probe_root TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('pending','received','expired','cancelled')),
  requested_at TEXT NOT NULL,
  deadline_at TEXT NOT NULL,
  received_at TEXT,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_mcp_probe_pending
ON native_codex_profile_mcp_probes(profile_id) WHERE state='pending';
CREATE TABLE IF NOT EXISTS native_codex_profile_execution_modes (
  profile_id TEXT PRIMARY KEY,
  selected_mode TEXT NOT NULL CHECK (selected_mode IN ('workspace_write','danger_full_access')),
  updated_at TEXT NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS native_codex_profile_mode_authorizations (
  profile_id TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode='danger_full_access'),
  filesystem_identity TEXT NOT NULL,
  authorized_at TEXT NOT NULL,
  revoked_at TEXT,
  PRIMARY KEY(profile_id, mode),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS native_codex_profile_full_access_canaries (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode='danger_full_access'),
  executable TEXT NOT NULL,
  version TEXT NOT NULL,
  sentinel_path TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('pending','passed','blocked','cancelled')),
  started_at TEXT NOT NULL,
  completed_at TEXT,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_full_access_canary_pending
ON native_codex_profile_full_access_canaries(profile_id) WHERE state='pending';
CREATE TABLE IF NOT EXISTS native_codex_profile_login_attempts (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  executable TEXT NOT NULL,
  version TEXT NOT NULL,
  correlation_id TEXT NOT NULL UNIQUE,
  state TEXT NOT NULL CHECK (state IN ('pending','launch_failed','terminal_succeeded','terminal_failed','cancelled','recovered_unobserved')),
  browser_handoff TEXT NOT NULL CHECK (browser_handoff='unobserved'),
  requested_at TEXT NOT NULL,
  launch_accepted_at TEXT,
  settled_at TEXT,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_login_attempt_pending
ON native_codex_profile_login_attempts(profile_id) WHERE state='pending';
CREATE TABLE IF NOT EXISTS native_codex_profile_sandbox_adoptions (
  profile_id TEXT PRIMARY KEY,
  filesystem_identity TEXT NOT NULL,
  executable TEXT NOT NULL,
  version TEXT NOT NULL,
  workspace_sandbox_supported INTEGER NOT NULL CHECK (workspace_sandbox_supported IN (0,1)),
  windows_sandbox_setup_supported INTEGER NOT NULL CHECK (windows_sandbox_setup_supported IN (0,1)),
  correlation_id TEXT NOT NULL UNIQUE,
  observed_at TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('verified','not_verified','invalidated')),
  elevated_mode_observed INTEGER NOT NULL CHECK (elevated_mode_observed IN (0,1)),
  CHECK (state <> 'verified' OR (workspace_sandbox_supported=1 AND windows_sandbox_setup_supported=1 AND elevated_mode_observed=1)),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS native_codex_profile_sandbox_adoption_confirmations (
  profile_id TEXT PRIMARY KEY,
  filesystem_identity TEXT NOT NULL,
  adoption_correlation_id TEXT NOT NULL,
  confirmation_correlation_id TEXT NOT NULL UNIQUE,
  confirmed_at TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('confirmed','invalidated')),
  invalidated_at TEXT,
  CHECK ((state='confirmed' AND invalidated_at IS NULL) OR (state='invalidated' AND invalidated_at IS NOT NULL)),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
"#;

pub(crate) const NATIVE_PROFILE_V22_MIGRATION: &str = r#"
ALTER TABLE native_codex_profile_readiness RENAME TO native_codex_profile_readiness_v21;
CREATE TABLE native_codex_profile_readiness (
  profile_id TEXT PRIMARY KEY,
  authentication TEXT NOT NULL CHECK (authentication IN ('unknown','authenticated','unauthenticated')),
  sandbox_initialization TEXT NOT NULL CHECK (sandbox_initialization IN ('unknown','initialized','failed','attention_required')),
  workspace_write_canary TEXT NOT NULL CHECK (workspace_write_canary IN ('not_run','passed','blocked')),
  mcp_reporting TEXT NOT NULL CHECK (mcp_reporting IN ('not_assessed','ready','probe_failed')),
  attention TEXT,
  login_requested_at TEXT,
  observed_at TEXT NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
INSERT INTO native_codex_profile_readiness (profile_id,authentication,sandbox_initialization,workspace_write_canary,mcp_reporting,attention,login_requested_at,observed_at)
SELECT profile_id,authentication,
  CASE sandbox_initialization WHEN 'unsupported' THEN 'attention_required' ELSE sandbox_initialization END,
  workspace_write_canary,
  CASE mcp_reporting WHEN 'not_configured' THEN 'not_assessed' ELSE mcp_reporting END,
  attention,login_requested_at,observed_at
FROM native_codex_profile_readiness_v21;
DROP TABLE native_codex_profile_readiness_v21;
"#;

pub(crate) const NATIVE_PROFILE_V23_MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS native_codex_profile_attentions (
  profile_id TEXT NOT NULL,
  concern TEXT NOT NULL CHECK (concern IN ('authentication','sandbox','canary','mcp_reporting','continuity','cli')),
  detail TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  PRIMARY KEY(profile_id, concern),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
INSERT OR IGNORE INTO native_codex_profile_attentions (profile_id,concern,detail,recorded_at)
SELECT profile_id,'continuity',attention,observed_at
FROM native_codex_profile_readiness
WHERE attention IS NOT NULL;
"#;

pub(crate) const NATIVE_PROFILE_V24_MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS native_codex_profile_setup_attempts (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  phase TEXT NOT NULL CHECK (phase IN ('sandbox_initialization','workspace_write_canary')),
  state TEXT NOT NULL CHECK (state IN ('pending','completed','failed','timed_out','cancelled')),
  started_at TEXT NOT NULL,
  deadline_at TEXT NOT NULL,
  completed_at TEXT,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_setup_attempt_pending
ON native_codex_profile_setup_attempts(profile_id,phase) WHERE state='pending';
CREATE TABLE IF NOT EXISTS native_codex_profile_mcp_probes (
  request_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  correlation_id TEXT NOT NULL UNIQUE,
  expected_capability TEXT NOT NULL,
  expected_server TEXT NOT NULL,
  expected_tool TEXT NOT NULL,
  expected_probe_root TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('pending','received','expired','cancelled')),
  requested_at TEXT NOT NULL,
  deadline_at TEXT NOT NULL,
  received_at TEXT,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_mcp_probe_pending
ON native_codex_profile_mcp_probes(profile_id) WHERE state='pending';
"#;

pub(crate) const NATIVE_PROFILE_V25_MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS native_codex_profile_execution_modes (
  profile_id TEXT PRIMARY KEY,
  selected_mode TEXT NOT NULL CHECK (selected_mode IN ('workspace_write','danger_full_access')),
  updated_at TEXT NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
INSERT OR IGNORE INTO native_codex_profile_execution_modes (profile_id,selected_mode,updated_at)
SELECT id,'workspace_write',updated_at FROM native_codex_profiles;
CREATE TABLE IF NOT EXISTS native_codex_profile_mode_authorizations (
  profile_id TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode='danger_full_access'),
  filesystem_identity TEXT NOT NULL,
  authorized_at TEXT NOT NULL,
  revoked_at TEXT,
  PRIMARY KEY(profile_id, mode),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
"#;

pub(crate) const NATIVE_PROFILE_V26_MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS native_codex_profile_full_access_canaries (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode='danger_full_access'),
  executable TEXT NOT NULL,
  version TEXT NOT NULL,
  sentinel_path TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('pending','passed','blocked','cancelled')),
  started_at TEXT NOT NULL,
  completed_at TEXT,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_full_access_canary_pending
ON native_codex_profile_full_access_canaries(profile_id) WHERE state='pending';
"#;

pub(crate) const NATIVE_PROFILE_V27_MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS native_codex_profile_login_attempts (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  executable TEXT NOT NULL,
  version TEXT NOT NULL,
  correlation_id TEXT NOT NULL UNIQUE,
  state TEXT NOT NULL CHECK (state IN ('pending','launch_failed','terminal_succeeded','terminal_failed','cancelled','recovered_unobserved')),
  browser_handoff TEXT NOT NULL CHECK (browser_handoff='unobserved'),
  requested_at TEXT NOT NULL,
  launch_accepted_at TEXT,
  settled_at TEXT,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_login_attempt_pending
ON native_codex_profile_login_attempts(profile_id) WHERE state='pending';
"#;

pub(crate) const NATIVE_PROFILE_V28_MIGRATION: &str = r#"
ALTER TABLE native_codex_profile_setup_attempts RENAME TO native_codex_profile_setup_attempts_v27;
CREATE TABLE native_codex_profile_setup_attempts (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  phase TEXT NOT NULL CHECK (phase IN ('sandbox_initialization','workspace_write_canary')),
  state TEXT NOT NULL CHECK (state IN ('pending','launch_failed','terminal_succeeded','terminal_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')),
  executable TEXT,
  version TEXT,
  workspace_sandbox_supported INTEGER,
  correlation_id TEXT NOT NULL UNIQUE,
  requested_at TEXT NOT NULL,
  launch_accepted_at TEXT,
  deadline_at TEXT NOT NULL,
  settled_at TEXT,
  terminal_classification TEXT NOT NULL CHECK (terminal_classification IN ('not_observed','exit_code','launch_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')),
  terminal_exit_code INTEGER,
  CHECK (state <> 'policy_unsupported' OR (phase IN ('sandbox_initialization','workspace_write_canary') AND terminal_classification='policy_unsupported' AND workspace_sandbox_supported=0 AND executable IS NOT NULL AND length(trim(executable))>0 AND version IS NOT NULL AND length(trim(version))>0 AND length(trim(correlation_id))>0 AND length(trim(requested_at))>0 AND length(trim(deadline_at))>0 AND settled_at IS NOT NULL AND length(trim(settled_at))>0 AND launch_accepted_at IS NULL AND terminal_exit_code IS NULL)),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
INSERT INTO native_codex_profile_setup_attempts (attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,launch_accepted_at,deadline_at,settled_at,terminal_classification,terminal_exit_code)
SELECT attempt_id,profile_id,'',phase,
  CASE state
    WHEN 'failed' THEN 'legacy_unclassified_failed'
    WHEN 'completed' THEN 'terminal_succeeded'
    WHEN 'timed_out' THEN 'timed_out'
    WHEN 'cancelled' THEN 'cancelled'
    WHEN 'pending' THEN 'recovered_unobserved'
    ELSE 'legacy_unclassified_failed'
  END,
  NULL,NULL,NULL,'legacy-' || attempt_id,started_at,NULL,deadline_at,completed_at,
  CASE state
    WHEN 'failed' THEN 'legacy_unclassified_failed'
    WHEN 'completed' THEN 'not_observed'
    WHEN 'timed_out' THEN 'timed_out'
    WHEN 'cancelled' THEN 'cancelled'
    WHEN 'pending' THEN 'recovered_unobserved'
    ELSE 'legacy_unclassified_failed'
  END,
  NULL
FROM native_codex_profile_setup_attempts_v27;
DROP TABLE native_codex_profile_setup_attempts_v27;
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_setup_attempt_pending
ON native_codex_profile_setup_attempts(profile_id,phase) WHERE state='pending';
"#;

pub(crate) const NATIVE_PROFILE_V29_MIGRATION: &str = r#"
ALTER TABLE native_codex_profile_setup_attempts RENAME TO native_codex_profile_setup_attempts_v28;
CREATE TABLE native_codex_profile_setup_attempts (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  phase TEXT NOT NULL CHECK (phase IN ('sandbox_initialization','workspace_write_canary')),
  state TEXT NOT NULL CHECK (state IN ('pending','launch_failed','terminal_succeeded','terminal_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')),
  executable TEXT,
  version TEXT,
  workspace_sandbox_supported INTEGER,
  correlation_id TEXT NOT NULL UNIQUE,
  requested_at TEXT NOT NULL,
  launch_accepted_at TEXT,
  deadline_at TEXT NOT NULL,
  settled_at TEXT,
  terminal_classification TEXT NOT NULL CHECK (terminal_classification IN ('not_observed','exit_code','launch_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')),
  terminal_exit_code INTEGER,
  CHECK (state <> 'policy_unsupported' OR (phase IN ('sandbox_initialization','workspace_write_canary') AND terminal_classification='policy_unsupported' AND workspace_sandbox_supported=0 AND executable IS NOT NULL AND length(trim(executable))>0 AND version IS NOT NULL AND length(trim(version))>0 AND length(trim(correlation_id))>0 AND length(trim(requested_at))>0 AND length(trim(deadline_at))>0 AND settled_at IS NOT NULL AND length(trim(settled_at))>0 AND launch_accepted_at IS NULL AND terminal_exit_code IS NULL)),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
INSERT INTO native_codex_profile_setup_attempts (attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,launch_accepted_at,deadline_at,settled_at,terminal_classification,terminal_exit_code)
SELECT attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,launch_accepted_at,deadline_at,settled_at,terminal_classification,terminal_exit_code
FROM native_codex_profile_setup_attempts_v28;
DROP TABLE native_codex_profile_setup_attempts_v28;
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_setup_attempt_pending
ON native_codex_profile_setup_attempts(profile_id,phase) WHERE state='pending';
"#;

pub(crate) const NATIVE_PROFILE_V30_MIGRATION: &str = r#"
ALTER TABLE native_codex_profile_setup_attempts RENAME TO native_codex_profile_setup_attempts_v29;
CREATE TABLE native_codex_profile_setup_attempts (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  phase TEXT NOT NULL CHECK (phase IN ('sandbox_initialization','workspace_write_canary')),
  state TEXT NOT NULL CHECK (state IN ('pending','launch_failed','terminal_succeeded','terminal_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')),
  executable TEXT,
  version TEXT,
  workspace_sandbox_supported INTEGER,
  correlation_id TEXT NOT NULL UNIQUE,
  requested_at TEXT NOT NULL,
  launch_accepted_at TEXT,
  deadline_at TEXT NOT NULL,
  settled_at TEXT,
  terminal_classification TEXT NOT NULL CHECK (terminal_classification IN ('not_observed','exit_code','launch_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')),
  terminal_exit_code INTEGER,
  CHECK (state <> 'policy_unsupported' OR (phase IN ('sandbox_initialization','workspace_write_canary') AND terminal_classification='policy_unsupported' AND workspace_sandbox_supported=0 AND executable IS NOT NULL AND length(trim(executable))>0 AND version IS NOT NULL AND length(trim(version))>0 AND length(trim(correlation_id))>0 AND length(trim(requested_at))>0 AND length(trim(deadline_at))>0 AND settled_at IS NOT NULL AND length(trim(settled_at))>0 AND launch_accepted_at IS NULL AND terminal_exit_code IS NULL)),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
INSERT INTO native_codex_profile_setup_attempts (attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,launch_accepted_at,deadline_at,settled_at,terminal_classification,terminal_exit_code)
SELECT attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,launch_accepted_at,deadline_at,settled_at,terminal_classification,terminal_exit_code
FROM native_codex_profile_setup_attempts_v29;
DROP TABLE native_codex_profile_setup_attempts_v29;
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_setup_attempt_pending
ON native_codex_profile_setup_attempts(profile_id,phase) WHERE state='pending';
"#;

pub(crate) const NATIVE_PROFILE_V31_MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS native_codex_profile_sandbox_adoptions (
  profile_id TEXT PRIMARY KEY,
  filesystem_identity TEXT NOT NULL,
  executable TEXT NOT NULL,
  version TEXT NOT NULL,
  workspace_sandbox_supported INTEGER NOT NULL CHECK (workspace_sandbox_supported IN (0,1)),
  windows_sandbox_setup_supported INTEGER NOT NULL CHECK (windows_sandbox_setup_supported IN (0,1)),
  correlation_id TEXT NOT NULL UNIQUE,
  observed_at TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('verified','not_verified','invalidated')),
  elevated_mode_observed INTEGER NOT NULL CHECK (elevated_mode_observed IN (0,1)),
  CHECK (state <> 'verified' OR (workspace_sandbox_supported=1 AND windows_sandbox_setup_supported=1 AND elevated_mode_observed=1)),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
"#;

pub(crate) const NATIVE_PROFILE_V32_MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS native_codex_profile_sandbox_adoption_confirmations (
  profile_id TEXT PRIMARY KEY,
  filesystem_identity TEXT NOT NULL,
  adoption_correlation_id TEXT NOT NULL,
  confirmation_correlation_id TEXT NOT NULL UNIQUE,
  confirmed_at TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('confirmed','invalidated')),
  invalidated_at TEXT,
  CHECK ((state='confirmed' AND invalidated_at IS NULL) OR (state='invalidated' AND invalidated_at IS NOT NULL)),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
"#;

pub(crate) const NATIVE_PROFILE_V33_MIGRATION: &str = r#"
ALTER TABLE native_codex_profile_setup_attempts RENAME TO native_codex_profile_setup_attempts_v32;
CREATE TABLE native_codex_profile_setup_attempts (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  phase TEXT NOT NULL CHECK (phase IN ('sandbox_initialization','workspace_write_canary')),
  state TEXT NOT NULL CHECK (state IN ('pending','launch_failed','terminal_succeeded','terminal_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')),
  executable TEXT,
  version TEXT,
  workspace_sandbox_supported INTEGER,
  correlation_id TEXT NOT NULL UNIQUE,
  requested_at TEXT NOT NULL,
  launch_accepted_at TEXT,
  deadline_at TEXT NOT NULL,
  settled_at TEXT,
  terminal_classification TEXT NOT NULL CHECK (terminal_classification IN ('not_observed','exit_code','receipt_missing','launch_failed','timed_out','cancelled','recovered_unobserved','legacy_unclassified_failed','policy_unsupported')),
  terminal_exit_code INTEGER,
  CHECK (state <> 'policy_unsupported' OR (phase IN ('sandbox_initialization','workspace_write_canary') AND terminal_classification='policy_unsupported' AND workspace_sandbox_supported=0 AND executable IS NOT NULL AND length(trim(executable))>0 AND version IS NOT NULL AND length(trim(version))>0 AND length(trim(correlation_id))>0 AND length(trim(requested_at))>0 AND length(trim(deadline_at))>0 AND settled_at IS NOT NULL AND length(trim(settled_at))>0 AND launch_accepted_at IS NULL AND terminal_exit_code IS NULL)),
  CHECK (terminal_classification <> 'receipt_missing' OR (phase='workspace_write_canary' AND state='terminal_failed' AND workspace_sandbox_supported=1 AND executable IS NOT NULL AND length(trim(executable))>0 AND version IS NOT NULL AND length(trim(version))>0 AND length(trim(correlation_id))>0 AND length(trim(requested_at))>0 AND launch_accepted_at IS NOT NULL AND length(trim(launch_accepted_at))>0 AND length(trim(deadline_at))>0 AND settled_at IS NOT NULL AND length(trim(settled_at))>0)),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
INSERT INTO native_codex_profile_setup_attempts (attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,launch_accepted_at,deadline_at,settled_at,terminal_classification,terminal_exit_code)
SELECT attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,launch_accepted_at,deadline_at,settled_at,terminal_classification,terminal_exit_code
FROM native_codex_profile_setup_attempts_v32;
DROP TABLE native_codex_profile_setup_attempts_v32;
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_setup_attempt_pending
ON native_codex_profile_setup_attempts(profile_id,phase) WHERE state='pending';
"#;

const MARKER_FILE: &str = ".codex-orchestrator-profile.json";
const PROFILE_QUERY_CONTRACT: &str = "native-codex-profile-query/v1";
const MCP_REPORTING_CAPABILITY: &str = "native-codex-profile-reporting/v1";
const MCP_REPORTING_SERVER: &str = "codex-orchestrator-reporting";
const MCP_REPORTING_TOOL: &str = "report_native_profile_readiness";
const SETUP_ATTEMPT_TIMEOUT_SECONDS: i64 = 120;
const MCP_PROBE_TIMEOUT_SECONDS: i64 = 300;
const WORKSPACE_WRITE_CANARY_COMMAND_FILE: &str = "native-codex-profile-canary.cmd";
static NATIVE_PROFILE_OPEN_GATE: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeCliInvocation {
    args: Vec<String>,
    cwd: PathBuf,
    codex_home: PathBuf,
    environment: Vec<(String, String)>,
    sandbox_receipt: Option<PathBuf>,
    sandbox_command_file: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeCliProvenance {
    executable: String,
    version: String,
    workspace_sandbox_supported: bool,
    danger_full_access_supported: bool,
    danger_network_enforcement_supported: bool,
    non_interactive_approval_supported: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeCliSurface {
    provenance: NativeCliProvenance,
    windows_sandbox_setup_supported: bool,
    workspace_launch_flags_supported: bool,
    workspace_launch_project_config_isolated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NativeCliReceipt {
    succeeded: bool,
    exit_code: Option<i32>,
    sandbox_receipt_observed: bool,
}

#[derive(Clone, Debug)]
struct PendingFullAccessCanary {
    attempt_id: String,
    profile_id: String,
    filesystem_identity: String,
    executable: String,
    version: String,
    sentinel_path: PathBuf,
}

trait NativeCliChild: Send {
    fn try_wait(&mut self) -> Result<Option<NativeCliReceipt>, String>;
    fn terminate(&mut self) -> Result<(), String>;
}

trait NativeCliPort: Send + Sync {
    fn run(&self, invocation: &NativeCliInvocation) -> Result<NativeCliReceipt, String>;
    fn start(&self, invocation: &NativeCliInvocation) -> Result<Box<dyn NativeCliChild>, String>;
    fn surface(&self) -> Result<NativeCliSurface, String>;
}

struct SystemNativeCliPort {
    program: Result<String, String>,
}
struct SystemNativeCliChild {
    child: Child,
    sandbox_receipt: Option<PathBuf>,
    sandbox_command_file: Option<PathBuf>,
    stdout_drain: Option<std::thread::JoinHandle<()>>,
    stderr_drain: Option<std::thread::JoinHandle<()>>,
}

impl NativeCliChild for SystemNativeCliChild {
    fn try_wait(&mut self) -> Result<Option<NativeCliReceipt>, String> {
        self.child
            .try_wait()
            .map(|status| status.map(|status| self.settled_receipt(status)))
            .map_err(|error| error.to_string())
    }
    fn terminate(&mut self) -> Result<(), String> {
        self.child
            .kill()
            .map_err(|error| error.to_string())
            .and_then(|_| {
                self.child
                    .wait()
                    .map(|_| {
                        self.release_drains();
                        self.remove_sandbox_command_file();
                    })
                    .map_err(|error| error.to_string())
            })
    }
}

impl SystemNativeCliChild {
    fn settled_receipt(&mut self, status: std::process::ExitStatus) -> NativeCliReceipt {
        self.release_drains();
        let receipt = NativeCliReceipt {
            succeeded: status.success(),
            exit_code: status.code(),
            sandbox_receipt_observed: self.sandbox_receipt.as_ref().is_some_and(|path| {
                fs::read_to_string(path)
                    .map(|value| value.trim() == "native-codex-profile-canary")
                    .unwrap_or(false)
            }),
        };
        self.remove_sandbox_command_file();
        receipt
    }

    fn release_drains(&mut self) {
        // A Windows sandbox helper can retain an inherited stream after the owned outer process
        // has settled. Detach the sink-only drains rather than making durable settlement wait on
        // that helper. The drains retain no output and end when their streams close.
        self.stdout_drain.take();
        self.stderr_drain.take();
    }

    fn remove_sandbox_command_file(&mut self) {
        if let Some(path) = self.sandbox_command_file.take() {
            let _ = fs::remove_file(path);
        }
    }
}

fn discard_native_cli_stream(mut stream: impl Read + Send + 'static) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut stream, &mut std::io::sink());
    })
}

fn spawn_system_native_cli_child(
    program: &str,
    invocation: &NativeCliInvocation,
) -> Result<SystemNativeCliChild, String> {
    let mut child = Command::new(program)
        .args(&invocation.args)
        .current_dir(&invocation.cwd)
        .env_clear()
        .envs(
            invocation
                .environment
                .iter()
                .map(|(key, value)| (key, value)),
        )
        .stdin(Stdio::null())
        // The Windows sandbox forwards its session stdio. Give it valid streams, but drain them
        // immediately so raw CLI output is neither retained nor exposed by this product.
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(SystemNativeCliChild {
        stdout_drain: child.stdout.take().map(discard_native_cli_stream),
        stderr_drain: child.stderr.take().map(discard_native_cli_stream),
        child,
        sandbox_receipt: invocation.sandbox_receipt.clone(),
        sandbox_command_file: invocation.sandbox_command_file.clone(),
    })
}

impl NativeCliPort for SystemNativeCliPort {
    fn run(&self, invocation: &NativeCliInvocation) -> Result<NativeCliReceipt, String> {
        let program = self
            .program
            .as_ref()
            .map_err(|_| "Codex CLI is unavailable for this profile".to_string())?;
        let mut child = spawn_system_native_cli_child(program, invocation)?;
        let status = child.child.wait().map_err(|error| error.to_string())?;
        Ok(child.settled_receipt(status))
    }
    fn start(&self, invocation: &NativeCliInvocation) -> Result<Box<dyn NativeCliChild>, String> {
        let program = self
            .program
            .as_ref()
            .map_err(|_| "Codex CLI is unavailable for this profile".to_string())?;
        Ok(Box::new(spawn_system_native_cli_child(program, invocation)?))
    }
    fn surface(&self) -> Result<NativeCliSurface, String> {
        let program = self
            .program
            .as_ref()
            .map_err(|_| "Codex CLI is unavailable for this profile".to_string())?;
        let version = native_cli_stdout(program, &["--version"])?;
        let exec_help = native_cli_stdout(program, &["exec", "--help"])?;
        let sandbox_help = native_cli_help_output(program, &["sandbox", "--help"])?;
        // The setup parser prints its help to stderr and exits non-zero before its required
        // `--current-user` argument is supplied. Its combined help output is the capability
        // evidence; a successful process exit would incorrectly reject this supported route.
        let sandbox_setup_help =
            native_cli_help_output(program, &["sandbox", "setup", "--help"])?;
        let (workspace_sandbox_supported, windows_sandbox_setup_supported) =
            windows_semantic_sandbox_capabilities(&sandbox_help, &sandbox_setup_help);
        let (workspace_launch_flags_supported, workspace_launch_project_config_isolated) =
            workspace_launch_semantic_capabilities(&exec_help);
        let danger_full_access_supported = exec_help.contains("danger-full-access");
        let danger_network_enforcement_supported = false;
        let non_interactive_approval_supported = exec_help.contains("--dangerously-bypass-approvals-and-sandbox");
        Ok(NativeCliSurface {
            provenance: NativeCliProvenance {
                executable: program.clone(),
                version: version.trim().to_string(),
                workspace_sandbox_supported,
                danger_full_access_supported,
                danger_network_enforcement_supported,
                non_interactive_approval_supported,
            },
            windows_sandbox_setup_supported,
            workspace_launch_flags_supported,
            workspace_launch_project_config_isolated,
        })
    }
}

fn windows_semantic_sandbox_capabilities(
    sandbox_help: &str,
    sandbox_setup_help: &str,
) -> (bool, bool) {
    let workspace_profile = ["--permission-profile", "--cd"]
        .iter()
        .all(|token| sandbox_help.contains(token));
    let elevated_setup = ["--elevated", "--current-user", "--codex-home"]
        .iter()
        .all(|token| sandbox_setup_help.contains(token));
    (workspace_profile, elevated_setup)
}

fn workspace_launch_semantic_capabilities(exec_help: &str) -> (bool, bool) {
    let launch_flags_supported = [
        "--json",
        "--ignore-user-config",
        "--ignore-rules",
        "--config",
        "--sandbox",
        "--cd",
        "--skip-git-repo-check",
        "workspace-write",
    ]
    .iter()
    .all(|token| exec_help.contains(token));
    // The documented switches above only suppress the selected CODEX_HOME config and
    // execpolicy rules. They do not establish that a project .codex/config.toml, hooks, or
    // MCP configuration is excluded, so this product has no safe launch authority yet.
    (launch_flags_supported, false)
}

fn observe_elevated_windows_sandbox_mode(home: &Path) -> Result<bool, String> {
    let file = fs::File::open(home.join("config.toml"))
        .map_err(|_| "The selected profile has no readable external sandbox configuration".to_string())?;
    let mut in_windows = false;
    let mut windows_table_count = 0;
    let mut elevated_assignments = 0;
    let mut non_elevated_assignments = 0;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|_| "The selected profile sandbox configuration cannot be read".to_string())?;
        if line.len() > 1024 {
            return Err("The selected profile sandbox configuration is not safely observable".into());
        }
        let value = line.trim();
        if value.starts_with('[') && value.ends_with(']') {
            in_windows = value == "[windows]";
            if in_windows {
                windows_table_count += 1;
                if windows_table_count > 1 {
                    return Err("The selected profile sandbox configuration is ambiguous".into());
                }
            }
        } else if in_windows && value.starts_with("sandbox") {
            if value == "sandbox = \"elevated\"" {
                elevated_assignments += 1;
            } else {
                non_elevated_assignments += 1;
            }
            if elevated_assignments > 1 || non_elevated_assignments > 0 {
                return Err("The selected profile sandbox configuration is ambiguous".into());
            }
        }
    }
    Ok(windows_table_count == 1 && elevated_assignments == 1)
}

fn native_cli_stdout(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .output()
        .map_err(|_| "Unable to inspect the resolved Codex CLI surface".to_string())?;
    if !output.status.success() {
        return Err("The resolved Codex CLI surface is unsupported".into());
    }
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(text)
}

fn native_cli_help_output(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .output()
        .map_err(|_| "Unable to inspect the resolved Codex CLI surface".to_string())?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(text)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Ownership {
    RegisteredExisting,
    ApplicationDedicated,
}

impl Ownership {
    fn database(self) -> &'static str {
        match self {
            Self::RegisteredExisting => "registered_existing",
            Self::ApplicationDedicated => "application_dedicated",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "registered_existing" => Ok(Self::RegisteredExisting),
            "application_dedicated" => Ok(Self::ApplicationDedicated),
            _ => Err("Stored profile ownership is invalid".into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Lifecycle {
    Active,
    MissingOrMoved,
    Replaced,
    Foreign,
    Malformed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExecutionMode {
    WorkspaceWrite,
    DangerFullAccess,
}

impl ExecutionMode {
    fn database(self) -> &'static str {
        match self {
            Self::WorkspaceWrite => "workspace_write",
            Self::DangerFullAccess => "danger_full_access",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "workspace_write" => Ok(Self::WorkspaceWrite),
            "danger_full_access" => Ok(Self::DangerFullAccess),
            _ => Err("Stored execution mode is invalid".into()),
        }
    }

    fn codex_sandbox(self) -> &'static str {
        match self {
            Self::WorkspaceWrite => "workspace-write",
            Self::DangerFullAccess => "danger-full-access",
        }
    }
}

impl Lifecycle {
    fn database(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::MissingOrMoved => "missing_or_moved",
            Self::Replaced => "replaced",
            Self::Foreign => "foreign",
            Self::Malformed => "malformed",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "active" => Ok(Self::Active),
            "missing_or_moved" => Ok(Self::MissingOrMoved),
            "replaced" => Ok(Self::Replaced),
            "foreign" => Ok(Self::Foreign),
            "malformed" => Ok(Self::Malformed),
            _ => Err("Stored profile lifecycle is invalid".into()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeProfileReadiness {
    authentication: String,
    sandbox_initialization: String,
    workspace_write_canary: String,
    danger_full_access_canary: String,
    mcp_reporting: String,
    attentions: NativeProfileAttentions,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeProfileExecution {
    selected_mode: ExecutionMode,
    danger_full_access_authorized: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeProfileLoginAttempt {
    disposition: String,
    browser_handoff: String,
    requested_at: Option<String>,
    launch_accepted_at: Option<String>,
    settled_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeProfileSetupAttempt {
    phase: String,
    disposition: String,
    executable: Option<String>,
    version: Option<String>,
    workspace_sandbox_supported: Option<bool>,
    correlation_id: Option<String>,
    requested_at: Option<String>,
    launch_accepted_at: Option<String>,
    deadline_at: Option<String>,
    settled_at: Option<String>,
    terminal_classification: String,
    terminal_exit_code: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeProfileSandboxAdoption {
    disposition: String,
    executable: Option<String>,
    version: Option<String>,
    workspace_sandbox_supported: Option<bool>,
    windows_sandbox_setup_supported: Option<bool>,
    correlation_id: Option<String>,
    observed_at: Option<String>,
    elevated_mode_observed: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeProfileSandboxAdoptionConfirmation {
    disposition: String,
    correlation_id: Option<String>,
    confirmed_at: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeProfileAttentions {
    authentication: Option<String>,
    sandbox: Option<String>,
    canary: Option<String>,
    mcp_reporting: Option<String>,
    continuity: Option<String>,
    cli: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeProfileDto {
    id: String,
    home_path: String,
    ownership: Ownership,
    lifecycle: Lifecycle,
    selected: bool,
    execution: NativeProfileExecution,
    login_attempt: NativeProfileLoginAttempt,
    setup_attempt: NativeProfileSetupAttempt,
    sandbox_adoption: NativeProfileSandboxAdoption,
    sandbox_adoption_confirmation: NativeProfileSandboxAdoptionConfirmation,
    readiness: NativeProfileReadiness,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeProfileQueryDto {
    contract: &'static str,
    profiles: Vec<NativeProfileDto>,
}

/// A validated command description, not a launch request, acceptance, provider observation, or
/// workflow outcome. Its working root and network restriction come from a separate application
/// target and are never profile-derived.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeLaunchProjectionDto {
    profile_id: String,
    mode: ExecutionMode,
    executable: String,
    version: String,
    arguments: Vec<String>,
    working_root: String,
    requested_network_disabled: bool,
    effective_network_enforced: bool,
    non_interactive_approval: bool,
    windows_uac_authority: &'static str,
}

/// Obtained from a separate application-owned work assignment. A profile never supplies a root,
/// network setting, role, project configuration, or MCP authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeLaunchTarget {
    working_root: PathBuf,
    network_disabled: bool,
}

impl NativeLaunchTarget {
    pub(crate) fn application_owned(
        working_root: PathBuf,
        network_disabled: bool,
    ) -> Result<Self, String> {
        let canonical = fs::canonicalize(&working_root)
            .map_err(|_| "Application-owned launch root is missing or inaccessible")?;
        if !canonical.is_absolute() || !canonical.is_dir() {
            return Err("Application-owned launch root must be an absolute directory".into());
        }
        Ok(Self {
            working_root: canonical,
            network_disabled,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeFullAccessCanaryProjectionDto {
    launch: NativeLaunchProjectionDto,
    sentinel_path: String,
    evidence_state: &'static str,
}

/// NCHP-03 supplies this only after its bounded, application-owned MCP action receives a
/// correlated receipt. It is deliberately not inferred from `codex mcp list` or a file write.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeMcpReportingReceipt {
    pub(crate) capability: String,
    pub(crate) server: String,
    pub(crate) tool: String,
    pub(crate) correlation_id: String,
    pub(crate) probe_root: PathBuf,
}

/// Private application authority for NCHP-03. This never appears in settings DTOs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeMcpReportingProbeAuthority {
    pub(crate) profile_id: String,
    pub(crate) correlation_id: String,
    pub(crate) capability: String,
    pub(crate) server: String,
    pub(crate) tool: String,
    pub(crate) probe_root: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SetupPhase {
    SandboxInitialization,
    WorkspaceWriteCanary,
}

impl SetupPhase {
    fn database(self) -> &'static str {
        match self {
            Self::SandboxInitialization => "sandbox_initialization",
            Self::WorkspaceWriteCanary => "workspace_write_canary",
        }
    }

    fn from_database(value: &str) -> Result<Self, String> {
        match value {
            "sandbox_initialization" => Ok(Self::SandboxInitialization),
            "workspace_write_canary" => Ok(Self::WorkspaceWriteCanary),
            _ => Err("Stored native profile setup phase is invalid".into()),
        }
    }

    fn attention_concern(self) -> &'static str {
        match self {
            Self::SandboxInitialization => "sandbox",
            Self::WorkspaceWriteCanary => "canary",
        }
    }
}

#[derive(Clone, Debug)]
struct PendingSetupAttempt {
    attempt_id: String,
    profile_id: String,
    filesystem_identity: String,
    phase: SetupPhase,
    deadline_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
struct PendingLoginAttempt {
    attempt_id: String,
    profile_id: String,
    filesystem_identity: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredProfile {
    id: String,
    home: PathBuf,
    identity: String,
    ownership: Ownership,
    lifecycle: Lifecycle,
    selected: bool,
    execution: NativeProfileExecution,
    login_attempt: NativeProfileLoginAttempt,
    setup_attempt: NativeProfileSetupAttempt,
    sandbox_adoption: NativeProfileSandboxAdoption,
    sandbox_adoption_confirmation: NativeProfileSandboxAdoptionConfirmation,
    readiness: NativeProfileReadiness,
}

impl From<StoredProfile> for NativeProfileDto {
    fn from(value: StoredProfile) -> Self {
        Self {
            id: value.id,
            home_path: value.home.to_string_lossy().into_owned(),
            ownership: value.ownership,
            lifecycle: value.lifecycle,
            selected: value.selected,
            execution: value.execution,
            login_attempt: value.login_attempt,
            setup_attempt: value.setup_attempt,
            sandbox_adoption: value.sandbox_adoption,
            sandbox_adoption_confirmation: value.sandbox_adoption_confirmation,
            readiness: value.readiness,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DedicatedMarker<'a> {
    contract: &'static str,
    profile_id: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadDedicatedMarker {
    contract: String,
    profile_id: String,
}

pub(crate) struct NativeProfileService {
    database_path: PathBuf,
    dedicated_root: PathBuf,
    cli: Arc<dyn NativeCliPort>,
    login_children: Mutex<HashMap<String, Box<dyn NativeCliChild>>>,
    setup_children: Mutex<HashMap<String, Box<dyn NativeCliChild>>>,
    full_access_canary_children: Mutex<HashMap<String, Box<dyn NativeCliChild>>>,
    operation_gate: Mutex<()>,
}

impl NativeProfileService {
    pub(crate) fn open(database_path: PathBuf, app_data_dir: PathBuf) -> Result<Self, String> {
        let _gate = NATIVE_PROFILE_OPEN_GATE
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| "Native profile initialization supervision is unavailable")?;
        let connection = crate::storage::open_active_database(&database_path)?;
        connection
            .execute_batch(NATIVE_PROFILE_SCHEMA)
            .map_err(|error| format!("Unable to initialize native profile schema: {error}"))?;
        Ok(Self {
            database_path,
            dedicated_root: app_data_dir.join("native-codex-homes"),
            cli: Arc::new(SystemNativeCliPort {
                program: crate::runtime::codex::resolve_program("codex".into()),
            }),
            login_children: Mutex::new(HashMap::new()),
            setup_children: Mutex::new(HashMap::new()),
            full_access_canary_children: Mutex::new(HashMap::new()),
            operation_gate: Mutex::new(()),
        })
    }

    fn connection(&self) -> Result<Connection, String> {
        crate::storage::open_active_database(&self.database_path)
    }

    pub(crate) fn query(&self) -> Result<NativeProfileQueryDto, String> {
        let mut connection = self.connection()?;
        for profile in load_profiles(&mut connection)? {
            self.revalidate(&profile)?;
            self.reconcile_sandbox_adoption(&profile.id)?;
            self.reconcile_login_attempt(&profile.id)?;
            self.reconcile_setup_attempts(&profile.id)?;
            self.reconcile_full_access_canary(&profile.id)?;
            self.expire_mcp_probe(&profile.id)?;
        }
        let profiles = load_profiles(&mut connection)?;
        Ok(NativeProfileQueryDto {
            contract: PROFILE_QUERY_CONTRACT,
            profiles: profiles.into_iter().map(Into::into).collect(),
        })
    }

    pub(crate) fn register_existing(
        &self,
        supplied_home: &str,
    ) -> Result<NativeProfileDto, String> {
        let home = validated_absolute_directory(supplied_home)?;
        if home.join(MARKER_FILE).exists() {
            return Err(
                "An application-owned Codex home cannot be registered as user-owned".into(),
            );
        }
        self.insert_profile(home, Ownership::RegisteredExisting)
    }

    pub(crate) fn create_dedicated(&self) -> Result<NativeProfileDto, String> {
        fs::create_dir_all(&self.dedicated_root)
            .map_err(|error| format!("Unable to create dedicated profile root: {error}"))?;
        let id = format!("native-profile-{}", Uuid::new_v4());
        let candidate = self.dedicated_root.join(&id);
        fs::create_dir(&candidate)
            .map_err(|error| format!("Unable to create dedicated Codex home: {error}"))?;
        let home = fs::canonicalize(&candidate)
            .map_err(|error| format!("Unable to canonicalize dedicated Codex home: {error}"))?;
        if let Err(error) = write_marker(&home, &id) {
            let _ = fs::remove_dir_all(&candidate);
            return Err(error);
        }
        match self.insert_profile_with_id(id, home, Ownership::ApplicationDedicated) {
            Ok(profile) => Ok(profile),
            Err(error) => {
                let _ = fs::remove_dir_all(&candidate);
                Err(error)
            }
        }
    }

    fn insert_profile(
        &self,
        home: PathBuf,
        ownership: Ownership,
    ) -> Result<NativeProfileDto, String> {
        if let Some(existing) = self.profile_by_home(&home)? {
            return Ok(existing.into());
        }
        self.insert_profile_with_id(
            format!("native-profile-{}", Uuid::new_v4()),
            home,
            ownership,
        )
    }

    fn insert_profile_with_id(
        &self,
        id: String,
        home: PathBuf,
        ownership: Ownership,
    ) -> Result<NativeProfileDto, String> {
        let identity = filesystem_identity(&home)?;
        let now = Utc::now().to_rfc3339();
        let connection = self.connection()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| format!("Unable to begin profile registration: {error}"))?;
        let inserted = transaction
            .execute(
                "INSERT OR IGNORE INTO native_codex_profiles (id,canonical_home_path,filesystem_identity,ownership,lifecycle,created_at,updated_at) VALUES (?1,?2,?3,?4,'active',?5,?5)",
                params![id, home.to_string_lossy(), identity, ownership.database(), now],
            )
            .map_err(|error| format!("Unable to register Codex home: {error}"))?;
        if inserted == 0 {
            transaction.commit().map_err(|error| error.to_string())?;
            return self
                .profile_by_home(&home)?
                .map(Into::into)
                .ok_or_else(|| "Concurrent profile registration did not produce a profile".into());
        }
        transaction
            .execute(
                "INSERT INTO native_codex_profile_readiness (profile_id,authentication,sandbox_initialization,workspace_write_canary,mcp_reporting,observed_at) VALUES (?1,'unknown','unknown','not_run','not_assessed',?2)",
                params![id, now],
            )
            .map_err(|error| format!("Unable to initialize profile readiness: {error}"))?;
        transaction
            .execute(
                "INSERT INTO native_codex_profile_execution_modes (profile_id,selected_mode,updated_at) VALUES (?1,'workspace_write',?2)",
                params![id, now],
            )
            .map_err(|error| format!("Unable to initialize profile execution mode: {error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("Unable to commit profile registration: {error}"))?;
        self.profile(&id).map(Into::into)
    }

    pub(crate) fn select(&self, id: &str) -> Result<NativeProfileDto, String> {
        let _gate = self
            .operation_gate
            .lock()
            .map_err(|_| "Native profile operation supervision is unavailable")?;
        let profile = self.profile(id)?;
        if profile.lifecycle != Lifecycle::Active {
            return Err("Native Codex home lost continuity and must be registered again".into());
        }
        let lifecycle = validate_profile(&profile);
        if lifecycle != Lifecycle::Active {
            self.record_lifecycle_while_gated(id, lifecycle)?;
            return Err("Only a currently validated native profile can be selected".into());
        }
        let now = Utc::now().to_rfc3339();
        let mut connection = self.connection()?;
        let profiles = load_profiles(&mut connection)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute("UPDATE native_codex_profiles SET selected_at=NULL", [])
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE native_codex_profiles SET selected_at=?2,updated_at=?2 WHERE id=?1",
                params![id, now],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        for unselected in profiles.into_iter().filter(|candidate| candidate.id != id) {
            if unselected.selected {
                self.invalidate_sandbox_adoption(&unselected.id)?;
            }
            self.reconcile_setup_attempts(&unselected.id)?;
        }
        self.profile(id).map(Into::into)
    }

    pub(crate) fn select_execution_mode(
        &self,
        id: &str,
        mode: ExecutionMode,
    ) -> Result<NativeProfileDto, String> {
        self.require_active(id)?;
        self.connection()?
            .execute(
                "UPDATE native_codex_profile_execution_modes SET selected_mode=?2,updated_at=?3 WHERE profile_id=?1",
                params![id, mode.database(), Utc::now().to_rfc3339()],
            )
            .map_err(|error| format!("Unable to select native execution mode: {error}"))?;
        self.profile(id).map(Into::into)
    }

    /// This is the only durable opt-in for the dangerous mode. Authentication, readiness,
    /// ownership, and an earlier launch never create it.
    pub(crate) fn authorize_danger_full_access(
        &self,
        id: &str,
    ) -> Result<NativeProfileDto, String> {
        let profile = self.require_active(id)?;
        if profile.execution.selected_mode != ExecutionMode::DangerFullAccess {
            return Err("Danger full access must be selected before it can be authorized".into());
        }
        self.connection()?
            .execute(
                "INSERT INTO native_codex_profile_mode_authorizations (profile_id,mode,filesystem_identity,authorized_at,revoked_at) VALUES (?1,'danger_full_access',?2,?3,NULL) ON CONFLICT(profile_id,mode) DO UPDATE SET filesystem_identity=excluded.filesystem_identity,authorized_at=excluded.authorized_at,revoked_at=NULL",
                params![id, profile.identity, Utc::now().to_rfc3339()],
            )
            .map_err(|error| format!("Unable to record danger full access authorization: {error}"))?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn revoke_danger_full_access(&self, id: &str) -> Result<NativeProfileDto, String> {
        self.require_active(id)?;
        self.connection()?
            .execute(
                "UPDATE native_codex_profile_mode_authorizations SET revoked_at=?2 WHERE profile_id=?1 AND mode='danger_full_access' AND revoked_at IS NULL",
                params![id, Utc::now().to_rfc3339()],
            )
            .map_err(|error| format!("Unable to revoke danger full access authorization: {error}"))?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn request_login(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_selected_active(id)?;
        self.reconcile_login_attempt(id)?;
        if load_pending_login_attempt(&self.connection()?, id)?.is_some() {
            return self.profile(id).map(Into::into);
        }
        let root = self.ensure_probe_root(id)?;
        let surface = self.cli.surface().map_err(|_| {
            self.set_attention(id, "cli", Some("codex_cli_unavailable"), false)
                .ok();
            "Unable to resolve supported Codex CLI for browser login".to_string()
        })?;
        let now = Utc::now();
        let attempt = PendingLoginAttempt {
            attempt_id: format!("native-login-attempt-{}", Uuid::new_v4()),
            profile_id: profile.id.clone(),
            filesystem_identity: profile.identity.clone(),
        };
        let inserted = self.connection()?.execute(
            "INSERT OR IGNORE INTO native_codex_profile_login_attempts (attempt_id,profile_id,filesystem_identity,executable,version,correlation_id,state,browser_handoff,requested_at) VALUES (?1,?2,?3,?4,?5,?6,'pending','unobserved',?7)",
            params![attempt.attempt_id, attempt.profile_id, attempt.filesystem_identity, surface.provenance.executable, surface.provenance.version, format!("native-login-{}", Uuid::new_v4()), now.to_rfc3339()],
        ).map_err(|error| format!("Unable to persist native browser login attempt: {error}"))?;
        if inserted == 0 {
            return self.profile(id).map(Into::into);
        }
        let mut child = match self.cli.start(&NativeCliInvocation {
            args: vec!["login".into()],
            cwd: root,
            codex_home: profile.home.clone(),
            environment: native_windows_cli_environment(&profile.home),
            sandbox_receipt: None,
            sandbox_command_file: None,
        }) {
            Ok(child) => child,
            Err(_) => {
                self.set_login_attempt_state(&attempt.attempt_id, "launch_failed")?;
                self.set_attention(id, "cli", Some("codex_cli_unavailable"), false)?;
                self.set_attention(
                    id,
                    "authentication",
                    Some("browser_login_launch_failed"),
                    false,
                )?;
                return self.profile(id).map(Into::into);
            }
        };
        match self.login_children.lock() {
            Ok(mut children) => {
                children.insert(id.to_string(), child);
            }
            Err(_) => {
                let _ = child.terminate();
                self.set_login_attempt_state(&attempt.attempt_id, "cancelled")?;
                return Err("Native profile login supervision is unavailable".into());
            }
        }
        self.connection()?.execute(
            "UPDATE native_codex_profile_login_attempts SET launch_accepted_at=?2 WHERE attempt_id=?1 AND state='pending'",
            params![attempt.attempt_id, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        self.set_attention(
            id,
            "authentication",
            Some("browser_login_attempt_pending"),
            false,
        )?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn refresh_readiness(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_selected_active(id)?;
        self.reconcile_login_attempt(id)?;
        let root = self.ensure_probe_root(id)?;
        let authenticated = self
            .cli
            .run(&NativeCliInvocation {
                args: vec!["login".into(), "status".into()],
                cwd: root,
                codex_home: profile.home.clone(),
                environment: native_windows_cli_environment(&profile.home),
                sandbox_receipt: None,
                sandbox_command_file: None,
            })
            .map_err(|_| {
                self.set_attention(id, "cli", Some("codex_cli_unavailable"), false)
                    .ok();
                "Codex CLI is unavailable for this profile".to_string()
            })?
            .succeeded;
        self.set_attention(id, "cli", None, false)?;
        self.update_readiness(
            id,
            Some(if authenticated {
                "authenticated"
            } else {
                "unauthenticated"
            }),
            None,
            None,
            None,
            None,
        )?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn request_sandbox_initialization(
        &self,
        id: &str,
    ) -> Result<NativeProfileDto, String> {
        self.require_selected_active(id)?;
        self.reconcile_setup_attempts(id)?;
        let profile = self.require_selected_active(id)?;
        self.start_setup_attempt(&profile, SetupPhase::SandboxInitialization)?;
        self.profile(id).map(Into::into)
    }

    /// A person must explicitly confirm the Windows/UAC stage. A successful setup process only
    /// records that the application-owned request completed; it never establishes this fact.
    pub(crate) fn confirm_sandbox_initialization(
        &self,
        id: &str,
    ) -> Result<NativeProfileDto, String> {
        self.require_selected_active(id)?;
        self.reconcile_setup_attempts(id)?;
        self.require_selected_active(id)?;
        let connection = self.connection()?;
        let latest_state = connection
            .query_row(
                "SELECT state FROM native_codex_profile_setup_attempts WHERE profile_id=?1 AND phase='sandbox_initialization' ORDER BY requested_at DESC,attempt_id DESC LIMIT 1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if latest_state.as_deref() != Some("terminal_succeeded") {
            return Err("A completed application-owned sandbox setup request is required before confirmation".into());
        }
        self.update_readiness(
            id,
            None,
            Some("initialized"),
            None,
            None,
            Some(("sandbox", None)),
        )?;
        self.profile(id).map(Into::into)
    }

    /// Records an observed external Windows sandbox postcondition. It never creates or rewrites
    /// that configuration and never stands in for a product-owned setup request or UAC event.
    pub(crate) fn verify_preprovisioned_sandbox(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_selected_active(id)?;
        let surface = self.cli.surface().map_err(|_| {
            self.set_attention(id, "cli", Some("codex_cli_surface_unsupported"), false).ok();
            "The resolved Codex CLI surface is unsupported for external sandbox verification".to_string()
        })?;
        let elevated_mode_observed = observe_elevated_windows_sandbox_mode(&profile.home)?;
        let verified = surface.provenance.workspace_sandbox_supported
            && surface.windows_sandbox_setup_supported
            && elevated_mode_observed;
        self.connection()?.execute(
            "INSERT INTO native_codex_profile_sandbox_adoptions (profile_id,filesystem_identity,executable,version,workspace_sandbox_supported,windows_sandbox_setup_supported,correlation_id,observed_at,state,elevated_mode_observed) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(profile_id) DO UPDATE SET filesystem_identity=excluded.filesystem_identity,executable=excluded.executable,version=excluded.version,workspace_sandbox_supported=excluded.workspace_sandbox_supported,windows_sandbox_setup_supported=excluded.windows_sandbox_setup_supported,correlation_id=excluded.correlation_id,observed_at=excluded.observed_at,state=excluded.state,elevated_mode_observed=excluded.elevated_mode_observed",
            params![id, profile.identity, surface.provenance.executable, surface.provenance.version, surface.provenance.workspace_sandbox_supported as i64, surface.windows_sandbox_setup_supported as i64, format!("native-sandbox-adoption-{}", Uuid::new_v4()), Utc::now().to_rfc3339(), if verified { "verified" } else { "not_verified" }, elevated_mode_observed as i64],
        ).map_err(|error| format!("Unable to persist external sandbox verification: {error}"))?;
        self.connection()?.execute(
            "UPDATE native_codex_profile_sandbox_adoption_confirmations SET state='invalidated',invalidated_at=COALESCE(invalidated_at,?2) WHERE profile_id=?1 AND state='confirmed'",
            params![id, Utc::now().to_rfc3339()],
        ).map_err(|error| format!("Unable to invalidate superseded external sandbox adoption confirmation: {error}"))?;
        self.update_readiness(
            id,
            None,
            Some("attention_required"),
            Some("blocked"),
            None,
            Some(("sandbox", Some(if verified { "external_sandbox_provisioning_verified_explicit_adoption_confirmation_required" } else { "external_sandbox_provisioning_not_verified" }))),
        )?;
        self.profile(id).map(Into::into)
    }

    /// This is an explicit product acknowledgment of a verified external postcondition. It does
    /// not claim that this product requested setup or observed a UAC interaction.
    pub(crate) fn confirm_preprovisioned_sandbox_adoption(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_selected_active(id)?;
        let adoption = load_sandbox_adoption(&self.connection()?, id, &profile.identity)?;
        let surface = self.cli.surface().map_err(|_| "The resolved Codex CLI surface is unsupported for external sandbox adoption".to_string())?;
        let still_observed = observe_elevated_windows_sandbox_mode(&profile.home)?;
        let valid = adoption.disposition == "verified"
            && adoption.executable.as_deref() == Some(surface.provenance.executable.as_str())
            && adoption.version.as_deref() == Some(surface.provenance.version.as_str())
            && adoption.workspace_sandbox_supported == Some(true)
            && adoption.windows_sandbox_setup_supported == Some(true)
            && surface.provenance.workspace_sandbox_supported
            && surface.windows_sandbox_setup_supported
            && still_observed;
        if !valid {
            self.invalidate_sandbox_adoption(id)?;
            return Err("The externally provisioned sandbox evidence no longer matches this selected profile".into());
        }
        let adoption_correlation = adoption.correlation_id.ok_or("The external sandbox observation has no durable correlation")?;
        let now = Utc::now().to_rfc3339();
        self.connection()?.execute(
            "INSERT INTO native_codex_profile_sandbox_adoption_confirmations (profile_id,filesystem_identity,adoption_correlation_id,confirmation_correlation_id,confirmed_at,state,invalidated_at) VALUES (?1,?2,?3,?4,?5,'confirmed',NULL) ON CONFLICT(profile_id) DO UPDATE SET filesystem_identity=excluded.filesystem_identity,adoption_correlation_id=excluded.adoption_correlation_id,confirmation_correlation_id=excluded.confirmation_correlation_id,confirmed_at=excluded.confirmed_at,state='confirmed',invalidated_at=NULL",
            params![id, profile.identity, adoption_correlation, format!("native-sandbox-adoption-confirmation-{}", Uuid::new_v4()), now],
        ).map_err(|error| format!("Unable to persist external sandbox adoption confirmation: {error}"))?;
        self.update_readiness(
            id,
            None,
            Some("initialized"),
            None,
            None,
            Some(("sandbox", Some("external_sandbox_adoption_confirmed_product_uac_unobserved"))),
        )?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn run_workspace_write_canary(&self, id: &str) -> Result<NativeProfileDto, String> {
        self.require_selected_active(id)?;
        self.reconcile_sandbox_adoption(id)?;
        self.reconcile_setup_attempts(id)?;
        let profile = self.require_selected_active(id)?;
        if self.profile(id)?.readiness.sandbox_initialization != "initialized" {
            self.update_readiness(
                id,
                None,
                None,
                Some("blocked"),
                None,
                Some((
                    "canary",
                    Some("workspace_write_canary_requires_observed_sandbox_initialization"),
                )),
            )?;
            return self.profile(id).map(Into::into);
        }
        self.start_setup_attempt(&profile, SetupPhase::WorkspaceWriteCanary)?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn probe_mcp_reporting(&self, id: &str) -> Result<NativeProfileDto, String> {
        self.begin_mcp_reporting_probe(id)?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn begin_mcp_reporting_probe(
        &self,
        id: &str,
    ) -> Result<NativeMcpReportingProbeAuthority, String> {
        self.require_active(id)?;
        let root = self.probe_root(id);
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("Unable to begin native MCP reporting probe: {error}"))?;
        let now = Utc::now();
        transaction.execute(
            "UPDATE native_codex_profile_mcp_probes SET state='expired' WHERE profile_id=?1 AND state='pending' AND deadline_at <= ?2",
            params![id, now.to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        if let Some(authority) = load_pending_mcp_probe(&transaction, id)? {
            transaction.commit().map_err(|error| error.to_string())?;
            return Ok(authority);
        }
        let authority = NativeMcpReportingProbeAuthority {
            profile_id: id.into(),
            correlation_id: format!("native-mcp-probe-{}", Uuid::new_v4()),
            capability: MCP_REPORTING_CAPABILITY.into(),
            server: MCP_REPORTING_SERVER.into(),
            tool: MCP_REPORTING_TOOL.into(),
            probe_root: root,
        };
        transaction.execute(
            "INSERT INTO native_codex_profile_mcp_probes (request_id,profile_id,correlation_id,expected_capability,expected_server,expected_tool,expected_probe_root,state,requested_at,deadline_at) VALUES (?1,?2,?3,?4,?5,?6,?7,'pending',?8,?9)",
            params![format!("native-mcp-request-{}", Uuid::new_v4()), authority.profile_id, authority.correlation_id, authority.capability, authority.server, authority.tool, authority.probe_root.to_string_lossy(), now.to_rfc3339(), (now + Duration::seconds(MCP_PROBE_TIMEOUT_SECONDS)).to_rfc3339()],
        ).map_err(|error| format!("Unable to persist native MCP reporting probe: {error}"))?;
        transaction.execute(
            "UPDATE native_codex_profile_readiness SET mcp_reporting='not_assessed',observed_at=?2 WHERE profile_id=?1",
            params![id, now.to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        self.write_attention(
            &transaction,
            id,
            "mcp_reporting",
            Some("mcp_reporting_probe_pending_application_receipt"),
        )?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(authority)
    }

    pub(crate) fn record_mcp_reporting_receipt(
        &self,
        id: &str,
        receipt: &NativeMcpReportingReceipt,
    ) -> Result<NativeProfileDto, String> {
        self.require_active(id)?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("Unable to begin native MCP receipt settlement: {error}"))?;
        let now = Utc::now();
        let expired = transaction.execute(
            "UPDATE native_codex_profile_mcp_probes SET state='expired' WHERE profile_id=?1 AND state='pending' AND deadline_at <= ?2",
            params![id, now.to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        if expired != 0 {
            transaction.execute(
                "UPDATE native_codex_profile_readiness SET mcp_reporting='not_assessed',observed_at=?2 WHERE profile_id=?1",
                params![id, now.to_rfc3339()],
            ).map_err(|error| error.to_string())?;
            self.write_attention(
                &transaction,
                id,
                "mcp_reporting",
                Some("mcp_reporting_probe_expired"),
            )?;
            transaction.commit().map_err(|error| error.to_string())?;
            return Err("The application-owned MCP reporting probe has expired".into());
        }
        let transitioned = transaction.execute(
            "UPDATE native_codex_profile_mcp_probes SET state='received',received_at=?2 WHERE profile_id=?1 AND state='pending' AND correlation_id=?3 AND expected_capability=?4 AND expected_server=?5 AND expected_tool=?6 AND expected_probe_root=?7 AND deadline_at > ?2",
            params![id, now.to_rfc3339(), receipt.correlation_id, receipt.capability, receipt.server, receipt.tool, receipt.probe_root.to_string_lossy()],
        ).map_err(|error| error.to_string())?;
        if transitioned != 1 {
            return Err(
                "MCP reporting receipt does not match one current application-owned pending probe"
                    .into(),
            );
        }
        transaction.execute(
            "UPDATE native_codex_profile_readiness SET mcp_reporting='ready',observed_at=?2 WHERE profile_id=?1",
            params![id, now.to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        self.write_attention(&transaction, id, "mcp_reporting", None)?;
        transaction.commit().map_err(|error| error.to_string())?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn resolve_selected_home(&self) -> Result<ResolvedNativeCodexHome, String> {
        let mut connection = self.connection()?;
        let profile = load_profiles(&mut connection)?
            .into_iter()
            .find(|profile| profile.selected)
            .ok_or("No native Codex home is selected")?;
        self.reconcile_sandbox_adoption(&profile.id)?;
        let profile = self.require_selected_active(&profile.id)?;
        if profile.lifecycle != Lifecycle::Active {
            return Err(
                "The selected native Codex home lost continuity and must be registered again"
                    .into(),
            );
        }
        let lifecycle = validate_profile(&profile);
        if lifecycle != Lifecycle::Active {
            self.record_lifecycle(&profile.id, lifecycle)?;
            return Err("The selected native Codex home no longer has validated continuity".into());
        }
        let readiness = &profile.readiness;
        if readiness.authentication != "authenticated"
            || readiness.sandbox_initialization != "initialized"
            || readiness.workspace_write_canary != "passed"
            || readiness.mcp_reporting != "ready"
        {
            return Err(
                "The selected native Codex home is not ready for an application consumer".into(),
            );
        }
        Ok(ResolvedNativeCodexHome {
            home: profile.home,
            readiness: readiness.clone(),
        })
    }

    /// Produces a command only after all mode-specific authority is independently valid. It does
    /// not start Codex and consequently cannot be mistaken for launch acceptance or activity.
    pub(crate) fn project_launch(
        &self,
        id: &str,
        target: &NativeLaunchTarget,
    ) -> Result<NativeLaunchProjectionDto, String> {
        let profile = self.require_active(id)?;
        if !profile.selected {
            return Err(
                "Only the currently selected native profile can receive a launch projection".into(),
            );
        }
        if !target.network_disabled {
            return Err(
                "Native execution requires an application-owned target with network disabled"
                    .into(),
            );
        }
        let surface = self.cli.surface().map_err(|_| {
            self.set_attention(id, "cli", Some("codex_cli_surface_unsupported"), false)
                .ok();
            "The resolved Codex CLI surface is unsupported for native execution".to_string()
        })?;
        let mode = profile.execution.selected_mode;
        match mode {
            ExecutionMode::WorkspaceWrite => {
                if !surface.workspace_launch_flags_supported {
                    self.set_attention(
                        id,
                        "cli",
                        Some("codex_cli_workspace_launch_surface_unsupported"),
                        false,
                    )?;
                    return Err(
                        "The resolved Codex CLI is missing the required workspace-write launch surface".into(),
                    );
                }
                if !surface.workspace_launch_project_config_isolated {
                    self.set_attention(
                        id,
                        "cli",
                        Some("codex_cli_workspace_launch_project_config_unsupported"),
                        false,
                    )?;
                    return Err(
                        "The resolved Codex CLI cannot exclude project configuration from a workspace-write launch".into(),
                    );
                }
                if !surface.provenance.workspace_sandbox_supported
                    || profile.readiness.sandbox_initialization != "initialized"
                    || profile.readiness.workspace_write_canary != "passed"
                {
                    return Err(
                        "Workspace-write launch authority is not currently established".into(),
                    );
                }
            }
            ExecutionMode::DangerFullAccess => {
                if !surface.provenance.danger_full_access_supported
                    || !surface.provenance.non_interactive_approval_supported
                    || !profile.execution.danger_full_access_authorized
                {
                    return Err(
                        "Danger full access launch authority is not currently established".into(),
                    );
                }
                if !surface.provenance.danger_network_enforcement_supported {
                    self.set_attention(
                        id,
                        "cli",
                        Some("codex_cli_danger_network_enforcement_unsupported"),
                        false,
                    )?;
                    return Err(
                        "The resolved Codex CLI cannot enforce the application-required network policy for danger full access".into(),
                    );
                }
            }
        }
        Ok(NativeLaunchProjectionDto {
            profile_id: profile.id,
            mode,
            executable: surface.provenance.executable,
            version: surface.provenance.version,
            arguments: vec![
                "exec".into(),
                "--json".into(),
                "--sandbox".into(),
                mode.codex_sandbox().into(),
                "--cd".into(),
                target.working_root.to_string_lossy().into_owned(),
                "--skip-git-repo-check".into(),
            ]
            .into_iter()
            .chain((mode == ExecutionMode::WorkspaceWrite).then_some("--ignore-user-config".into()))
            .chain((mode == ExecutionMode::WorkspaceWrite).then_some("--ignore-rules".into()))
            .chain((mode == ExecutionMode::WorkspaceWrite).then_some("--config".into()))
            .chain((mode == ExecutionMode::WorkspaceWrite).then_some("sandbox_workspace_write.network_access=false".into()))
            .chain((mode == ExecutionMode::WorkspaceWrite).then_some("--config".into()))
            .chain((mode == ExecutionMode::WorkspaceWrite).then_some("sandbox_workspace_write.writable_roots=[]".into()))
            .chain((mode == ExecutionMode::WorkspaceWrite).then_some("--config".into()))
            .chain((mode == ExecutionMode::WorkspaceWrite).then_some("sandbox_workspace_write.exclude_tmpdir_env_var=true".into()))
            .chain((mode == ExecutionMode::WorkspaceWrite).then_some("--config".into()))
            .chain((mode == ExecutionMode::WorkspaceWrite).then_some("sandbox_workspace_write.exclude_slash_tmp=true".into()))
            .chain((mode == ExecutionMode::DangerFullAccess).then_some("--dangerously-bypass-approvals-and-sandbox".into()))
            .collect(),
            working_root: target.working_root.to_string_lossy().into_owned(),
            requested_network_disabled: target.network_disabled,
            effective_network_enforced: true,
            non_interactive_approval: mode == ExecutionMode::DangerFullAccess,
            windows_uac_authority: "not_granted",
        })
    }

    pub(crate) fn project_full_access_canary(
        &self,
        id: &str,
    ) -> Result<NativeFullAccessCanaryProjectionDto, String> {
        let target =
            NativeLaunchTarget::application_owned(self.full_access_canary_root(id)?, true)?;
        let launch = self.project_launch(id, &target)?;
        if launch.mode != ExecutionMode::DangerFullAccess {
            return Err(
                "The full-access canary requires the selected danger full access mode".into(),
            );
        }
        let sentinel_path = self
            .full_access_canary_root(id)?
            .join("native-full-access-canary.txt");
        Ok(NativeFullAccessCanaryProjectionDto {
            launch,
            sentinel_path: sentinel_path.to_string_lossy().into_owned(),
            evidence_state: "not_run",
        })
    }

    /// Starts only the bounded, application-owned canary. Provider activity and semantic workflow
    /// completion remain outside this receipt; only the exact sentinel settles the canary fact.
    pub(crate) fn run_danger_full_access_canary(
        &self,
        id: &str,
    ) -> Result<NativeProfileDto, String> {
        self.reconcile_full_access_canary(id)?;
        if load_pending_full_access_canary(&self.connection()?, id)?.is_some() {
            return self.profile(id).map(Into::into);
        }
        let profile = self.require_active(id)?;
        let projection = self.project_full_access_canary(id)?;
        let sentinel = PathBuf::from(&projection.sentinel_path);
        let prompt = format!(
            "Create only the application-owned sentinel file at {} with the exact contents native-codex-profile-full-access-canary, then stop.",
            sentinel.display()
        );
        let attempt = PendingFullAccessCanary {
            attempt_id: format!("native-full-access-canary-{}", Uuid::new_v4()),
            profile_id: profile.id.clone(),
            filesystem_identity: profile.identity,
            executable: projection.launch.executable.clone(),
            version: projection.launch.version.clone(),
            sentinel_path: sentinel.clone(),
        };
        self.connection()?.execute(
            "INSERT INTO native_codex_profile_full_access_canaries (attempt_id,profile_id,filesystem_identity,mode,executable,version,sentinel_path,state,started_at) VALUES (?1,?2,?3,'danger_full_access',?4,?5,?6,'pending',?7)",
            params![attempt.attempt_id, attempt.profile_id, attempt.filesystem_identity, attempt.executable, attempt.version, attempt.sentinel_path.to_string_lossy(), Utc::now().to_rfc3339()],
        ).map_err(|error| format!("Unable to persist full-access canary request: {error}"))?;
        let mut args = projection.launch.arguments;
        args.push(prompt);
        let invocation = NativeCliInvocation {
            args,
            cwd: PathBuf::from(&projection.launch.working_root),
            codex_home: profile.home.clone(),
            environment: native_profile_environment(&profile.home),
            sandbox_receipt: Some(sentinel),
            sandbox_command_file: None,
        };
        match self.cli.start(&invocation) {
            Ok(mut child) => match self.full_access_canary_children.lock() {
                Ok(mut children) => {
                    children.insert(attempt.attempt_id, child);
                    self.profile(id).map(Into::into)
                }
                Err(_) => {
                    let _ = child.terminate();
                    self.set_full_access_canary_state(id, "blocked")?;
                    Err("Native full-access canary supervision is unavailable".into())
                }
            },
            Err(_) => {
                self.set_full_access_canary_state(id, "blocked")?;
                Err("Unable to start the supported full-access canary".into())
            }
        }
    }

    fn reconcile_full_access_canary(&self, id: &str) -> Result<(), String> {
        let Some(attempt) = load_pending_full_access_canary(&self.connection()?, id)? else {
            return Ok(());
        };
        let current = self.require_active(id);
        let surface = self.cli.surface();
        let authority_valid = current.as_ref().is_ok_and(|profile| {
            profile.selected
                && profile.execution.selected_mode == ExecutionMode::DangerFullAccess
                && profile.execution.danger_full_access_authorized
                && profile.identity == attempt.filesystem_identity
        }) && surface.as_ref().is_ok_and(|surface| {
            surface.provenance.danger_full_access_supported
                && surface.provenance.non_interactive_approval_supported
                && surface.provenance.executable == attempt.executable
                && surface.provenance.version == attempt.version
        });
        if !authority_valid {
            self.set_full_access_canary_state(id, "blocked")?;
            return Ok(());
        }
        let outcome = self
            .full_access_canary_children
            .lock()
            .map_err(|_| "Native full-access canary supervision is unavailable")?
            .get_mut(&attempt.attempt_id)
            .map(|child| child.try_wait())
            .transpose()?
            .flatten();
        match outcome {
            Some(receipt) if receipt.succeeded && receipt.sandbox_receipt_observed => {
                self.full_access_canary_children.lock().ok().and_then(|mut children| children.remove(&attempt.attempt_id));
                self.set_full_access_canary_state(id, "passed")?;
            }
            Some(_) => {
                self.full_access_canary_children.lock().ok().and_then(|mut children| children.remove(&attempt.attempt_id));
                self.set_full_access_canary_state(id, "blocked")?;
            }
            None if !self.full_access_canary_children.lock().map_err(|_| "Native full-access canary supervision is unavailable")?.contains_key(&attempt.attempt_id) => {
                self.set_full_access_canary_state(id, "blocked")?;
            }
            None => {}
        }
        Ok(())
    }

    fn set_full_access_canary_state(&self, id: &str, state: &str) -> Result<(), String> {
        self.connection()?.execute(
            "UPDATE native_codex_profile_full_access_canaries SET state=?2,completed_at=?3 WHERE profile_id=?1 AND state='pending'",
            params![id, state, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        self.connection()?.execute(
            "UPDATE native_codex_profile_readiness SET danger_full_access_canary=?2,observed_at=?3 WHERE profile_id=?1",
            params![id, state, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn require_active(&self, id: &str) -> Result<StoredProfile, String> {
        let profile = self.profile(id)?;
        if profile.lifecycle != Lifecycle::Active {
            return Err("Native Codex home lost continuity and must be registered again".into());
        }
        let lifecycle = validate_profile(&profile);
        if lifecycle != Lifecycle::Active {
            self.record_lifecycle(id, lifecycle)?;
            return Err("Native Codex home is not currently validated".into());
        }
        Ok(profile)
    }

    fn require_selected_active(&self, id: &str) -> Result<StoredProfile, String> {
        let profile = self.require_active(id)?;
        if !profile.selected {
            return Err("Only the selected native Codex profile can perform this operation".into());
        }
        Ok(profile)
    }

    fn revalidate(&self, profile: &StoredProfile) -> Result<(), String> {
        let lifecycle = validate_profile(profile);
        if lifecycle != Lifecycle::Active {
            self.record_lifecycle(&profile.id, lifecycle)?;
        }
        Ok(())
    }

    fn probe_root(&self, id: &str) -> PathBuf {
        self.dedicated_root
            .parent()
            .unwrap_or(&self.dedicated_root)
            .join("native-codex-profile-probes")
            .join(id)
    }

    fn ensure_probe_root(&self, id: &str) -> Result<PathBuf, String> {
        let root = self.probe_root(id);
        fs::create_dir_all(&root).map_err(|error| {
            format!("Unable to create application-owned native profile probe root: {error}")
        })?;
        fs::canonicalize(root)
            .map_err(|_| "Unable to validate application-owned native profile probe root".into())
    }

    fn full_access_canary_root(&self, id: &str) -> Result<PathBuf, String> {
        let root = self
            .dedicated_root
            .parent()
            .unwrap_or(&self.dedicated_root)
            .join("native-codex-full-access-canaries")
            .join(id);
        fs::create_dir_all(&root).map_err(|error| {
            format!("Unable to create application-owned full-access canary root: {error}")
        })?;
        fs::canonicalize(root)
            .map_err(|_| "Unable to validate application-owned full-access canary root".into())
    }

    fn start_setup_attempt(
        &self,
        profile: &StoredProfile,
        phase: SetupPhase,
    ) -> Result<(), String> {
        let id = &profile.id;
        let gate = self
            .operation_gate
            .lock()
            .map_err(|_| "Native profile operation supervision is unavailable")?;
        let current = self.profile(id)?;
        let lifecycle = validate_profile(&current);
        if current.lifecycle != Lifecycle::Active || lifecycle != Lifecycle::Active || !current.selected {
            drop(gate);
            if lifecycle != Lifecycle::Active {
                self.record_lifecycle(id, lifecycle)?;
            }
            return Err("Only the selected, currently validated native Codex profile can start setup or canary work".into());
        }
        let surface = self.cli.surface().map_err(|_| {
            self.set_attention(id, "cli", Some("codex_cli_surface_unsupported"), false)
                .ok();
            "The resolved Codex CLI surface is unsupported for native sandbox setup".to_string()
        })?;
        let now = Utc::now();
        let attempt = PendingSetupAttempt {
            attempt_id: format!("native-setup-attempt-{}", Uuid::new_v4()),
            profile_id: id.clone(),
            filesystem_identity: current.identity.clone(),
            phase,
            deadline_at: now + Duration::seconds(SETUP_ATTEMPT_TIMEOUT_SECONDS),
        };
        if !surface.provenance.workspace_sandbox_supported
            || !surface.windows_sandbox_setup_supported
        {
            let mut unsupported_provenance = surface.provenance.clone();
            unsupported_provenance.workspace_sandbox_supported = false;
            self.persist_setup_attempt(
                &attempt,
                &unsupported_provenance,
                "policy_unsupported",
                "policy_unsupported",
            )?;
            self.set_attention(
                id,
                "cli",
                Some("codex_cli_workspace_semantic_policy_unsupported"),
                false,
            )?;
            return self.update_readiness(
                id,
                None,
                (phase == SetupPhase::SandboxInitialization).then_some("attention_required"),
                (phase == SetupPhase::WorkspaceWriteCanary).then_some("blocked"),
                None,
                Some((
                    phase.attention_concern(),
                    Some("native_sandbox_semantic_policy_unsupported"),
                )),
            );
        }
        let inserted = self.persist_setup_attempt(&attempt, &surface.provenance, "pending", "not_observed")?;
        if inserted == 0 {
            return self.set_attention(
                id,
                phase.attention_concern(),
                Some("native_sandbox_attempt_pending_human_or_uac_attention"),
                false,
            );
        }
        let (cwd, args, sandbox_receipt, sandbox_command_file) = match phase {
            // This is the supported Windows provisioning command. Its launch acceptance and
            // terminal outcome do not observe a UAC interaction or confirm initialization.
            SetupPhase::SandboxInitialization => (
                profile.home.clone(),
                vec![
                    "sandbox".into(),
                    "setup".into(),
                    "--elevated".into(),
                    "--current-user".into(),
                    "--codex-home".into(),
                    profile.home.to_string_lossy().into_owned(),
                ],
                None,
                None,
            ),
            SetupPhase::WorkspaceWriteCanary => {
                let root = self.probe_root(id);
                fs::create_dir_all(&root).map_err(|error| {
                    format!("Unable to create application-owned sandbox probe root: {error}")
                })?;
                let output = root.join(format!("{}.txt", phase.database()));
                if let Err(error) = fs::remove_file(&output) {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        self.set_setup_attempt_state(
                            &attempt.attempt_id,
                            "launch_failed",
                            "launch_failed",
                            None,
                        )?;
                        self.update_readiness(
                            id,
                            None,
                            None,
                            Some("blocked"),
                            None,
                            Some((
                                "canary",
                                Some("native_sandbox_canary_receipt_cleanup_failed"),
                            )),
                        )?;
                        return Ok(());
                    }
                }
                let command_file = root.join(WORKSPACE_WRITE_CANARY_COMMAND_FILE);
                if let Err(error) = fs::remove_file(&command_file) {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        self.set_setup_attempt_state(
                            &attempt.attempt_id,
                            "launch_failed",
                            "launch_failed",
                            None,
                        )?;
                        self.update_readiness(
                            id,
                            None,
                            None,
                            Some("blocked"),
                            None,
                            Some((
                                "canary",
                                Some("native_sandbox_canary_command_cleanup_failed"),
                            )),
                        )?;
                        return Err(format!(
                            "Unable to clear the application-owned sandbox canary command: {error}"
                        ));
                    }
                }
                if let Err(error) = fs::write(
                    &command_file,
                    "@echo off\r\necho native-codex-profile-canary>workspace_write_canary.txt\r\n",
                ) {
                    self.set_setup_attempt_state(
                        &attempt.attempt_id,
                        "launch_failed",
                        "launch_failed",
                        None,
                    )?;
                    self.update_readiness(
                        id,
                        None,
                        None,
                        Some("blocked"),
                        None,
                        Some(("canary", Some("native_sandbox_canary_command_prepare_failed"))),
                    )?;
                    return Err(format!(
                        "Unable to prepare the application-owned sandbox canary command: {error}"
                    ));
                }
                (
                    root.clone(),
                    vec![
                        "sandbox".into(),
                        "-P".into(),
                        ":workspace".into(),
                        "-C".into(),
                        root.to_string_lossy().into_owned(),
                        "--".into(),
                        "cmd.exe".into(),
                        "/d".into(),
                        "/c".into(),
                        // Do not send a quote-bearing redirection expression through the
                        // sandbox's CreateProcess command-vector boundary. The command file is
                        // application-authored, relative to the exact probe root, and removed
                        // after the owned child settles.
                        format!(".\\{WORKSPACE_WRITE_CANARY_COMMAND_FILE}"),
                    ],
                    Some(output),
                    Some(command_file),
                )
            }
        };
        let invocation = NativeCliInvocation {
            args,
            cwd,
            codex_home: profile.home.clone(),
            environment: native_windows_cli_environment(&profile.home),
            sandbox_receipt,
            sandbox_command_file: sandbox_command_file.clone(),
        };
        match self.cli.start(&invocation) {
            Ok(mut child) => {
                match self.setup_children.lock() {
                    Ok(mut children) => {
                        children.insert(attempt.attempt_id.clone(), child);
                    }
                    Err(_) => {
                        let _ = child.terminate();
                        if let Some(path) = sandbox_command_file.as_ref() {
                            let _ = fs::remove_file(path);
                        }
                        self.set_setup_attempt_state(
                            &attempt.attempt_id,
                            "cancelled",
                            "cancelled",
                            None,
                        )?;
                        return Err("Native sandbox child supervision is unavailable".into());
                    }
                }
                self.mark_setup_attempt_launch_accepted(&attempt.attempt_id)?;
                self.set_attention(
                    id,
                    phase.attention_concern(),
                    Some("native_sandbox_attempt_pending_human_or_uac_attention"),
                    false,
                )
            }
            Err(_) => {
                if let Some(path) = sandbox_command_file.as_ref() {
                    let _ = fs::remove_file(path);
                }
                self.set_setup_attempt_state(
                    &attempt.attempt_id,
                    "launch_failed",
                    "launch_failed",
                    None,
                )?;
                self.set_attention(id, "cli", Some("codex_cli_unavailable"), false)?;
                self.update_readiness(
                    id,
                    None,
                    (phase == SetupPhase::SandboxInitialization).then_some("attention_required"),
                    (phase == SetupPhase::WorkspaceWriteCanary).then_some("blocked"),
                    None,
                    Some((
                        phase.attention_concern(),
                        Some("native_sandbox_launch_failed"),
                    )),
                )
            }
        }
    }

    fn reconcile_setup_attempts(&self, id: &str) -> Result<(), String> {
        for attempt in load_pending_setup_attempts(&self.connection()?, id)? {
            let profile = self.profile(id)?;
            let authority_valid = profile.lifecycle == Lifecycle::Active
                && validate_profile(&profile) == Lifecycle::Active
                && profile.selected
                && profile.identity == attempt.filesystem_identity;
            if !authority_valid {
                if let Some(mut child) = self
                    .setup_children
                    .lock()
                    .map_err(|_| "Native sandbox child supervision is unavailable")?
                    .remove(&attempt.attempt_id)
                {
                    let _ = child.terminate();
                }
                self.settle_failed_setup_attempt(&attempt, "cancelled", "cancelled", None)?;
                continue;
            }
            let outcome = {
                let mut children = self
                    .setup_children
                    .lock()
                    .map_err(|_| "Native sandbox child supervision is unavailable")?;
                match children.get_mut(&attempt.attempt_id) {
                    Some(child) => match child.try_wait()? {
                        Some(receipt) => {
                            children.remove(&attempt.attempt_id);
                            Some(Ok(receipt))
                        }
                        None if Utc::now() >= attempt.deadline_at => {
                            let mut child = children.remove(&attempt.attempt_id).expect("present");
                            let _ = child.terminate();
                            Some(Err("timed_out"))
                        }
                        None => None,
                    },
                    None if Utc::now() >= attempt.deadline_at => Some(Err("timed_out")),
                    None => Some(Err("recovered_unobserved")),
                }
            };
            let Some(outcome) = outcome else { continue };
            match outcome {
                Ok(receipt)
                    if receipt.succeeded
                        && (attempt.phase == SetupPhase::SandboxInitialization
                            || receipt.sandbox_receipt_observed) =>
                {
                    self.set_setup_attempt_state(
                        &attempt.attempt_id,
                        "terminal_succeeded",
                        if receipt.exit_code.is_some() { "exit_code" } else { "not_observed" },
                        receipt.exit_code,
                    )?;
                    if attempt.phase == SetupPhase::SandboxInitialization {
                        self.update_readiness(
                            id,
                            None,
                            Some("attention_required"),
                            None,
                            None,
                            Some((
                                "sandbox",
                                Some("native_sandbox_setup_completed_explicit_uac_confirmation_required"),
                            )),
                        )?;
                    } else {
                        self.update_readiness(
                            id,
                            None,
                            None,
                            Some("passed"),
                            None,
                            Some((attempt.phase.attention_concern(), None)),
                        )?;
                        self.remove_workspace_write_canary_command(&attempt.profile_id);
                    }
                }
                Ok(receipt) => {
                    let terminal_classification = if attempt.phase == SetupPhase::WorkspaceWriteCanary
                        && !receipt.sandbox_receipt_observed
                    {
                        // This says only that the one owned sentinel was absent; the separately
                        // persisted exit code remains available without retaining raw CLI output.
                        "receipt_missing"
                    } else if receipt.exit_code.is_some() {
                        "exit_code"
                    } else {
                        "not_observed"
                    };
                    self.settle_failed_setup_attempt(
                        &attempt,
                        "terminal_failed",
                        terminal_classification,
                        receipt.exit_code,
                    )?
                }
                Err(state) => self.settle_failed_setup_attempt(
                    &attempt,
                    state,
                    state,
                    None,
                )?,
            }
        }
        Ok(())
    }

    fn settle_failed_setup_attempt(
        &self,
        attempt: &PendingSetupAttempt,
        state: &str,
        terminal_classification: &str,
        terminal_exit_code: Option<i32>,
    ) -> Result<(), String> {
        self.set_setup_attempt_state(
            &attempt.attempt_id,
            state,
            terminal_classification,
            terminal_exit_code,
        )?;
        self.update_readiness(
            &attempt.profile_id,
            None,
            (attempt.phase == SetupPhase::SandboxInitialization).then_some("attention_required"),
            (attempt.phase == SetupPhase::WorkspaceWriteCanary).then_some("blocked"),
            None,
            Some((
                attempt.phase.attention_concern(),
                Some(match state {
                    "timed_out" => "native_sandbox_attempt_timed_out_human_or_uac_attention",
                    "cancelled" => "native_sandbox_attempt_cancelled_before_observation",
                    "recovered_unobserved" => "native_sandbox_attempt_recovered_without_owned_process",
                    _ => "native_sandbox_attempt_failed",
                }),
            )),
        )?;
        if attempt.phase == SetupPhase::WorkspaceWriteCanary {
            self.remove_workspace_write_canary_command(&attempt.profile_id);
        }
        Ok(())
    }

    fn remove_workspace_write_canary_command(&self, profile_id: &str) {
        let _ = fs::remove_file(
            self.probe_root(profile_id)
                .join(WORKSPACE_WRITE_CANARY_COMMAND_FILE),
        );
    }

    fn mark_setup_attempt_launch_accepted(&self, attempt_id: &str) -> Result<(), String> {
        self.connection()?.execute(
            "UPDATE native_codex_profile_setup_attempts SET launch_accepted_at=?2 WHERE attempt_id=?1 AND state='pending'",
            params![attempt_id, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn persist_setup_attempt(
        &self,
        attempt: &PendingSetupAttempt,
        provenance: &NativeCliProvenance,
        state: &str,
        terminal_classification: &str,
    ) -> Result<usize, String> {
        self.connection()?.execute(
            "INSERT OR IGNORE INTO native_codex_profile_setup_attempts (attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,deadline_at,settled_at,terminal_classification) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,CASE WHEN ?5='pending' THEN NULL ELSE ?10 END,?12)",
            params![attempt.attempt_id, attempt.profile_id, attempt.filesystem_identity, attempt.phase.database(), state, provenance.executable, provenance.version, provenance.workspace_sandbox_supported, format!("native-setup-{}", Uuid::new_v4()), Utc::now().to_rfc3339(), attempt.deadline_at.to_rfc3339(), terminal_classification],
        ).map_err(|error| format!("Unable to persist native sandbox attempt: {error}"))
    }

    fn set_setup_attempt_state(
        &self,
        attempt_id: &str,
        state: &str,
        terminal_classification: &str,
        terminal_exit_code: Option<i32>,
    ) -> Result<(), String> {
        self.connection()?.execute(
            "UPDATE native_codex_profile_setup_attempts SET state=?2,settled_at=?3,terminal_classification=?4,terminal_exit_code=?5 WHERE attempt_id=?1 AND state='pending'",
            params![attempt_id, state, Utc::now().to_rfc3339(), terminal_classification, terminal_exit_code],
        ).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn expire_mcp_probe(&self, id: &str) -> Result<(), String> {
        let connection = self.connection()?;
        let expired = connection
            .execute(
                "UPDATE native_codex_profile_mcp_probes SET state='expired' WHERE profile_id=?1 AND state='pending' AND deadline_at <= ?2",
                params![id, Utc::now().to_rfc3339()],
            )
            .map_err(|error| error.to_string())?;
        if expired != 0 {
            self.update_readiness(
                id,
                None,
                None,
                None,
                Some("not_assessed"),
                Some(("mcp_reporting", Some("mcp_reporting_probe_expired"))),
            )?;
        }
        Ok(())
    }

    fn reconcile_login_attempt(&self, id: &str) -> Result<(), String> {
        let Some(attempt) = load_pending_login_attempt(&self.connection()?, id)? else {
            return Ok(());
        };
        let profile = self.profile(id)?;
        let authority_valid = profile.selected
            && profile.lifecycle == Lifecycle::Active
            && validate_profile(&profile) == Lifecycle::Active
            && profile.identity == attempt.filesystem_identity;
        if !authority_valid {
            if let Some(mut child) = self
                .login_children
                .lock()
                .map_err(|_| "Native profile login supervision is unavailable")?
                .remove(id)
            {
                let _ = child.terminate();
            }
            self.set_login_attempt_state(&attempt.attempt_id, "cancelled")?;
            self.set_attention(
                id,
                "authentication",
                Some("browser_login_attempt_cancelled"),
                false,
            )?;
            return Ok(());
        }
        let outcome = {
            let mut children = self
                .login_children
                .lock()
                .map_err(|_| "Native profile login supervision is unavailable")?;
            match children.get_mut(id) {
                Some(child) => match child.try_wait()? {
                    Some(receipt) => {
                        children.remove(id);
                        Some(Ok(receipt.succeeded))
                    }
                    None => None,
                },
                None => Some(Err(())),
            }
        };
        match outcome {
            Some(Ok(true)) => {
                self.set_login_attempt_state(&attempt.attempt_id, "terminal_succeeded")?;
                self.set_attention(
                    id,
                    "authentication",
                    Some("browser_login_terminal_succeeded_browser_handoff_unobserved"),
                    false,
                )?;
            }
            Some(Ok(false)) => {
                self.set_login_attempt_state(&attempt.attempt_id, "terminal_failed")?;
                self.set_attention(
                    id,
                    "authentication",
                    Some("browser_login_terminal_failed"),
                    false,
                )?;
            }
            Some(Err(())) => {
                self.set_login_attempt_state(&attempt.attempt_id, "recovered_unobserved")?;
                self.set_attention(
                    id,
                    "authentication",
                    Some("browser_login_recovered_without_owned_process"),
                    false,
                )?;
            }
            None => {}
        }
        Ok(())
    }

    fn set_login_attempt_state(&self, attempt_id: &str, state: &str) -> Result<(), String> {
        self.connection()?.execute(
            "UPDATE native_codex_profile_login_attempts SET state=?2,settled_at=?3 WHERE attempt_id=?1 AND state='pending'",
            params![attempt_id, state, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn profile(&self, id: &str) -> Result<StoredProfile, String> {
        let mut connection = self.connection()?;
        load_profiles(&mut connection)?
            .into_iter()
            .find(|profile| profile.id == id)
            .ok_or_else(|| "Native Codex profile was not found".into())
    }

    fn profile_by_home(&self, home: &Path) -> Result<Option<StoredProfile>, String> {
        let mut connection = self.connection()?;
        Ok(load_profiles(&mut connection)?
            .into_iter()
            .find(|profile| profile.home == home))
    }

    fn record_lifecycle(&self, id: &str, lifecycle: Lifecycle) -> Result<(), String> {
        let _gate = self
            .operation_gate
            .lock()
            .map_err(|_| "Native profile operation supervision is unavailable")?;
        self.record_lifecycle_while_gated(id, lifecycle)
    }

    /// Caller owns `operation_gate`; preserves lifecycle transition and child cancellation as
    /// one serialized operation without recursively locking the non-reentrant mutex.
    fn record_lifecycle_while_gated(&self, id: &str, lifecycle: Lifecycle) -> Result<(), String> {
        if let Some(mut child) = self
            .login_children
            .lock()
            .map_err(|_| "Native profile login supervision is unavailable")?
            .remove(id)
        {
            let _ = child.terminate();
        }
        let attempts = load_pending_setup_attempts(&self.connection()?, id)?;
        let mut children = self
            .setup_children
            .lock()
            .map_err(|_| "Native sandbox child supervision is unavailable")?;
        for attempt in attempts {
            if let Some(mut child) = children.remove(&attempt.attempt_id) {
                let _ = child.terminate();
            }
        }
        drop(children);
        if let Ok(mut children) = self.full_access_canary_children.lock() {
            if let Some(mut child) = load_pending_full_access_canary(&self.connection()?, id)?
                .and_then(|attempt| children.remove(&attempt.attempt_id))
            {
                let _ = child.terminate();
            }
        }
        let connection = self.connection()?;
        connection.execute("UPDATE native_codex_profiles SET lifecycle=?2,selected_at=NULL,updated_at=?3 WHERE id=?1", params![id, lifecycle.database(), Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        connection.execute("UPDATE native_codex_profile_mode_authorizations SET revoked_at=COALESCE(revoked_at,?2) WHERE profile_id=?1 AND mode='danger_full_access'", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        connection.execute("UPDATE native_codex_profile_readiness SET authentication='unknown',sandbox_initialization='unknown',workspace_write_canary='not_run',danger_full_access_canary='blocked',mcp_reporting='not_assessed',observed_at=?2 WHERE profile_id=?1", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        connection.execute("UPDATE native_codex_profile_setup_attempts SET state='cancelled',settled_at=?2,terminal_classification='cancelled' WHERE profile_id=?1 AND state='pending'", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        connection.execute("UPDATE native_codex_profile_mcp_probes SET state='cancelled' WHERE profile_id=?1 AND state='pending'", params![id]).map_err(|error| error.to_string())?;
        connection.execute("UPDATE native_codex_profile_full_access_canaries SET state='cancelled',completed_at=?2 WHERE profile_id=?1 AND state='pending'", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        connection.execute("UPDATE native_codex_profile_login_attempts SET state='cancelled',settled_at=?2 WHERE profile_id=?1 AND state='pending'", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        connection.execute("UPDATE native_codex_profile_sandbox_adoptions SET state='invalidated' WHERE profile_id=?1", params![id]).map_err(|error| error.to_string())?;
        connection.execute("UPDATE native_codex_profile_sandbox_adoption_confirmations SET state='invalidated',invalidated_at=COALESCE(invalidated_at,?2) WHERE profile_id=?1 AND state='confirmed'", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        self.write_attention(
            &connection,
            id,
            "continuity",
            Some("profile_continuity_lost"),
        )?;
        Ok(())
    }

    fn invalidate_sandbox_adoption(&self, id: &str) -> Result<(), String> {
        let connection = self.connection()?;
        connection.execute(
            "UPDATE native_codex_profile_sandbox_adoptions SET state='invalidated' WHERE profile_id=?1",
            params![id],
        ).map_err(|error| error.to_string())?;
        connection.execute(
            "UPDATE native_codex_profile_sandbox_adoption_confirmations SET state='invalidated',invalidated_at=COALESCE(invalidated_at,?2) WHERE profile_id=?1 AND state='confirmed'",
            params![id, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        self.update_readiness(
            id,
            None,
            Some("attention_required"),
            Some("blocked"),
            None,
            Some(("sandbox", Some("external_sandbox_adoption_evidence_invalidated"))),
        )
    }

    /// A verified external observation and a product confirmation are both bound to the current
    /// selected profile, identity, CLI provenance/capabilities, and exact narrow observation.
    /// Any drift invalidates the durable adoption evidence before readiness can be consumed.
    fn reconcile_sandbox_adoption(&self, id: &str) -> Result<(), String> {
        let profile = self.profile(id)?;
        let adoption = profile.sandbox_adoption;
        let confirmation = profile.sandbox_adoption_confirmation;
        if adoption.disposition == "not_verified" && confirmation.disposition == "not_confirmed" {
            return Ok(());
        }
        // An already-invalidated external route must not continually overwrite a later,
        // independently valid product-owned setup result.
        if adoption.disposition == "invalidated" {
            return Ok(());
        }
        if confirmation.disposition == "invalidated" {
            return self.invalidate_sandbox_adoption(id);
        }
        if !profile.selected || profile.lifecycle != Lifecycle::Active || adoption.disposition != "verified" {
            return self.invalidate_sandbox_adoption(id);
        }
        let surface = self.cli.surface();
        let observed = observe_elevated_windows_sandbox_mode(&profile.home);
        let valid = surface.as_ref().ok().is_some_and(|surface| {
            adoption.executable.as_deref() == Some(surface.provenance.executable.as_str())
                && adoption.version.as_deref() == Some(surface.provenance.version.as_str())
                && adoption.workspace_sandbox_supported == Some(true)
                && adoption.windows_sandbox_setup_supported == Some(true)
                && surface.provenance.workspace_sandbox_supported
                && surface.windows_sandbox_setup_supported
        }) && observed == Ok(true);
        if !valid {
            return self.invalidate_sandbox_adoption(id);
        }
        if confirmation.disposition == "invalidated" {
            self.update_readiness(
                id,
                None,
                Some("attention_required"),
                Some("blocked"),
                None,
                Some(("sandbox", Some("external_sandbox_adoption_evidence_invalidated"))),
            )?;
        }
        Ok(())
    }

    fn update_readiness(
        &self,
        id: &str,
        authentication: Option<&str>,
        sandbox: Option<&str>,
        canary: Option<&str>,
        mcp: Option<&str>,
        attention: Option<(&str, Option<&str>)>,
    ) -> Result<(), String> {
        let connection = self.connection()?;
        connection.execute(
            "UPDATE native_codex_profile_readiness SET authentication=COALESCE(?2,authentication),sandbox_initialization=COALESCE(?3,sandbox_initialization),workspace_write_canary=COALESCE(?4,workspace_write_canary),mcp_reporting=COALESCE(?5,mcp_reporting),observed_at=?6 WHERE profile_id=?1",
            params![id, authentication, sandbox, canary, mcp, Utc::now().to_rfc3339()],
        ).map_err(|error| format!("Unable to record native profile readiness: {error}"))?;
        if let Some((concern, detail)) = attention {
            self.write_attention(&connection, id, concern, detail)?;
        }
        Ok(())
    }

    fn set_attention(
        &self,
        id: &str,
        concern: &str,
        attention: Option<&str>,
        reset_readiness: bool,
    ) -> Result<(), String> {
        let connection = self.connection()?;
        if reset_readiness {
            connection.execute("UPDATE native_codex_profile_readiness SET authentication='unknown',sandbox_initialization='unknown',workspace_write_canary='not_run',mcp_reporting='not_assessed',observed_at=?2 WHERE profile_id=?1", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        } else {
            connection
                .execute(
                    "UPDATE native_codex_profile_readiness SET observed_at=?2 WHERE profile_id=?1",
                    params![id, Utc::now().to_rfc3339()],
                )
                .map_err(|error| error.to_string())?;
        }
        self.write_attention(&connection, id, concern, attention)?;
        Ok(())
    }

    fn write_attention(
        &self,
        connection: &Connection,
        id: &str,
        concern: &str,
        detail: Option<&str>,
    ) -> Result<(), String> {
        if let Some(detail) = detail {
            connection.execute(
                "INSERT INTO native_codex_profile_attentions (profile_id,concern,detail,recorded_at) VALUES (?1,?2,?3,?4) ON CONFLICT(profile_id,concern) DO UPDATE SET detail=excluded.detail,recorded_at=excluded.recorded_at",
                params![id, concern, detail, Utc::now().to_rfc3339()],
            ).map_err(|error| error.to_string())?;
        } else {
            connection.execute(
                "DELETE FROM native_codex_profile_attentions WHERE profile_id=?1 AND concern=?2",
                params![id, concern],
            ).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

impl Drop for NativeProfileService {
    fn drop(&mut self) {
        if let Ok(mut children) = self.login_children.lock() {
            for child in children.values_mut() {
                let _ = child.terminate();
            }
            children.clear();
        }
        if let Ok(mut children) = self.setup_children.lock() {
            for child in children.values_mut() {
                let _ = child.terminate();
            }
            children.clear();
        }
        if let Ok(mut children) = self.full_access_canary_children.lock() {
            for child in children.values_mut() {
                let _ = child.terminate();
            }
            children.clear();
        }
    }
}

/// This capability is intentionally application-side only. Its construction is the single point
/// at which selected identity and all independently observed readiness facts are consumed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedNativeCodexHome {
    pub(crate) home: PathBuf,
    pub(crate) readiness: NativeProfileReadiness,
}

fn validated_absolute_directory(supplied: &str) -> Result<PathBuf, String> {
    let path = Path::new(supplied);
    if !path.is_absolute() {
        return Err("Codex home must be an absolute path".into());
    }
    let canonical = fs::canonicalize(path).map_err(|_| "Codex home is missing or inaccessible")?;
    if !canonical.is_dir() {
        return Err("Codex home must be a directory".into());
    }
    Ok(canonical)
}

fn native_profile_environment(home: &Path) -> Vec<(String, String)> {
    vec![("CODEX_HOME".into(), home.to_string_lossy().into_owned())]
}

/// Windows CLI work retains only the system launch facilities and launching user's standard
/// directories. `CODEX_HOME` remains product-selected; no inherited Codex home, credentials,
/// or provider state is admitted.
fn native_windows_cli_environment(home: &Path) -> Vec<(String, String)> {
    native_windows_cli_environment_from(home, &|key| std::env::var(key).ok())
}

fn native_windows_cli_environment_from(
    home: &Path,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Vec<(String, String)> {
    let mut environment = native_profile_environment(home);
    #[cfg(windows)]
    for key in [
        "APPDATA",
        "COMSPEC",
        "HOMEDRIVE",
        "HOMEPATH",
        "LOCALAPPDATA",
        "PATH",
        "PATHEXT",
        "SYSTEMROOT",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "WINDIR",
    ] {
        if let Some(value) = lookup(key).filter(|value| !value.trim().is_empty()) {
            environment.push((key.into(), value));
        }
    }
    environment
}

fn filesystem_identity(home: &Path) -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::{iter::once, os::windows::ffi::OsStrExt, ptr::null_mut};
        use windows_sys::Win32::{
            Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
            Storage::FileSystem::{
                CreateFileW, GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
                FILE_FLAG_BACKUP_SEMANTICS, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
                FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            },
        };
        let path = home
            .as_os_str()
            .encode_wide()
            .chain(once(0))
            .collect::<Vec<_>>();
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err("Unable to open Codex home identity".into());
        }
        let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        let succeeded = unsafe { GetFileInformationByHandle(handle, &mut information) } != 0;
        unsafe { CloseHandle(handle) };
        if !succeeded {
            return Err("Unable to read collision-resistant Windows directory identity".into());
        }
        return Ok(format!(
            "windows:{}:{}:{}",
            information.dwVolumeSerialNumber, information.nFileIndexHigh, information.nFileIndexLow
        ));
    }
    #[cfg(not(windows))]
    {
        let metadata = fs::metadata(home).map_err(|_| "Codex home is missing or inaccessible")?;
        let created = metadata
            .created()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|time| time.as_nanos())
            .ok_or("Filesystem does not expose a usable directory identity")?;
        Ok(format!("directory:{created}"))
    }
}

fn write_marker(home: &Path, id: &str) -> Result<(), String> {
    let path = home.join(MARKER_FILE);
    let payload = serde_json::to_vec(&DedicatedMarker {
        contract: "native-codex-home-marker/v1",
        profile_id: id,
    })
    .map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("Unable to create dedicated home ownership marker: {error}"))?;
    file.write_all(&payload)
        .map_err(|error| format!("Unable to write dedicated home ownership marker: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Unable to persist dedicated home ownership marker: {error}"))
}

fn validate_profile(profile: &StoredProfile) -> Lifecycle {
    let home = match validated_absolute_directory(&profile.home.to_string_lossy()) {
        Ok(home) => home,
        Err(_) => return Lifecycle::MissingOrMoved,
    };
    let identity = match filesystem_identity(&home) {
        Ok(identity) => identity,
        Err(_) => return Lifecycle::MissingOrMoved,
    };
    if identity != profile.identity {
        return Lifecycle::Replaced;
    }
    match profile.ownership {
        Ownership::RegisteredExisting => {
            if home.join(MARKER_FILE).exists() {
                Lifecycle::Foreign
            } else {
                Lifecycle::Active
            }
        }
        Ownership::ApplicationDedicated => match fs::read(home.join(MARKER_FILE))
            .ok()
            .and_then(|payload| serde_json::from_slice::<ReadDedicatedMarker>(&payload).ok())
        {
            Some(marker)
                if marker.contract == "native-codex-home-marker/v1"
                    && marker.profile_id == profile.id =>
            {
                Lifecycle::Active
            }
            Some(_) => Lifecycle::Foreign,
            None => Lifecycle::Malformed,
        },
    }
}

fn load_pending_setup_attempts(
    connection: &Connection,
    profile_id: &str,
) -> Result<Vec<PendingSetupAttempt>, String> {
    let mut statement = connection
        .prepare(
            "SELECT attempt_id,profile_id,filesystem_identity,phase,deadline_at FROM native_codex_profile_setup_attempts WHERE profile_id=?1 AND state='pending' ORDER BY requested_at",
        )
        .map_err(|error| error.to_string())?;
    let attempts = statement
        .query_map(params![profile_id], |row| {
            let deadline: String = row.get(4)?;
            Ok(PendingSetupAttempt {
                attempt_id: row.get(0)?,
                profile_id: row.get(1)?,
                filesystem_identity: row.get(2)?,
                phase: SetupPhase::from_database(&row.get::<_, String>(3)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                deadline_at: DateTime::parse_from_rfc3339(&deadline)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc),
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(attempts)
}

fn load_sandbox_adoption(
    connection: &Connection,
    profile_id: &str,
    identity: &str,
) -> Result<NativeProfileSandboxAdoption, String> {
    let adoption = connection
        .query_row(
            "SELECT filesystem_identity,executable,version,workspace_sandbox_supported,windows_sandbox_setup_supported,correlation_id,observed_at,state,elevated_mode_observed FROM native_codex_profile_sandbox_adoptions WHERE profile_id=?1",
            params![profile_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, i64>(8)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((stored_identity, executable, version, workspace, setup, correlation, observed_at, state, elevated)) = adoption else {
        return Ok(NativeProfileSandboxAdoption { disposition: "not_verified".into(), executable: None, version: None, workspace_sandbox_supported: None, windows_sandbox_setup_supported: None, correlation_id: None, observed_at: None, elevated_mode_observed: None });
    };
    if stored_identity != identity || state == "invalidated" {
        return Ok(NativeProfileSandboxAdoption { disposition: "invalidated".into(), executable: Some(executable), version: Some(version), workspace_sandbox_supported: Some(workspace != 0), windows_sandbox_setup_supported: Some(setup != 0), correlation_id: Some(correlation), observed_at: Some(observed_at), elevated_mode_observed: Some(elevated != 0) });
    }
    let adoption = NativeProfileSandboxAdoption { disposition: state, executable: Some(executable), version: Some(version), workspace_sandbox_supported: Some(workspace != 0), windows_sandbox_setup_supported: Some(setup != 0), correlation_id: Some(correlation), observed_at: Some(observed_at), elevated_mode_observed: Some(elevated != 0) };
    validate_sandbox_adoption(&adoption)?;
    Ok(adoption)
}

fn validate_sandbox_adoption(adoption: &NativeProfileSandboxAdoption) -> Result<(), String> {
    let required = |value: &Option<String>| value.as_deref().is_some_and(|value| !value.is_empty() && value.trim() == value);
    if !matches!(adoption.disposition.as_str(), "verified" | "not_verified" | "invalidated")
        || !required(&adoption.executable)
        || !required(&adoption.version)
        || !required(&adoption.correlation_id)
        || adoption.observed_at.as_deref().is_none_or(|value| DateTime::parse_from_rfc3339(value).is_err())
        || adoption.workspace_sandbox_supported.is_none()
        || adoption.windows_sandbox_setup_supported.is_none()
        || adoption.elevated_mode_observed.is_none()
        || (adoption.disposition == "verified" && (!adoption.workspace_sandbox_supported.unwrap() || !adoption.windows_sandbox_setup_supported.unwrap() || !adoption.elevated_mode_observed.unwrap()))
    {
        return Err("Native sandbox adoption evidence violates its durable invariant".into());
    }
    Ok(())
}

fn load_sandbox_adoption_confirmation(
    connection: &Connection,
    profile_id: &str,
    identity: &str,
    adoption: &NativeProfileSandboxAdoption,
) -> Result<NativeProfileSandboxAdoptionConfirmation, String> {
    let confirmation = connection.query_row(
        "SELECT filesystem_identity,adoption_correlation_id,confirmation_correlation_id,confirmed_at,state FROM native_codex_profile_sandbox_adoption_confirmations WHERE profile_id=?1",
        params![profile_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)),
    ).optional().map_err(|error| error.to_string())?;
    let Some((stored_identity, adoption_correlation, correlation, confirmed_at, state)) = confirmation else {
        return Ok(NativeProfileSandboxAdoptionConfirmation { disposition: "not_confirmed".into(), correlation_id: None, confirmed_at: None });
    };
    let matches_observation = stored_identity == identity
        && adoption.disposition == "verified"
        && adoption.correlation_id.as_deref() == Some(adoption_correlation.as_str());
    let confirmation = NativeProfileSandboxAdoptionConfirmation {
        disposition: if state == "confirmed" && matches_observation { "confirmed".into() } else { "invalidated".into() },
        correlation_id: Some(correlation),
        confirmed_at: Some(confirmed_at),
    };
    validate_sandbox_adoption_confirmation(&confirmation)?;
    Ok(confirmation)
}

fn validate_sandbox_adoption_confirmation(
    confirmation: &NativeProfileSandboxAdoptionConfirmation,
) -> Result<(), String> {
    if confirmation.disposition == "not_confirmed"
        && confirmation.correlation_id.is_none()
        && confirmation.confirmed_at.is_none()
    {
        return Ok(());
    }
    if !matches!(confirmation.disposition.as_str(), "confirmed" | "invalidated")
        || confirmation.correlation_id.as_deref().is_none_or(|value| value.is_empty() || value.trim() != value)
        || confirmation.confirmed_at.as_deref().is_none_or(|value| DateTime::parse_from_rfc3339(value).is_err())
    {
        return Err("Native sandbox adoption confirmation violates its durable invariant".into());
    }
    Ok(())
}

fn load_pending_full_access_canary(
    connection: &Connection,
    profile_id: &str,
) -> Result<Option<PendingFullAccessCanary>, String> {
    connection
        .query_row(
            "SELECT attempt_id,profile_id,filesystem_identity,executable,version,sentinel_path FROM native_codex_profile_full_access_canaries WHERE profile_id=?1 AND state='pending'",
            params![profile_id],
            |row| Ok(PendingFullAccessCanary {
                attempt_id: row.get(0)?,
                profile_id: row.get(1)?,
                filesystem_identity: row.get(2)?,
                executable: row.get(3)?,
                version: row.get(4)?,
                sentinel_path: PathBuf::from(row.get::<_, String>(5)?),
            }),
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn load_pending_login_attempt(
    connection: &Connection,
    profile_id: &str,
) -> Result<Option<PendingLoginAttempt>, String> {
    connection.query_row(
        "SELECT attempt_id,profile_id,filesystem_identity FROM native_codex_profile_login_attempts WHERE profile_id=?1 AND state='pending'",
        params![profile_id],
        |row| Ok(PendingLoginAttempt {
            attempt_id: row.get(0)?,
            profile_id: row.get(1)?,
            filesystem_identity: row.get(2)?,
        }),
    ).optional().map_err(|error| error.to_string())
}

fn load_pending_mcp_probe(
    connection: &Connection,
    profile_id: &str,
) -> Result<Option<NativeMcpReportingProbeAuthority>, String> {
    connection
        .query_row(
            "SELECT correlation_id,expected_capability,expected_server,expected_tool,expected_probe_root FROM native_codex_profile_mcp_probes WHERE profile_id=?1 AND state='pending'",
            params![profile_id],
            |row| Ok(NativeMcpReportingProbeAuthority {
                profile_id: profile_id.into(),
                correlation_id: row.get(0)?,
                capability: row.get(1)?,
                server: row.get(2)?,
                tool: row.get(3)?,
                probe_root: PathBuf::from(row.get::<_, String>(4)?),
            }),
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn load_profiles(connection: &mut Connection) -> Result<Vec<StoredProfile>, String> {
    let mut statement = connection.prepare("SELECT p.id,p.canonical_home_path,p.filesystem_identity,p.ownership,p.lifecycle,p.selected_at,r.authentication,r.sandbox_initialization,r.workspace_write_canary,r.danger_full_access_canary,r.mcp_reporting,e.selected_mode,a.filesystem_identity,a.revoked_at,(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='authentication'),(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='sandbox'),(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='canary'),(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='mcp_reporting'),(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='continuity'),(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='cli'),(SELECT state FROM native_codex_profile_login_attempts l WHERE l.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT browser_handoff FROM native_codex_profile_login_attempts l WHERE l.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT requested_at FROM native_codex_profile_login_attempts l WHERE l.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT launch_accepted_at FROM native_codex_profile_login_attempts l WHERE l.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT settled_at FROM native_codex_profile_login_attempts l WHERE l.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT phase FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT state FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT executable FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT version FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT workspace_sandbox_supported FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT correlation_id FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT requested_at FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT launch_accepted_at FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT deadline_at FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT settled_at FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT terminal_classification FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT terminal_exit_code FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1) FROM native_codex_profiles p JOIN native_codex_profile_readiness r ON r.profile_id=p.id JOIN native_codex_profile_execution_modes e ON e.profile_id=p.id LEFT JOIN native_codex_profile_mode_authorizations a ON a.profile_id=p.id AND a.mode='danger_full_access' ORDER BY p.created_at").map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let identity: String = row.get(2)?;
            let authorization_identity: Option<String> = row.get(12)?;
            let authorization_revoked: Option<String> = row.get(13)?;
            Ok(StoredProfile {
                id: row.get(0)?,
                home: PathBuf::from(row.get::<_, String>(1)?),
                identity: identity.clone(),
                ownership: Ownership::parse(&row.get::<_, String>(3)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                lifecycle: Lifecycle::parse(&row.get::<_, String>(4)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                selected: row.get::<_, Option<String>>(5)?.is_some(),
                execution: NativeProfileExecution {
                    selected_mode: ExecutionMode::parse(&row.get::<_, String>(11)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    danger_full_access_authorized: authorization_identity == Some(identity)
                        && authorization_revoked.is_none(),
                },
                login_attempt: NativeProfileLoginAttempt {
                    disposition: row
                        .get::<_, Option<String>>(20)?
                        .unwrap_or_else(|| "not_requested".into()),
                    browser_handoff: row
                        .get::<_, Option<String>>(21)?
                        .unwrap_or_else(|| "unobserved".into()),
                    requested_at: row.get(22)?,
                    launch_accepted_at: row.get(23)?,
                    settled_at: row.get(24)?,
                },
                setup_attempt: NativeProfileSetupAttempt {
                    phase: row
                        .get::<_, Option<String>>(25)?
                        .unwrap_or_else(|| "not_requested".into()),
                    disposition: row
                        .get::<_, Option<String>>(26)?
                        .unwrap_or_else(|| "not_requested".into()),
                    executable: row.get(27)?,
                    version: row.get(28)?,
                    workspace_sandbox_supported: row
                        .get::<_, Option<i64>>(29)?
                        .map(|value| value != 0),
                    correlation_id: row.get(30)?,
                    requested_at: row.get(31)?,
                    launch_accepted_at: row.get(32)?,
                    deadline_at: row.get(33)?,
                    settled_at: row.get(34)?,
                    terminal_classification: row
                        .get::<_, Option<String>>(35)?
                        .unwrap_or_else(|| "not_observed".into()),
                    terminal_exit_code: row.get(36)?,
                },
                sandbox_adoption: NativeProfileSandboxAdoption {
                    disposition: "not_verified".into(),
                    executable: None,
                    version: None,
                    workspace_sandbox_supported: None,
                    windows_sandbox_setup_supported: None,
                    correlation_id: None,
                    observed_at: None,
                    elevated_mode_observed: None,
                },
                sandbox_adoption_confirmation: NativeProfileSandboxAdoptionConfirmation {
                    disposition: "not_confirmed".into(),
                    correlation_id: None,
                    confirmed_at: None,
                },
                readiness: NativeProfileReadiness {
                    authentication: row.get(6)?,
                    sandbox_initialization: row.get(7)?,
                    workspace_write_canary: row.get(8)?,
                    danger_full_access_canary: row.get(9)?,
                    mcp_reporting: row.get(10)?,
                    attentions: NativeProfileAttentions {
                        authentication: row.get(14)?,
                        sandbox: row.get(15)?,
                        canary: row.get(16)?,
                        mcp_reporting: row.get(17)?,
                        continuity: row.get(18)?,
                        cli: row.get(19)?,
                    },
                },
            })
        })
        .map_err(|error| error.to_string())?;
    let mut profiles = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    for profile in &mut profiles {
        validate_setup_attempt(&profile.setup_attempt)?;
        profile.sandbox_adoption = load_sandbox_adoption(connection, &profile.id, &profile.identity)?;
        profile.sandbox_adoption_confirmation = load_sandbox_adoption_confirmation(
            connection,
            &profile.id,
            &profile.identity,
            &profile.sandbox_adoption,
        )?;
    }
    Ok(profiles)
}

fn validate_setup_attempt(attempt: &NativeProfileSetupAttempt) -> Result<(), String> {
    let required = |value: &Option<String>| {
        value
            .as_deref()
            .is_some_and(|value| !value.is_empty() && value.trim() == value)
    };
    match attempt.terminal_classification.as_str() {
        "policy_unsupported" => {
            if attempt.disposition != "policy_unsupported"
                || !matches!(
                    attempt.phase.as_str(),
                    "sandbox_initialization" | "workspace_write_canary"
                )
                || attempt.workspace_sandbox_supported != Some(false)
                || !required(&attempt.executable)
                || !required(&attempt.version)
                || !required(&attempt.correlation_id)
                || !required(&attempt.requested_at)
                || !required(&attempt.deadline_at)
                || !required(&attempt.settled_at)
                || attempt.launch_accepted_at.is_some()
                || attempt.terminal_exit_code.is_some()
            {
                return Err("Native policy-unsupported setup attempt violates its durable invariant".into());
            }
        }
        "receipt_missing" => {
            if attempt.disposition != "terminal_failed"
                || attempt.phase != "workspace_write_canary"
                || attempt.workspace_sandbox_supported != Some(true)
                || !required(&attempt.executable)
                || !required(&attempt.version)
                || !required(&attempt.correlation_id)
                || !required(&attempt.requested_at)
                || !required(&attempt.launch_accepted_at)
                || !required(&attempt.deadline_at)
                || !required(&attempt.settled_at)
            {
                return Err("Native receipt-missing setup attempt violates its durable invariant".into());
            }
        }
        _ => return Ok(()),
    }
    let requested = DateTime::parse_from_rfc3339(attempt.requested_at.as_deref().unwrap())
        .map_err(|_| "Native setup attempt has an invalid request timestamp")?;
    let deadline = DateTime::parse_from_rfc3339(attempt.deadline_at.as_deref().unwrap())
        .map_err(|_| "Native setup attempt has an invalid deadline timestamp")?;
    let settled = DateTime::parse_from_rfc3339(attempt.settled_at.as_deref().unwrap())
        .map_err(|_| "Native setup attempt has an invalid settlement timestamp")?;
    if deadline < requested || settled < requested {
        return Err("Native setup attempt has contradictory timestamps".into());
    }
    if let Some(accepted_at) = attempt.launch_accepted_at.as_deref() {
        let accepted = DateTime::parse_from_rfc3339(accepted_at)
            .map_err(|_| "Native setup attempt has an invalid launch timestamp")?;
        if accepted < requested || settled < accepted {
            return Err("Native setup attempt has contradictory launch timestamps".into());
        }
    }
    Ok(())
}

pub(crate) struct NativeProfileTauriState {
    service: NativeProfileService,
}
impl NativeProfileTauriState {
    pub(crate) fn new(service: NativeProfileService) -> Self {
        Self { service }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RegisterNativeProfileInput {
    home_path: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativeProfileIdInput {
    profile_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativeExecutionModeInput {
    profile_id: String,
    mode: ExecutionMode,
}

#[tauri::command]
pub(crate) fn load_native_profile_query(
    state: State<'_, NativeProfileTauriState>,
) -> Result<NativeProfileQueryDto, String> {
    state.service.query()
}
#[tauri::command]
pub(crate) fn register_native_profile(
    state: State<'_, NativeProfileTauriState>,
    input: RegisterNativeProfileInput,
) -> Result<NativeProfileDto, String> {
    state.service.register_existing(&input.home_path)
}
#[tauri::command]
pub(crate) fn create_dedicated_native_profile(
    state: State<'_, NativeProfileTauriState>,
) -> Result<NativeProfileDto, String> {
    state.service.create_dedicated()
}
#[tauri::command]
pub(crate) fn select_native_profile(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.select(&input.profile_id)
}
#[tauri::command]
pub(crate) fn select_native_profile_execution_mode(
    state: State<'_, NativeProfileTauriState>,
    input: NativeExecutionModeInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .select_execution_mode(&input.profile_id, input.mode)
}
#[tauri::command]
pub(crate) fn authorize_native_profile_danger_full_access(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .authorize_danger_full_access(&input.profile_id)
}
#[tauri::command]
pub(crate) fn revoke_native_profile_danger_full_access(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.revoke_danger_full_access(&input.profile_id)
}
#[tauri::command]
pub(crate) fn request_native_profile_login(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.request_login(&input.profile_id)
}
#[tauri::command]
pub(crate) fn refresh_native_profile_readiness(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.refresh_readiness(&input.profile_id)
}
#[tauri::command]
pub(crate) fn request_native_profile_sandbox_initialization(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .request_sandbox_initialization(&input.profile_id)
}
#[tauri::command]
pub(crate) fn confirm_native_profile_sandbox_initialization(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .confirm_sandbox_initialization(&input.profile_id)
}
#[tauri::command]
pub(crate) fn verify_native_profile_preprovisioned_sandbox(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.verify_preprovisioned_sandbox(&input.profile_id)
}
#[tauri::command]
pub(crate) fn confirm_native_profile_preprovisioned_sandbox_adoption(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .confirm_preprovisioned_sandbox_adoption(&input.profile_id)
}
#[tauri::command]
pub(crate) fn run_native_profile_workspace_write_canary(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.run_workspace_write_canary(&input.profile_id)
}
#[tauri::command]
pub(crate) fn run_native_profile_danger_full_access_canary(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.run_danger_full_access_canary(&input.profile_id)
}
#[tauri::command]
pub(crate) fn probe_native_profile_mcp_reporting(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.probe_mcp_reporting(&input.profile_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::thread;

    struct FakeChild {
        result: Option<NativeCliReceipt>,
        terminated: Arc<Mutex<usize>>,
    }
    impl NativeCliChild for FakeChild {
        fn try_wait(&mut self) -> Result<Option<NativeCliReceipt>, String> {
            Ok(self.result.take())
        }
        fn terminate(&mut self) -> Result<(), String> {
            self.result = Some(NativeCliReceipt {
                succeeded: false,
                exit_code: None,
                sandbox_receipt_observed: false,
            });
            *self.terminated.lock().unwrap() += 1;
            Ok(())
        }
    }
    struct FakeCli {
        receipt: NativeCliReceipt,
        calls: Mutex<Vec<NativeCliInvocation>>,
        starts: Mutex<usize>,
        terminated: Arc<Mutex<usize>>,
        next_child_result: Mutex<Option<NativeCliReceipt>>,
        start_error: bool,
        surface_supported: bool,
        workspace_sandbox_supported: bool,
        windows_sandbox_setup_supported: bool,
        workspace_launch_flags_supported: bool,
        workspace_launch_project_config_isolated: bool,
        danger_network_enforcement_supported: bool,
    }
    impl FakeCli {
        fn succeeding() -> Self {
            Self {
                receipt: NativeCliReceipt {
                    succeeded: true,
                    exit_code: Some(0),
                    sandbox_receipt_observed: true,
                },
                calls: Mutex::new(vec![]),
                starts: Mutex::new(0),
                terminated: Arc::new(Mutex::new(0)),
                next_child_result: Mutex::new(None),
                start_error: false,
                surface_supported: true,
                workspace_sandbox_supported: true,
                windows_sandbox_setup_supported: true,
                workspace_launch_flags_supported: true,
                workspace_launch_project_config_isolated: false,
                danger_network_enforcement_supported: false,
            }
        }

        fn unsupported_surface() -> Self {
            Self {
                surface_supported: false,
                ..Self::succeeding()
            }
        }

        fn unsupported_workspace_policy() -> Self {
            Self {
                workspace_sandbox_supported: false,
                ..Self::succeeding()
            }
        }

        fn unsupported_windows_sandbox_setup() -> Self {
            Self {
                windows_sandbox_setup_supported: false,
                ..Self::succeeding()
            }
        }

        fn enforcing_application_network_policy() -> Self {
            Self {
                danger_network_enforcement_supported: true,
                ..Self::succeeding()
            }
        }

        fn failing_start() -> Self {
            Self {
                start_error: true,
                ..Self::succeeding()
            }
        }
    }
    impl NativeCliPort for FakeCli {
        fn run(&self, invocation: &NativeCliInvocation) -> Result<NativeCliReceipt, String> {
            self.calls.lock().unwrap().push(invocation.clone());
            Ok(self.receipt)
        }
        fn start(
            &self,
            invocation: &NativeCliInvocation,
        ) -> Result<Box<dyn NativeCliChild>, String> {
            self.calls.lock().unwrap().push(invocation.clone());
            *self.starts.lock().unwrap() += 1;
            if self.start_error {
                return Err("launch failed".into());
            }
            Ok(Box::new(FakeChild {
                result: self.next_child_result.lock().unwrap().take(),
                terminated: self.terminated.clone(),
            }))
        }
        fn surface(&self) -> Result<NativeCliSurface, String> {
            if !self.surface_supported {
                return Err("unsupported".into());
            }
            Ok(NativeCliSurface {
                provenance: NativeCliProvenance {
                    executable: "C:/application-owned/codex.exe".into(),
                    version: "codex-cli test".into(),
                    workspace_sandbox_supported: self.workspace_sandbox_supported,
                    danger_full_access_supported: true,
                    danger_network_enforcement_supported: self.danger_network_enforcement_supported,
                    non_interactive_approval_supported: true,
                },
                windows_sandbox_setup_supported: self.windows_sandbox_setup_supported,
                workspace_launch_flags_supported: self.workspace_launch_flags_supported,
                workspace_launch_project_config_isolated: self
                    .workspace_launch_project_config_isolated,
            })
        }
    }

    fn service() -> (tempfile::TempDir, NativeProfileService) {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        service.cli = Arc::new(FakeCli::succeeding());
        (directory, service)
    }

    #[test]
    fn creates_selects_and_reopens_a_dedicated_profile_without_provider_state() {
        let (directory, service) = service();
        let created = service.create_dedicated().unwrap();
        assert_eq!(created.ownership, Ownership::ApplicationDedicated);
        let selected = service.select(&created.id).unwrap();
        assert!(selected.selected);
        drop(service);
        let reopened = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let query = reopened.query().unwrap();
        assert_eq!(query.contract, PROFILE_QUERY_CONTRACT);
        assert_eq!(query.profiles.len(), 1);
        assert!(query.profiles[0].selected);
        assert!(reopened.resolve_selected_home().is_err());
    }

    #[test]
    fn registration_rejects_relative_and_application_owned_homes() {
        let (_directory, service) = service();
        assert!(service.register_existing("relative").is_err());
        let dedicated = service.create_dedicated().unwrap();
        assert!(service.register_existing(&dedicated.home_path).is_err());
    }

    #[test]
    fn replacement_and_malformed_marker_fail_closed() {
        let (_directory, service) = service();
        let dedicated = service.create_dedicated().unwrap();
        service.select(&dedicated.id).unwrap();
        let before = service.profile(&dedicated.id).unwrap();
        service.update_readiness(
            &dedicated.id,
            Some("authenticated"),
            Some("initialized"),
            Some("passed"),
            Some("ready"),
            None,
        ).unwrap();
        let now = Utc::now().to_rfc3339();
        let connection = service.connection().unwrap();
        connection.execute(
            "INSERT INTO native_codex_profile_mode_authorizations (profile_id,mode,filesystem_identity,authorized_at) VALUES (?1,'danger_full_access',?2,?3)",
            params![dedicated.id, before.identity, now],
        ).unwrap();
        connection.execute(
            "INSERT INTO native_codex_profile_sandbox_adoptions (profile_id,filesystem_identity,executable,version,workspace_sandbox_supported,windows_sandbox_setup_supported,correlation_id,observed_at,state,elevated_mode_observed) VALUES (?1,?2,'C:/codex.exe','codex-cli test',1,1,'native-adoption-observation',?3,'verified',1)",
            params![dedicated.id, before.identity, now],
        ).unwrap();
        connection.execute(
            "INSERT INTO native_codex_profile_sandbox_adoption_confirmations (profile_id,filesystem_identity,adoption_correlation_id,confirmation_correlation_id,confirmed_at,state,invalidated_at) VALUES (?1,?2,'native-adoption-observation','native-adoption-confirmation',?3,'confirmed',NULL)",
            params![dedicated.id, before.identity, now],
        ).unwrap();
        fs::write(
            Path::new(&dedicated.home_path).join(MARKER_FILE),
            b"malformed",
        )
        .unwrap();
        assert!(service.select(&dedicated.id).is_err());
        let query = service.query().unwrap();
        let profile = &query.profiles[0];
        assert_eq!(profile.lifecycle, Lifecycle::Malformed);
        assert!(!profile.selected);
        assert!(!profile.execution.danger_full_access_authorized);
        assert_eq!(profile.readiness.authentication, "unknown");
        assert_eq!(profile.readiness.sandbox_initialization, "unknown");
        assert_eq!(profile.readiness.workspace_write_canary, "not_run");
        assert_eq!(profile.sandbox_adoption.disposition, "invalidated");
        assert_eq!(profile.sandbox_adoption_confirmation.disposition, "invalidated");
    }

    #[test]
    fn readiness_facts_do_not_imply_consumer_resolution() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service
            .update_readiness(
                &profile.id,
                Some("authenticated"),
                Some("attention_required"),
                Some("blocked"),
                Some("probe_failed"),
                Some((
                    "sandbox",
                    Some("sandbox_probe_failed_or_uac_attention_required"),
                )),
            )
            .unwrap();
        assert!(service.resolve_selected_home().is_err());
    }

    #[test]
    fn readiness_requires_every_fact_before_the_resolver_exposes_a_home() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service
            .update_readiness(
                &profile.id,
                Some("authenticated"),
                Some("initialized"),
                Some("passed"),
                Some("ready"),
                None,
            )
            .unwrap();
        assert_eq!(
            service.resolve_selected_home().unwrap().home,
            PathBuf::from(profile.home_path)
        );
    }

    #[test]
    fn setup_retries_are_idempotent_and_migration_adds_the_profile_tables() {
        let (directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        let first = service.request_sandbox_initialization(&profile.id).unwrap();
        let second = service.request_sandbox_initialization(&profile.id).unwrap();
        assert_eq!(first.readiness, second.readiness);
        assert_eq!(
            service
                .run_workspace_write_canary(&profile.id)
                .unwrap()
                .readiness
                .workspace_write_canary,
            "blocked"
        );

        let connection =
            crate::storage::open_active_database(&directory.path().join("migration.sqlite"))
                .unwrap();
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
    fn sandbox_setup_requires_explicit_uac_confirmation_before_the_canary_can_start() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
            succeeded: true,
            exit_code: Some(0),
            sandbox_receipt_observed: false,
        });
        service.request_sandbox_initialization(&profile.id).unwrap();
        assert!(!service.probe_root(&profile.id).exists());
        let mut query = service.query().unwrap();
        let awaiting_confirmation = query.profiles.remove(0);
        assert_eq!(
            awaiting_confirmation.readiness.sandbox_initialization,
            "attention_required"
        );
        assert_eq!(
            awaiting_confirmation.readiness.attentions.sandbox,
            Some("native_sandbox_setup_completed_explicit_uac_confirmation_required".into())
        );
        assert_eq!(
            service
                .run_workspace_write_canary(&profile.id)
                .unwrap()
                .readiness
                .workspace_write_canary,
            "blocked"
        );
        service.confirm_sandbox_initialization(&profile.id).unwrap();
        *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
            succeeded: true,
            exit_code: Some(0),
            sandbox_receipt_observed: true,
        });
        service.run_workspace_write_canary(&profile.id).unwrap();
        let mut query = service.query().unwrap();
        let canaried = query.profiles.remove(0);
        assert_eq!(canaried.readiness.workspace_write_canary, "passed");
        assert_eq!(canaried.setup_attempt.phase, "workspace_write_canary");
        assert_eq!(canaried.setup_attempt.disposition, "terminal_succeeded");
        assert_eq!(canaried.setup_attempt.terminal_exit_code, Some(0));

        let calls = fake.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(
            calls[0].args,
            vec![
                "sandbox",
                "setup",
                "--elevated",
                "--current-user",
                "--codex-home",
                profile.home_path.as_str(),
            ]
        );
        assert_eq!(calls[0].cwd, PathBuf::from(&profile.home_path));
        assert_eq!(calls[1].args[0], "sandbox");
        assert_eq!(calls[1].args[1], "-P");
        assert_eq!(calls[1].args[2], ":workspace");
        assert_eq!(calls[1].args[3], "-C");
        assert_eq!(calls[1].args[4], service.probe_root(&profile.id).to_string_lossy());
        assert_eq!(
            calls[1].args,
            vec![
                "sandbox".into(),
                "-P".into(),
                ":workspace".into(),
                "-C".into(),
                service.probe_root(&profile.id).to_string_lossy().into_owned(),
                "--".into(),
                "cmd.exe".into(),
                "/d".into(),
                "/c".into(),
                ".\\native-codex-profile-canary.cmd".into(),
            ]
        );
        assert_eq!(calls[1].cwd, service.probe_root(&profile.id));
        for call in calls.iter() {
            assert_eq!(
                call.environment,
                native_windows_cli_environment(Path::new(&profile.home_path))
            );
            assert!(!call
                .args
                .iter()
                .any(|argument| argument.contains("state-json")));
            assert!(!call
                .args
                .iter()
                .any(|argument| argument == "--include-managed-config" || argument == "--add-dir"));
            assert!(!call
                .args
                .iter()
                .any(|argument| argument.contains("dangerously")));
        }
        assert!(calls[0].sandbox_receipt.is_none());
        assert!(calls[1].sandbox_receipt.is_some());
        assert!(!service
            .probe_root(&profile.id)
            .join(WORKSPACE_WRITE_CANARY_COMMAND_FILE)
            .exists());
    }

    #[test]
    fn workspace_canary_receipt_absence_is_durable_and_separate_from_exit_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service
            .update_readiness(
                &profile.id,
                None,
                Some("initialized"),
                Some("not_run"),
                None,
                None,
            )
            .unwrap();
        *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
            succeeded: false,
            exit_code: Some(1),
            sandbox_receipt_observed: false,
        });
        let stale_receipt = service
            .probe_root(&profile.id)
            .join("workspace_write_canary.txt");
        fs::create_dir_all(stale_receipt.parent().unwrap()).unwrap();
        fs::write(&stale_receipt, "native-codex-profile-canary").unwrap();

        service.run_workspace_write_canary(&profile.id).unwrap();

        let profile = service.query().unwrap().profiles.remove(0);
        assert_eq!(profile.readiness.workspace_write_canary, "blocked");
        assert_eq!(profile.setup_attempt.phase, "workspace_write_canary");
        assert_eq!(profile.setup_attempt.disposition, "terminal_failed");
        assert_eq!(profile.setup_attempt.terminal_classification, "receipt_missing");
        assert_eq!(profile.setup_attempt.terminal_exit_code, Some(1));
        assert!(!stale_receipt.exists());
    }

    #[test]
    fn contradictory_receipt_missing_attempt_fails_closed_before_profile_reconciliation() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service
            .update_readiness(
                &profile.id,
                None,
                Some("initialized"),
                Some("not_run"),
                None,
                None,
            )
            .unwrap();
        *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
            succeeded: false,
            exit_code: Some(1),
            sandbox_receipt_observed: false,
        });
        service.run_workspace_write_canary(&profile.id).unwrap();
        assert_eq!(
            service.query().unwrap().profiles[0].setup_attempt.terminal_classification,
            "receipt_missing"
        );
        let connection = service.connection().unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        assert_eq!(
            connection
                .execute(
                    "UPDATE native_codex_profile_setup_attempts SET phase='sandbox_initialization' WHERE profile_id=?1",
                    params![profile.id],
                )
                .unwrap(),
            1
        );
        connection
            .execute_batch("PRAGMA ignore_check_constraints=OFF;")
            .unwrap();
        assert!(load_profiles(&mut service.connection().unwrap()).is_err());
    }

    #[test]
    fn windows_semantic_capability_detection_accepts_combined_setup_help_and_rejects_drift() {
        assert_eq!(
            windows_semantic_sandbox_capabilities(
                "Options: --permission-profile <NAME> --cd <DIR>",
                "Error: --current-user required\nOptions: --elevated --current-user --codex-home <DIR>",
            ),
            (true, true)
        );
        assert_eq!(
            windows_semantic_sandbox_capabilities(
                "Options: --permission-profile <NAME>",
                "Options: --elevated --current-user --codex-home <DIR>",
            ),
            (false, true)
        );
        assert_eq!(
            windows_semantic_sandbox_capabilities(
                "Options: --permission-profile <NAME> --cd <DIR>",
                "Options: --elevated --current-user",
            ),
            (true, false)
        );
    }

    #[test]
    fn workspace_launch_capability_requires_every_known_flag_but_stays_unavailable_without_project_isolation() {
        assert_eq!(
            workspace_launch_semantic_capabilities(
                "--json --ignore-user-config --ignore-rules --config <KEY=VALUE> --sandbox <MODE> --cd <DIR> --skip-git-repo-check workspace-write",
            ),
            (true, false)
        );
        assert_eq!(
            workspace_launch_semantic_capabilities(
                "--json --ignore-user-config --ignore-rules --config <KEY=VALUE> --sandbox <MODE> --skip-git-repo-check workspace-write",
            ),
            (false, false)
        );
    }

    #[test]
    fn unsupported_workspace_policy_is_durable_and_blocks_children_and_uac() {
        let (directory, mut service) = service();
        let fake = Arc::new(FakeCli::unsupported_workspace_policy());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();

        let requested = service.request_sandbox_initialization(&profile.id).unwrap();
        assert_eq!(requested.setup_attempt.disposition, "policy_unsupported");
        assert_eq!(
            requested.setup_attempt.terminal_classification,
            "policy_unsupported"
        );
        assert_eq!(requested.setup_attempt.workspace_sandbox_supported, Some(false));
        assert_eq!(
            requested.setup_attempt.executable.as_deref(),
            Some("C:/application-owned/codex.exe")
        );
        assert_eq!(requested.setup_attempt.version.as_deref(), Some("codex-cli test"));
        assert!(requested.setup_attempt.correlation_id.is_some());
        assert!(requested.setup_attempt.requested_at.is_some());
        assert!(requested.setup_attempt.deadline_at.is_some());
        assert!(requested.setup_attempt.launch_accepted_at.is_none());
        assert!(requested.setup_attempt.settled_at.is_some());
        assert!(requested.setup_attempt.terminal_exit_code.is_none());
        assert_eq!(*fake.starts.lock().unwrap(), 0);
        assert_eq!(
            requested.readiness.attentions.sandbox,
            Some("native_sandbox_semantic_policy_unsupported".into())
        );
        assert!(service.confirm_sandbox_initialization(&profile.id).is_err());
        assert_eq!(
            service
                .run_workspace_write_canary(&profile.id)
                .unwrap()
                .readiness
                .workspace_write_canary,
            "blocked"
        );
        assert_eq!(*fake.starts.lock().unwrap(), 0);
        assert!(!directory.path().join("app").join("probes").exists());
    }

    #[test]
    fn missing_windows_setup_capability_fails_closed_before_a_child_or_probe_root() {
        let (directory, mut service) = service();
        let fake = Arc::new(FakeCli::unsupported_windows_sandbox_setup());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();

        let requested = service.request_sandbox_initialization(&profile.id).unwrap();
        assert_eq!(requested.setup_attempt.disposition, "policy_unsupported");
        assert_eq!(requested.setup_attempt.workspace_sandbox_supported, Some(false));
        assert_eq!(*fake.starts.lock().unwrap(), 0);
        assert!(!directory.path().join("app").join("probes").exists());
    }

    #[test]
    fn policy_unsupported_attempts_reject_contradictory_storage_and_query_facts() {
        let (_directory, mut service) = service();
        service.cli = Arc::new(FakeCli::unsupported_workspace_policy());
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service.request_sandbox_initialization(&profile.id).unwrap();
        let connection = service.connection().unwrap();
        for mutation in [
            "phase='not_requested'",
            "launch_accepted_at='2026-08-07T12:00:01Z'",
            "workspace_sandbox_supported=1",
            "terminal_exit_code=1",
            "terminal_classification='exit_code'",
            "executable=NULL",
            "executable=''",
            "version=NULL",
            "version=''",
            "correlation_id=''",
            "requested_at=''",
            "deadline_at=''",
            "settled_at=NULL",
            "settled_at=''",
        ] {
            assert!(connection
                .execute(
                    &format!("UPDATE native_codex_profile_setup_attempts SET {mutation} WHERE profile_id=?1"),
                    params![profile.id],
                )
                .is_err());
        }
        connection
            .execute_batch("PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        connection
            .execute(
                "UPDATE native_codex_profile_setup_attempts SET launch_accepted_at='2026-08-07T12:00:01Z' WHERE profile_id=?1",
                params![profile.id],
            )
            .unwrap();
        connection
            .execute_batch("PRAGMA ignore_check_constraints=OFF;")
            .unwrap();
        assert!(service.query().is_err());
    }

    #[test]
    fn setup_attempt_persists_safe_provenance_then_classifies_terminal_failure() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
            succeeded: false,
            exit_code: Some(7),
            sandbox_receipt_observed: false,
        });
        service.cli = fake;
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        let requested = service.request_sandbox_initialization(&profile.id).unwrap();
        assert_eq!(requested.setup_attempt.phase, "sandbox_initialization");
        assert_eq!(requested.setup_attempt.disposition, "pending");
        assert_eq!(
            requested.setup_attempt.executable.as_deref(),
            Some("C:/application-owned/codex.exe")
        );
        assert_eq!(requested.setup_attempt.version.as_deref(), Some("codex-cli test"));
        assert_eq!(requested.setup_attempt.workspace_sandbox_supported, Some(true));
        assert!(requested.setup_attempt.correlation_id.is_some());
        assert!(requested.setup_attempt.requested_at.is_some());
        assert!(requested.setup_attempt.launch_accepted_at.is_some());
        assert!(requested.setup_attempt.settled_at.is_none());

        let settled = service.query().unwrap().profiles.remove(0);
        assert_eq!(settled.setup_attempt.disposition, "terminal_failed");
        assert_eq!(settled.setup_attempt.terminal_classification, "exit_code");
        assert_eq!(settled.setup_attempt.terminal_exit_code, Some(7));
        assert_eq!(settled.readiness.sandbox_initialization, "attention_required");
        assert!(service.confirm_sandbox_initialization(&profile.id).is_err());
    }

    #[test]
    fn setup_launch_failure_is_durable_without_uac_or_initialization_claims() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        service.cli = Arc::new(FakeCli::failing_start());
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        let result = service.request_sandbox_initialization(&profile.id).unwrap();
        assert_eq!(result.setup_attempt.disposition, "launch_failed");
        assert_eq!(result.setup_attempt.terminal_classification, "launch_failed");
        assert!(result.setup_attempt.launch_accepted_at.is_none());
        assert_eq!(result.readiness.sandbox_initialization, "attention_required");
        assert_ne!(
            result.readiness.attentions.sandbox.as_deref(),
            Some("native_sandbox_setup_completed_explicit_uac_confirmation_required")
        );
        assert!(service.confirm_sandbox_initialization(&profile.id).is_err());
    }

    #[test]
    fn pending_sandbox_attempts_are_reused_then_timeout_without_launch_success() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service.request_sandbox_initialization(&profile.id).unwrap();
        service.request_sandbox_initialization(&profile.id).unwrap();
        assert_eq!(*fake.starts.lock().unwrap(), 1);
        let connection = service.connection().unwrap();
        connection
            .execute(
                "UPDATE native_codex_profile_setup_attempts SET deadline_at='2000-01-01T00:00:00+00:00' WHERE profile_id=?1",
                params![profile.id],
            )
            .unwrap();
        let result = service.query().unwrap();
        assert_eq!(
            result.profiles[0].readiness.sandbox_initialization,
            "attention_required"
        );
        assert_eq!(
            result.profiles[0].readiness.attentions.sandbox,
            Some("native_sandbox_attempt_timed_out_human_or_uac_attention".into())
        );
        assert_eq!(*fake.terminated.lock().unwrap(), 1);
        assert_eq!(result.profiles[0].setup_attempt.disposition, "timed_out");
        assert_eq!(result.profiles[0].setup_attempt.terminal_classification, "timed_out");
    }

    #[test]
    fn reopened_pending_setup_is_recovered_without_an_owned_child_to_observe() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("active.sqlite");
        let mut service =
            NativeProfileService::open(database.clone(), directory.path().join("app")).unwrap();
        service.cli = Arc::new(FakeCli::succeeding());
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service.request_sandbox_initialization(&profile.id).unwrap();
        drop(service);
        let reopened = NativeProfileService::open(database, directory.path().join("app")).unwrap();
        let query = reopened.query().unwrap();
        assert_eq!(
            query.profiles[0].readiness.sandbox_initialization,
            "attention_required"
        );
        assert_eq!(
            query.profiles[0].readiness.attentions.sandbox,
            Some("native_sandbox_attempt_recovered_without_owned_process".into())
        );
        assert_eq!(
            query.profiles[0].setup_attempt.disposition,
            "recovered_unobserved"
        );
    }

    #[test]
    fn canary_before_observed_initialization_never_starts_a_child() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        assert!(service.confirm_sandbox_initialization(&profile.id).is_err());
        let blocked = service.run_workspace_write_canary(&profile.id).unwrap();
        assert_eq!(blocked.readiness.workspace_write_canary, "blocked");
        assert_eq!(*fake.starts.lock().unwrap(), 0);
        assert_eq!(
            blocked.readiness.attentions.canary,
            Some("workspace_write_canary_requires_observed_sandbox_initialization".into())
        );
    }

    #[test]
    fn setup_confirmation_and_canary_require_the_currently_selected_profile() {
        let (directory, mut service) = service();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let selected = service.create_dedicated().unwrap();
        let unselected = service.create_dedicated().unwrap();
        service.select(&selected.id).unwrap();

        assert!(service.request_sandbox_initialization(&unselected.id).is_err());
        assert!(service.confirm_sandbox_initialization(&unselected.id).is_err());
        assert!(service.run_workspace_write_canary(&unselected.id).is_err());
        assert_eq!(*fake.starts.lock().unwrap(), 0);
        assert!(!service.probe_root(&unselected.id).exists());
        assert!(!directory.path().join("app").join("probes").exists());
    }

    #[test]
    fn switching_selection_cancels_only_the_previous_profile_owned_setup_attempt() {
        let (_directory, mut service) = service();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let first = service.create_dedicated().unwrap();
        let second = service.create_dedicated().unwrap();
        service.select(&first.id).unwrap();
        service.request_sandbox_initialization(&first.id).unwrap();

        service.select(&second.id).unwrap();
        assert_eq!(*fake.terminated.lock().unwrap(), 1);
        assert_eq!(
            service.profile(&first.id).unwrap().setup_attempt.disposition,
            "cancelled"
        );
        assert!(service.confirm_sandbox_initialization(&first.id).is_err());

        service.select(&first.id).unwrap();
        assert!(service.confirm_sandbox_initialization(&first.id).is_err());
        assert_eq!(
            service.profile(&first.id).unwrap().setup_attempt.disposition,
            "cancelled"
        );
    }

    #[test]
    fn switching_selection_cancels_the_previous_profile_owned_workspace_canary() {
        let (_directory, mut service) = service();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let first = service.create_dedicated().unwrap();
        let second = service.create_dedicated().unwrap();
        service.select(&first.id).unwrap();
        service
            .update_readiness(&first.id, None, Some("initialized"), None, None, None)
            .unwrap();
        service.run_workspace_write_canary(&first.id).unwrap();

        service.select(&second.id).unwrap();
        assert_eq!(*fake.terminated.lock().unwrap(), 1);
        assert_eq!(
            service.profile(&first.id).unwrap().setup_attempt.disposition,
            "cancelled"
        );
        assert_eq!(
            service
                .profile(&first.id)
                .unwrap()
                .readiness
                .workspace_write_canary,
            "blocked"
        );
    }

    #[test]
    fn externally_provisioned_sandbox_is_adopted_without_a_setup_attempt_or_uac_claim() {
        let (directory, mut service) = service();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        fs::write(Path::new(&profile.home_path).join("config.toml"), "[windows]\nsandbox = \"elevated\"\n").unwrap();

        let verified = service.verify_preprovisioned_sandbox(&profile.id).unwrap();
        assert_eq!(verified.sandbox_adoption.disposition, "verified");
        assert_eq!(verified.setup_attempt.disposition, "not_requested");
        assert!(verified.sandbox_adoption.observed_at.is_some());
        assert!(verified.sandbox_adoption.correlation_id.is_some());
        assert_eq!(verified.readiness.sandbox_initialization, "attention_required");
        assert_eq!(*fake.starts.lock().unwrap(), 0);
        assert!(service.connection().unwrap().execute(
            "UPDATE native_codex_profile_sandbox_adoptions SET elevated_mode_observed=0 WHERE profile_id=?1",
            params![profile.id],
        ).is_err());

        let confirmed = service.confirm_preprovisioned_sandbox_adoption(&profile.id).unwrap();
        assert_eq!(confirmed.sandbox_adoption_confirmation.disposition, "confirmed");
        assert!(confirmed.sandbox_adoption_confirmation.confirmed_at.is_some());
        assert!(confirmed.sandbox_adoption_confirmation.correlation_id.is_some());
        assert_eq!(confirmed.readiness.sandbox_initialization, "initialized");
        assert_eq!(confirmed.readiness.attentions.sandbox, Some("external_sandbox_adoption_confirmed_product_uac_unobserved".into()));
        assert_eq!(*fake.starts.lock().unwrap(), 0);
        drop(service);
        let mut reopened = NativeProfileService::open(directory.path().join("active.sqlite"), directory.path().join("app")).unwrap();
        reopened.cli = fake;
        let mut reopened_query = reopened.query().unwrap();
        let reopened_profile = reopened_query.profiles.remove(0);
        assert_eq!(reopened_profile.sandbox_adoption.disposition, "verified");
        assert_eq!(reopened_profile.sandbox_adoption_confirmation.disposition, "confirmed");
    }

    #[test]
    fn v31_and_v32_migrations_add_external_adoption_storage_without_setup_backfill() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("active.sqlite");
        let connection = crate::storage::open_active_database(&database).unwrap();
        connection.execute_batch("DROP TABLE native_codex_profile_sandbox_adoption_confirmations; DROP TABLE native_codex_profile_sandbox_adoptions; PRAGMA user_version=30;").unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        assert_eq!(connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0)).unwrap(), crate::storage::ACTIVE_SCHEMA_VERSION);
        assert_eq!(connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='native_codex_profile_sandbox_adoptions')", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
        assert_eq!(connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='native_codex_profile_sandbox_adoption_confirmations')", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
    }

    #[test]
    fn external_sandbox_adoption_rejects_unselected_and_invalidates_on_continuity_loss() {
        let (_directory, mut service) = service();
        service.cli = Arc::new(FakeCli::succeeding());
        let first = service.create_dedicated().unwrap();
        let second = service.create_dedicated().unwrap();
        service.select(&first.id).unwrap();
        fs::write(Path::new(&first.home_path).join("config.toml"), "[windows]\nsandbox = \"elevated\"\n").unwrap();
        assert!(service.verify_preprovisioned_sandbox(&second.id).is_err());
        service.verify_preprovisioned_sandbox(&first.id).unwrap();
        fs::remove_file(Path::new(&first.home_path).join(MARKER_FILE)).unwrap();
        service.query().unwrap();
        assert_eq!(service.profile(&first.id).unwrap().sandbox_adoption.disposition, "invalidated");
    }

    #[test]
    fn external_sandbox_adoption_confirmation_is_invalidated_by_selection_and_observation_drift() {
        let (_directory, mut service) = service();
        service.cli = Arc::new(FakeCli::succeeding());
        let first = service.create_dedicated().unwrap();
        let second = service.create_dedicated().unwrap();
        service.select(&first.id).unwrap();
        let config = Path::new(&first.home_path).join("config.toml");
        fs::write(&config, "[windows]\nsandbox = \"elevated\"\n").unwrap();
        service.verify_preprovisioned_sandbox(&first.id).unwrap();
        service.confirm_preprovisioned_sandbox_adoption(&first.id).unwrap();

        service.select(&second.id).unwrap();
        let invalidated = service.profile(&first.id).unwrap();
        assert_eq!(invalidated.sandbox_adoption.disposition, "invalidated");
        assert_eq!(invalidated.sandbox_adoption_confirmation.disposition, "invalidated");
        assert_eq!(invalidated.readiness.sandbox_initialization, "attention_required");
        assert_eq!(invalidated.readiness.workspace_write_canary, "blocked");

        service.select(&first.id).unwrap();
        service.verify_preprovisioned_sandbox(&first.id).unwrap();
        service.confirm_preprovisioned_sandbox_adoption(&first.id).unwrap();
        fs::remove_file(config).unwrap();
        let drifted = service.query().unwrap().profiles.into_iter().find(|profile| profile.id == first.id).unwrap();
        assert_eq!(drifted.sandbox_adoption.disposition, "invalidated");
        assert_eq!(drifted.sandbox_adoption_confirmation.disposition, "invalidated");
        assert_eq!(drifted.readiness.sandbox_initialization, "attention_required");
    }

    #[test]
    fn elevated_sandbox_observation_rejects_duplicate_windows_assignments() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("config.toml"), "[windows]\nsandbox = \"elevated\"\nsandbox = \"elevated\"\n").unwrap();
        assert!(observe_elevated_windows_sandbox_mode(directory.path()).is_err());
        fs::write(directory.path().join("config.toml"), "[windows]\nsandbox = \"elevated\"\n[windows]\n").unwrap();
        assert!(observe_elevated_windows_sandbox_mode(directory.path()).is_err());
    }

    #[test]
    fn browser_login_is_idempotently_supervised_and_owned_children_are_reaped_on_shutdown() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service.request_login(&profile.id).unwrap();
        service.request_login(&profile.id).unwrap();
        assert_eq!(*fake.starts.lock().unwrap(), 1);
        assert_eq!(
            service
                .profile(&profile.id)
                .unwrap()
                .readiness
                .attentions
                .authentication,
            Some("browser_login_attempt_pending".into())
        );
        drop(service);
        assert_eq!(*fake.terminated.lock().unwrap(), 1);
    }

    #[test]
    fn windows_login_environment_is_allowlisted_and_keeps_the_product_selected_home() {
        let environment = native_windows_cli_environment_from(Path::new("C:/product-owned-home"), &|key| {
            match key {
                "PATH" => Some("C:/Windows/System32;C:/Windows".into()),
                "SYSTEMROOT" => Some("C:/Windows".into()),
                "USERPROFILE" => Some("C:/Users/launching-user".into()),
                "CODEX_HOME" => Some("C:/foreign-home".into()),
                "UNRELATED_SECRET" => Some("must-not-pass".into()),
                _ => None,
            }
        });
        assert!(environment.contains(&("CODEX_HOME".into(), "C:/product-owned-home".into())));
        assert!(environment.contains(&("PATH".into(), "C:/Windows/System32;C:/Windows".into())));
        assert!(environment.contains(&("SYSTEMROOT".into(), "C:/Windows".into())));
        assert!(environment.contains(&("USERPROFILE".into(), "C:/Users/launching-user".into())));
        assert!(!environment.iter().any(|(key, _)| key == "UNRELATED_SECRET"));
        assert!(!environment.iter().any(|(_, value)| value == "C:/foreign-home"));
    }

    #[cfg(windows)]
    #[test]
    fn system_cli_port_drains_windows_stdio_and_observes_only_its_bound_receipt() {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join("selected-home");
        let root = directory.path().join("application-owned-probe-root");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&root).unwrap();
        let receipt = root.join("workspace_write_canary.txt");
        let other = root.join("not-the-designated-receipt.txt");
        let script = root.join("bound-system-cli-port.cmd");
        let other_script = root.join("unbound-system-cli-port.cmd");
        fs::write(
            &script,
            format!(
                "@echo off\r\nif /i not \"%CODEX_HOME%\"==\"{}\" exit /b 2\r\nif /i not \"%CD%\"==\"{}\" exit /b 3\r\necho native-codex-profile-canary>\"{}\"\r\necho discarded-stderr>&2\r\n",
                home.to_string_lossy(),
                root.to_string_lossy(),
                receipt.to_string_lossy(),
            ),
        )
        .unwrap();
        fs::write(
            &other_script,
            format!(
                "@echo off\r\necho native-codex-profile-canary>\"{}\"\r\nexit /b 0\r\n",
                other.to_string_lossy(),
            ),
        )
        .unwrap();
        let command = std::env::var("COMSPEC").unwrap();
        let port = SystemNativeCliPort {
            program: Ok(command),
        };
        let invocation = NativeCliInvocation {
            args: vec![
                "/d".into(),
                "/s".into(),
                "/c".into(),
                script.to_string_lossy().into_owned(),
            ],
            cwd: root.clone(),
            codex_home: home.clone(),
            environment: native_windows_cli_environment(&home),
            sandbox_receipt: Some(receipt.clone()),
            sandbox_command_file: None,
        };
        let settled = port.run(&invocation).unwrap();
        assert_eq!(settled.exit_code, Some(0));
        assert!(settled.succeeded);
        assert!(settled.sandbox_receipt_observed);

        let unbound = NativeCliInvocation {
            args: vec![
                "/d".into(),
                "/s".into(),
                "/c".into(),
                other_script.to_string_lossy().into_owned(),
            ],
            sandbox_receipt: Some(root.join("different-designated-receipt.txt")),
            ..invocation
        };
        let settled = port.run(&unbound).unwrap();
        assert!(settled.succeeded);
        assert!(!settled.sandbox_receipt_observed);
        assert!(other.exists());
    }

    #[cfg(windows)]
    #[test]
    fn system_cli_port_forwards_a_workspace_command_file_without_receipt_path_quoting() {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join("selected-home");
        let root = directory.path().join("application owned probe root");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&root).unwrap();
        let receipt = root.join("workspace_write_canary.txt");
        let command = root.join("native-codex-profile-canary.cmd");
        fs::write(
            &command,
            "@echo off\r\necho native-codex-profile-canary>workspace_write_canary.txt\r\n",
        )
        .unwrap();
        let port = SystemNativeCliPort {
            program: Ok(std::env::var("COMSPEC").unwrap()),
        };
        let settled = port
            .run(&NativeCliInvocation {
                args: vec![
                    "/d".into(),
                    "/c".into(),
                    ".\\native-codex-profile-canary.cmd".into(),
                ],
                cwd: root,
                codex_home: home.clone(),
                environment: native_windows_cli_environment(&home),
                sandbox_receipt: Some(receipt),
                sandbox_command_file: Some(command.clone()),
            })
            .unwrap();

        assert!(settled.succeeded);
        assert!(settled.sandbox_receipt_observed);
        assert!(!command.exists());
    }

    #[cfg(windows)]
    #[test]
    fn system_cli_port_quote_bearing_workspace_payload_fails_when_the_probe_path_has_spaces() {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join("selected-home");
        let root = directory.path().join("application owned probe root");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&root).unwrap();
        let receipt = root.join("workspace_write_canary.txt");
        let port = SystemNativeCliPort {
            program: Ok(std::env::var("COMSPEC").unwrap()),
        };

        let settled = port
            .run(&NativeCliInvocation {
                args: vec![
                    "/d".into(),
                    "/s".into(),
                    "/c".into(),
                    format!("echo native-codex-profile-canary>\"{}\"", receipt.display()),
                ],
                cwd: root,
                codex_home: home.clone(),
                environment: native_windows_cli_environment(&home),
                sandbox_receipt: Some(receipt),
                sandbox_command_file: None,
            })
            .unwrap();

        assert!(!settled.succeeded);
        assert!(!settled.sandbox_receipt_observed);
    }

    #[test]
    fn login_and_status_use_the_allowlisted_windows_environment_without_a_real_browser() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service.request_login(&profile.id).unwrap();
        service.refresh_readiness(&profile.id).unwrap();
        let calls = fake.calls.lock().unwrap();
        for call in calls.iter().filter(|call| call.args.first().is_some_and(|arg| arg == "login")) {
            assert_eq!(call.environment, native_windows_cli_environment(Path::new(&profile.home_path)));
            assert!(call.environment.iter().any(|(key, _)| key == "CODEX_HOME"));
        }
    }

    #[test]
    fn browser_login_exit_requires_a_separate_status_observation() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
            succeeded: true,
            exit_code: Some(0),
            sandbox_receipt_observed: false,
        });
        service.cli = fake;
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service.request_login(&profile.id).unwrap();
        let mut query = service.query().unwrap();
        let after_exit = query.profiles.remove(0);
        assert_eq!(after_exit.readiness.authentication, "unknown");
        assert_eq!(
            after_exit.readiness.attentions.authentication,
            Some("browser_login_terminal_succeeded_browser_handoff_unobserved".into())
        );
        assert_eq!(after_exit.login_attempt.disposition, "terminal_succeeded");
        assert_eq!(after_exit.login_attempt.browser_handoff, "unobserved");
    }

    #[test]
    fn browser_login_launch_failure_is_durable_without_authentication_or_handoff_claims() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        service.cli = Arc::new(FakeCli::failing_start());
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        let result = service.request_login(&profile.id).unwrap();
        assert_eq!(result.login_attempt.disposition, "launch_failed");
        assert_eq!(result.login_attempt.browser_handoff, "unobserved");
        assert_eq!(result.readiness.authentication, "unknown");
        let persisted: (String, String, Option<String>) = service.connection().unwrap().query_row(
            "SELECT executable,version,launch_accepted_at FROM native_codex_profile_login_attempts WHERE profile_id=?1",
            params![profile.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert_eq!(persisted.0, "C:/application-owned/codex.exe");
        assert_eq!(persisted.1, "codex-cli test");
        assert!(persisted.2.is_none());
    }

    #[test]
    fn browser_login_terminal_failure_is_durable_and_does_not_claim_unauthenticated() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
            succeeded: false,
            exit_code: Some(1),
            sandbox_receipt_observed: false,
        });
        service.cli = fake;
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service.request_login(&profile.id).unwrap();
        let result = service.query().unwrap().profiles.remove(0);
        assert_eq!(result.login_attempt.disposition, "terminal_failed");
        assert_eq!(result.login_attempt.browser_handoff, "unobserved");
        assert_eq!(result.readiness.authentication, "unknown");
    }

    #[test]
    fn login_refresh_preserves_pending_attempt_creates_probe_root_and_reconciles_cli_attention() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service
            .set_attention(&profile.id, "cli", Some("codex_cli_unavailable"), false)
            .unwrap();
        let refreshed = service.refresh_readiness(&profile.id).unwrap();
        assert!(service.probe_root(&profile.id).is_dir());
        assert_eq!(refreshed.login_attempt.disposition, "not_requested");
        assert_eq!(refreshed.readiness.attentions.cli, None);
        service.request_login(&profile.id).unwrap();
        let refreshed = service.refresh_readiness(&profile.id).unwrap();
        assert_eq!(refreshed.login_attempt.disposition, "pending");
        assert_eq!(refreshed.login_attempt.browser_handoff, "unobserved");
        assert_eq!(refreshed.readiness.authentication, "authenticated");
        assert_eq!(
            refreshed.readiness.attentions.authentication,
            Some("browser_login_attempt_pending".into())
        );
        assert_eq!(
            fake.calls.lock().unwrap().last().unwrap().cwd,
            fs::canonicalize(service.probe_root(&profile.id)).unwrap()
        );
    }

    #[test]
    fn pending_login_reopen_recovers_without_claiming_process_or_browser_activity() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("active.sqlite");
        let mut service =
            NativeProfileService::open(database.clone(), directory.path().join("app")).unwrap();
        service.cli = Arc::new(FakeCli::succeeding());
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service.request_login(&profile.id).unwrap();
        drop(service);
        let reopened = NativeProfileService::open(database, directory.path().join("app")).unwrap();
        let profile = reopened.query().unwrap().profiles.remove(0);
        assert_eq!(profile.login_attempt.disposition, "recovered_unobserved");
        assert_eq!(profile.login_attempt.browser_handoff, "unobserved");
        assert_eq!(profile.readiness.authentication, "unknown");
    }

    #[test]
    fn login_operations_require_the_selected_validated_profile() {
        let (_directory, mut service) = service();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        assert!(service.request_login(&profile.id).is_err());
        assert!(service.refresh_readiness(&profile.id).is_err());
        assert_eq!(*fake.starts.lock().unwrap(), 0);
        assert!(fake.calls.lock().unwrap().is_empty());
        service.select(&profile.id).unwrap();
        fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
        assert!(service.request_login(&profile.id).is_err());
    }

    #[test]
    fn query_revalidation_clears_selection_and_invalidates_readiness() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service
            .update_readiness(
                &profile.id,
                Some("authenticated"),
                Some("initialized"),
                Some("passed"),
                Some("ready"),
                None,
            )
            .unwrap();
        fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
        let query = service.query().unwrap();
        assert!(!query.profiles[0].selected);
        assert_eq!(query.profiles[0].lifecycle, Lifecycle::Malformed);
        assert_eq!(query.profiles[0].readiness.authentication, "unknown");
        assert_eq!(
            query.profiles[0].readiness.workspace_write_canary,
            "not_run"
        );
    }

    #[test]
    fn danger_full_access_is_explicit_durable_and_revocable_per_profile_identity() {
        let (directory, service) = service();
        let first = service.create_dedicated().unwrap();
        let second = service.create_dedicated().unwrap();
        service.select(&first.id).unwrap();
        service
            .select_execution_mode(&first.id, ExecutionMode::DangerFullAccess)
            .unwrap();
        assert!(
            !service
                .profile(&first.id)
                .unwrap()
                .execution
                .danger_full_access_authorized
        );
        service.authorize_danger_full_access(&first.id).unwrap();
        assert!(
            service
                .profile(&first.id)
                .unwrap()
                .execution
                .danger_full_access_authorized
        );
        assert!(
            !service
                .profile(&second.id)
                .unwrap()
                .execution
                .danger_full_access_authorized
        );
        drop(service);
        let reopened = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        assert!(
            reopened
                .profile(&first.id)
                .unwrap()
                .execution
                .danger_full_access_authorized
        );
        reopened.revoke_danger_full_access(&first.id).unwrap();
        assert!(
            !reopened
                .profile(&first.id)
                .unwrap()
                .execution
                .danger_full_access_authorized
        );
    }

    #[test]
    fn danger_authority_fails_closed_when_identity_is_stale_or_continuity_is_lost() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service
            .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
            .unwrap();
        service.authorize_danger_full_access(&profile.id).unwrap();
        service
            .connection()
            .unwrap()
            .execute(
                "UPDATE native_codex_profile_mode_authorizations SET filesystem_identity='stale' WHERE profile_id=?1",
                params![profile.id],
            )
            .unwrap();
        assert!(
            !service
                .profile(&profile.id)
                .unwrap()
                .execution
                .danger_full_access_authorized
        );
        service.authorize_danger_full_access(&profile.id).unwrap();
        fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
        service.query().unwrap();
        let revoked: Option<String> = service.connection().unwrap().query_row(
            "SELECT revoked_at FROM native_codex_profile_mode_authorizations WHERE profile_id=?1 AND mode='danger_full_access'",
            params![profile.id], |row| row.get(0),
        ).unwrap();
        assert!(revoked.is_some());
    }

    #[test]
    fn workspace_launch_stays_action_incapable_without_project_config_isolation() {
        let (directory, mut service) = service();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        let root = directory.path().join("assigned-application-root");
        fs::create_dir_all(&root).unwrap();
        let workspace_target = NativeLaunchTarget::application_owned(root.clone(), true).unwrap();
        assert!(service
            .project_launch(&profile.id, &workspace_target)
            .is_err());
        service
            .update_readiness(
                &profile.id,
                None,
                Some("initialized"),
                Some("passed"),
                None,
                None,
            )
            .unwrap();
        assert!(service.project_launch(&profile.id, &workspace_target).is_err());
        assert_eq!(
            service.profile(&profile.id).unwrap().readiness.attentions.cli,
            Some("codex_cli_workspace_launch_project_config_unsupported".into())
        );
        service
            .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
            .unwrap();
        assert!(service
            .project_launch(&profile.id, &workspace_target)
            .is_err());
        service.authorize_danger_full_access(&profile.id).unwrap();
        service.cli = Arc::new(FakeCli::enforcing_application_network_policy());
        let danger = service
            .project_launch(&profile.id, &workspace_target)
            .unwrap();
        assert!(danger
            .arguments
            .windows(2)
            .any(|pair| pair == ["--sandbox", "danger-full-access"]));
        assert!(danger
            .arguments
            .iter()
            .any(|argument| argument == "--dangerously-bypass-approvals-and-sandbox"));
        assert!(!danger
            .arguments
            .iter()
            .any(|argument| argument.contains("state-json")));
        assert!(danger.non_interactive_approval);
        assert!(danger.requested_network_disabled);
        assert!(danger.effective_network_enforced);
        assert_eq!(danger.windows_uac_authority, "not_granted");
        let canary = service.project_full_access_canary(&profile.id).unwrap();
        assert_eq!(canary.evidence_state, "not_run");
        assert!(canary
            .sentinel_path
            .contains("native-full-access-canary.txt"));
        assert_ne!(canary.launch.working_root, root.to_string_lossy());
    }

    #[test]
    fn unsupported_cli_surface_fails_closed_before_launch_projection() {
        let (directory, mut service) = service();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service
            .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
            .unwrap();
        service.authorize_danger_full_access(&profile.id).unwrap();
        service.cli = Arc::new(FakeCli::unsupported_surface());
        let root = directory.path().join("assigned-application-root");
        fs::create_dir_all(&root).unwrap();
        let target = NativeLaunchTarget::application_owned(root, true).unwrap();
        assert!(service.project_launch(&profile.id, &target).is_err());
        assert_eq!(
            service.profile(&profile.id).unwrap().readiness.attentions.cli,
            Some("codex_cli_surface_unsupported".into())
        );
    }

    #[test]
    fn full_access_canary_persists_then_settles_only_the_owned_sentinel_receipt() {
        let (directory, mut service) = service();
        let fake = Arc::new(FakeCli::enforcing_application_network_policy());
        *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
            succeeded: true,
            exit_code: Some(0),
            sandbox_receipt_observed: true,
        });
        service.cli = fake;
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service
            .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
            .unwrap();
        service.authorize_danger_full_access(&profile.id).unwrap();
        service.run_danger_full_access_canary(&profile.id).unwrap();
        assert_eq!(
            service.profile(&profile.id).unwrap().readiness.danger_full_access_canary,
            "not_run"
        );
        service.query().unwrap();
        assert_eq!(
            service.profile(&profile.id).unwrap().readiness.danger_full_access_canary,
            "passed"
        );
        let persisted: String = service.connection().unwrap().query_row(
            "SELECT state FROM native_codex_profile_full_access_canaries WHERE profile_id=?1",
            params![profile.id], |row| row.get(0),
        ).unwrap();
        assert_eq!(persisted, "passed");
        assert!(directory.path().join("app").exists());
    }

    #[test]
    fn unsupported_danger_network_policy_blocks_canary_before_a_child_starts() {
        let (_directory, mut service) = service();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service
            .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
            .unwrap();
        service.authorize_danger_full_access(&profile.id).unwrap();
        assert!(service.run_danger_full_access_canary(&profile.id).is_err());
        assert_eq!(*fake.starts.lock().unwrap(), 0);
        assert_eq!(
            service.profile(&profile.id).unwrap().readiness.attentions.cli,
            Some("codex_cli_danger_network_enforcement_unsupported".into())
        );
    }

    #[test]
    fn continuity_loss_cancels_owned_login_and_sandbox_children() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        service.request_login(&profile.id).unwrap();
        service.request_sandbox_initialization(&profile.id).unwrap();

        fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
        service.query().unwrap();

        assert_eq!(*fake.terminated.lock().unwrap(), 2);
        assert_eq!(
            service.profile(&profile.id).unwrap().lifecycle,
            Lifecycle::Malformed
        );
        assert_eq!(
            service.profile(&profile.id).unwrap().login_attempt.disposition,
            "cancelled"
        );
        assert_eq!(
            service.profile(&profile.id).unwrap().setup_attempt.disposition,
            "cancelled"
        );
    }

    #[test]
    fn continuity_loss_is_terminal_until_the_home_is_registered_again() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        let home = PathBuf::from(&profile.home_path);
        fs::remove_file(home.join(MARKER_FILE)).unwrap();
        service.query().unwrap();
        write_marker(&home, &profile.id).unwrap();
        let query = service.query().unwrap();
        assert_eq!(query.profiles[0].lifecycle, Lifecycle::Malformed);
        assert!(service.select(&profile.id).is_err());
        assert!(service.request_sandbox_initialization(&profile.id).is_err());
    }

    #[test]
    fn one_attention_can_be_cleared_without_erasing_another_concern() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        service
            .set_attention(
                &profile.id,
                "sandbox",
                Some("sandbox_setup_failed_or_uac_attention_required"),
                false,
            )
            .unwrap();
        service
            .set_attention(
                &profile.id,
                "authentication",
                Some("browser_login_in_progress"),
                false,
            )
            .unwrap();
        service
            .set_attention(&profile.id, "authentication", None, false)
            .unwrap();
        assert_eq!(
            service
                .profile(&profile.id)
                .unwrap()
                .readiness
                .attentions
                .authentication,
            None
        );
        assert_eq!(
            service
                .profile(&profile.id)
                .unwrap()
                .readiness
                .attentions
                .sandbox,
            Some("sandbox_setup_failed_or_uac_attention_required".into())
        );
        assert_eq!(
            service
                .profile(&profile.id)
                .unwrap()
                .readiness
                .authentication,
            "unknown"
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
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<String>>(3)?, row.get::<_, Option<String>>(4)?, row.get::<_, Option<i64>>(5)?, row.get::<_, Option<String>>(6)?, row.get::<_, Option<i32>>(7)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(rows, vec![
            ("failed".into(), "legacy_unclassified_failed".into(), "legacy_unclassified_failed".into(), None, None, None, None, None),
            ("pending".into(), "recovered_unobserved".into(), "recovered_unobserved".into(), None, None, None, None, None),
            ("completed".into(), "terminal_succeeded".into(), "not_observed".into(), None, None, None, None, None),
            ("timed_out".into(), "timed_out".into(), "timed_out".into(), None, None, None, None, None),
            ("cancelled".into(), "cancelled".into(), "cancelled".into(), None, None, None, None, None),
        ]);
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

    #[test]
    fn mcp_reporting_probe_changes_only_its_own_readiness_fact() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        let result = service.probe_mcp_reporting(&profile.id).unwrap();
        assert_eq!(result.readiness.authentication, "unknown");
        assert_eq!(result.readiness.sandbox_initialization, "unknown");
        assert_eq!(result.readiness.workspace_write_canary, "not_run");
        assert_eq!(result.readiness.mcp_reporting, "not_assessed");
        assert_eq!(
            result.readiness.attentions.mcp_reporting,
            Some("mcp_reporting_probe_pending_application_receipt".into())
        );
    }

    #[test]
    fn mcp_receipts_require_one_pending_application_owned_correlation() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        let authority = service.begin_mcp_reporting_probe(&profile.id).unwrap();
        assert!(service
            .record_mcp_reporting_receipt(
                &profile.id,
                &NativeMcpReportingReceipt {
                    capability: authority.capability.clone(),
                    server: authority.server.clone(),
                    tool: authority.tool.clone(),
                    correlation_id: String::new(),
                    probe_root: authority.probe_root.clone(),
                },
            )
            .is_err());
        let ready = service
            .record_mcp_reporting_receipt(
                &profile.id,
                &NativeMcpReportingReceipt {
                    capability: authority.capability.clone(),
                    server: authority.server.clone(),
                    tool: authority.tool.clone(),
                    correlation_id: authority.correlation_id.clone(),
                    probe_root: authority.probe_root.clone(),
                },
            )
            .unwrap();
        assert_eq!(ready.readiness.mcp_reporting, "ready");
        assert!(service
            .record_mcp_reporting_receipt(
                &profile.id,
                &NativeMcpReportingReceipt {
                    capability: authority.capability,
                    server: authority.server,
                    tool: authority.tool,
                    correlation_id: authority.correlation_id,
                    probe_root: authority.probe_root,
                },
            )
            .is_err());
    }

    #[test]
    fn concurrent_mcp_receipts_transition_exactly_one_pending_probe() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("active.sqlite");
        let app = directory.path().join("app");
        let service = NativeProfileService::open(database.clone(), app.clone()).unwrap();
        let profile = service.create_dedicated().unwrap();
        let authority = service.begin_mcp_reporting_probe(&profile.id).unwrap();
        drop(service);
        let receipt = NativeMcpReportingReceipt {
            capability: authority.capability,
            server: authority.server,
            tool: authority.tool,
            correlation_id: authority.correlation_id,
            probe_root: authority.probe_root,
        };
        let barrier = Arc::new(Barrier::new(2));
        let mut joins = vec![];
        for _ in 0..2 {
            let database = database.clone();
            let app = app.clone();
            let profile_id = profile.id.clone();
            let receipt = receipt.clone();
            let barrier = barrier.clone();
            joins.push(thread::spawn(move || {
                let service = NativeProfileService::open(database, app).unwrap();
                barrier.wait();
                service.record_mcp_reporting_receipt(&profile_id, &receipt)
            }));
        }
        let outcomes = joins
            .into_iter()
            .map(|join| join.join().unwrap().is_ok())
            .collect::<Vec<_>>();
        assert_eq!(outcomes.into_iter().filter(|success| *success).count(), 1);
        let reopened = NativeProfileService::open(database, app).unwrap();
        assert_eq!(
            reopened
                .profile(&profile.id)
                .unwrap()
                .readiness
                .mcp_reporting,
            "ready"
        );
    }

    #[test]
    fn cancelled_or_expired_probe_cannot_set_mcp_ready() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        let authority = service.begin_mcp_reporting_probe(&profile.id).unwrap();
        service
            .connection()
            .unwrap()
            .execute(
                "UPDATE native_codex_profile_mcp_probes SET state='cancelled' WHERE profile_id=?1 AND state='pending'",
                params![profile.id],
            )
            .unwrap();
        assert!(service
            .record_mcp_reporting_receipt(
                &profile.id,
                &NativeMcpReportingReceipt {
                    capability: authority.capability,
                    server: authority.server,
                    tool: authority.tool,
                    correlation_id: authority.correlation_id,
                    probe_root: authority.probe_root,
                },
            )
            .is_err());
        assert_ne!(
            service
                .profile(&profile.id)
                .unwrap()
                .readiness
                .mcp_reporting,
            "ready"
        );
    }

    #[test]
    fn concurrent_begin_reuses_the_one_durable_mcp_authority() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("active.sqlite");
        let app = directory.path().join("app");
        let service = NativeProfileService::open(database.clone(), app.clone()).unwrap();
        let profile = service.create_dedicated().unwrap();
        drop(service);
        let barrier = Arc::new(Barrier::new(2));
        let mut joins = vec![];
        for _ in 0..2 {
            let database = database.clone();
            let app = app.clone();
            let profile_id = profile.id.clone();
            let barrier = barrier.clone();
            joins.push(thread::spawn(move || {
                let service = NativeProfileService::open(database, app).unwrap();
                barrier.wait();
                service.begin_mcp_reporting_probe(&profile_id)
            }));
        }
        let first = joins.remove(0).join().unwrap().unwrap();
        let second = joins.remove(0).join().unwrap().unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn foreign_and_stale_mcp_probe_receipts_are_rejected_without_readiness_success() {
        let (_directory, service) = service();
        let first = service.create_dedicated().unwrap();
        let second = service.create_dedicated().unwrap();
        let authority = service.begin_mcp_reporting_probe(&first.id).unwrap();
        assert!(service
            .record_mcp_reporting_receipt(
                &second.id,
                &NativeMcpReportingReceipt {
                    capability: authority.capability.clone(),
                    server: authority.server.clone(),
                    tool: authority.tool.clone(),
                    correlation_id: authority.correlation_id.clone(),
                    probe_root: authority.probe_root.clone(),
                },
            )
            .is_err());
        let connection = service.connection().unwrap();
        connection
            .execute(
                "UPDATE native_codex_profile_mcp_probes SET deadline_at='2000-01-01T00:00:00+00:00' WHERE profile_id=?1",
                params![first.id],
            )
            .unwrap();
        service.query().unwrap();
        assert!(service
            .record_mcp_reporting_receipt(
                &first.id,
                &NativeMcpReportingReceipt {
                    capability: authority.capability,
                    server: authority.server,
                    tool: authority.tool,
                    correlation_id: authority.correlation_id,
                    probe_root: authority.probe_root,
                },
            )
            .is_err());
        assert_eq!(
            service.profile(&first.id).unwrap().readiness.mcp_reporting,
            "not_assessed"
        );
    }

    #[test]
    fn pending_mcp_probe_reopens_with_the_same_private_correlation() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("active.sqlite");
        let service =
            NativeProfileService::open(database.clone(), directory.path().join("app")).unwrap();
        let profile = service.create_dedicated().unwrap();
        let authority = service.begin_mcp_reporting_probe(&profile.id).unwrap();
        drop(service);
        let reopened = NativeProfileService::open(database, directory.path().join("app")).unwrap();
        let retained = reopened.begin_mcp_reporting_probe(&profile.id).unwrap();
        assert_eq!(retained, authority);
        let ready = reopened
            .record_mcp_reporting_receipt(
                &profile.id,
                &NativeMcpReportingReceipt {
                    capability: retained.capability,
                    server: retained.server,
                    tool: retained.tool,
                    correlation_id: retained.correlation_id,
                    probe_root: retained.probe_root,
                },
            )
            .unwrap();
        assert_eq!(ready.readiness.mcp_reporting, "ready");
    }

    #[test]
    fn unavailable_cli_is_profile_attention_not_composition_failure() {
        let (_directory, mut service) = service();
        struct UnavailableCli;
        impl NativeCliPort for UnavailableCli {
            fn run(&self, _: &NativeCliInvocation) -> Result<NativeCliReceipt, String> {
                Err("missing".into())
            }
            fn start(&self, _: &NativeCliInvocation) -> Result<Box<dyn NativeCliChild>, String> {
                Err("missing".into())
            }
            fn surface(&self) -> Result<NativeCliSurface, String> {
                Err("missing".into())
            }
        }
        service.cli = Arc::new(UnavailableCli);
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        assert!(service.refresh_readiness(&profile.id).is_err());
        assert_eq!(
            service
                .profile(&profile.id)
                .unwrap()
                .readiness
                .attentions
                .cli,
            Some("codex_cli_unavailable".into())
        );
    }
}
