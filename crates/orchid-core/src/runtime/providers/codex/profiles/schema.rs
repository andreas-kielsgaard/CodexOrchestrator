//! Current schema and historical desktop migration statements.
pub const NATIVE_PROFILE_SCHEMA: &str = r#"
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
CREATE TABLE IF NOT EXISTS native_codex_profile_preferences (
  profile_id TEXT PRIMARY KEY,
  personality TEXT NOT NULL CHECK (personality IN ('inherit','none','friendly','pragmatic')),
  updated_at TEXT NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS agent_session_native_profile_bindings (
  session_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  bound_at TEXT NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS agent_session_native_profile_launch_provenance (
  invocation_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  environment_key TEXT NOT NULL CHECK(environment_key='CODEX_HOME'),
  invocation_mode TEXT NOT NULL CHECK(invocation_mode IN ('start','resume')),
  prepared_at TEXT NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
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
  state TEXT NOT NULL CHECK (state IN ('pending','dispatching','received','expired','cancelled')),
  requested_at TEXT NOT NULL,
  deadline_at TEXT NOT NULL,
  dispatch_claim_id TEXT,
  dispatch_claimed_at TEXT,
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
  authority_scope TEXT NOT NULL CHECK (authority_scope IN ('filesystem_only','full_machine_filesystem_and_unrestricted_network')),
  authority_version TEXT NOT NULL CHECK (authority_version IN ('danger-full-access/filesystem-only/v1','danger-full-access/unrestricted-network/v1')),
  authorization_correlation_id TEXT,
  authorized_at TEXT NOT NULL,
  revoked_at TEXT,
  CHECK ((authority_scope='filesystem_only' AND authority_version='danger-full-access/filesystem-only/v1' AND authorization_correlation_id IS NULL) OR (authority_scope='full_machine_filesystem_and_unrestricted_network' AND authority_version='danger-full-access/unrestricted-network/v1' AND authorization_correlation_id IS NOT NULL AND length(trim(authorization_correlation_id))>0)),
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
  authorization_correlation_id TEXT,
  authorization_version TEXT,
  correlation_id TEXT,
  state TEXT NOT NULL CHECK (state IN ('pending','passed','launch_failed','terminal_failed','timed_out','cancelled','recovered_unobserved','cleanup_failed','legacy_unverified')),
  requested_at TEXT NOT NULL,
  launch_accepted_at TEXT,
  deadline_at TEXT,
  settled_at TEXT,
  process_activity TEXT NOT NULL CHECK (process_activity IN ('unobserved','launch_accepted','terminal_observed')),
  provider_activity TEXT NOT NULL CHECK (provider_activity='unobserved'),
  terminal_classification TEXT NOT NULL CHECK (terminal_classification IN ('not_observed','exit_code','receipt_missing','launch_failed','timed_out','cancelled','recovered_unobserved','cleanup_failed','legacy_unverified')),
  terminal_exit_code INTEGER,
  receipt_observed INTEGER NOT NULL CHECK (receipt_observed IN (0,1)),
  cleanup_disposition TEXT NOT NULL CHECK (cleanup_disposition IN ('pending','removed','failed','not_observed')),
  CHECK (state <> 'pending' OR (launch_accepted_at IS NULL OR process_activity='launch_accepted')),
  CHECK (state <> 'passed' OR (launch_accepted_at IS NOT NULL AND settled_at IS NOT NULL AND process_activity='terminal_observed' AND receipt_observed=1 AND cleanup_disposition='removed')),
  CHECK (state <> 'launch_failed' OR (launch_accepted_at IS NULL AND settled_at IS NOT NULL AND terminal_classification='launch_failed' AND terminal_exit_code IS NULL AND receipt_observed=0)),
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

pub const NATIVE_PROFILE_V22_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V23_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V24_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V25_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V26_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V27_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V28_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V29_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V30_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V31_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V32_MIGRATION: &str = r#"
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

pub const NATIVE_PROFILE_V33_MIGRATION: &str = r#"
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

/// Existing danger approval rows covered only filesystem authority. Preserve their durable
/// identity and timestamp, but do not reinterpret them as approval for unrestricted networking.
pub const NATIVE_PROFILE_V34_MIGRATION: &str = r#"
ALTER TABLE native_codex_profile_mode_authorizations RENAME TO native_codex_profile_mode_authorizations_v33;
CREATE TABLE native_codex_profile_mode_authorizations (
  profile_id TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode='danger_full_access'),
  filesystem_identity TEXT NOT NULL,
  authority_scope TEXT NOT NULL CHECK (authority_scope IN ('filesystem_only','full_machine_filesystem_and_unrestricted_network')),
  authority_version TEXT NOT NULL CHECK (authority_version IN ('danger-full-access/filesystem-only/v1','danger-full-access/unrestricted-network/v1')),
  authorization_correlation_id TEXT,
  authorized_at TEXT NOT NULL,
  revoked_at TEXT,
  CHECK ((authority_scope='filesystem_only' AND authority_version='danger-full-access/filesystem-only/v1' AND authorization_correlation_id IS NULL) OR (authority_scope='full_machine_filesystem_and_unrestricted_network' AND authority_version='danger-full-access/unrestricted-network/v1' AND authorization_correlation_id IS NOT NULL AND length(trim(authorization_correlation_id))>0)),
  PRIMARY KEY(profile_id, mode),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
INSERT INTO native_codex_profile_mode_authorizations (profile_id,mode,filesystem_identity,authority_scope,authority_version,authorization_correlation_id,authorized_at,revoked_at)
SELECT profile_id,mode,filesystem_identity,'filesystem_only','danger-full-access/filesystem-only/v1',NULL,authorized_at,revoked_at
FROM native_codex_profile_mode_authorizations_v33;
DROP TABLE native_codex_profile_mode_authorizations_v33;
"#;

pub const NATIVE_PROFILE_V34_FULL_ACCESS_CANARY_MIGRATION: &str = r#"
ALTER TABLE native_codex_profile_full_access_canaries RENAME TO native_codex_profile_full_access_canaries_v33;
CREATE TABLE native_codex_profile_full_access_canaries (
  attempt_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  filesystem_identity TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode='danger_full_access'),
  executable TEXT NOT NULL,
  version TEXT NOT NULL,
  sentinel_path TEXT NOT NULL,
  authorization_correlation_id TEXT,
  authorization_version TEXT,
  correlation_id TEXT,
  state TEXT NOT NULL CHECK (state IN ('pending','passed','launch_failed','terminal_failed','timed_out','cancelled','recovered_unobserved','cleanup_failed','legacy_unverified')),
  requested_at TEXT NOT NULL,
  launch_accepted_at TEXT,
  deadline_at TEXT,
  settled_at TEXT,
  process_activity TEXT NOT NULL CHECK (process_activity IN ('unobserved','launch_accepted','terminal_observed')),
  provider_activity TEXT NOT NULL CHECK (provider_activity='unobserved'),
  terminal_classification TEXT NOT NULL CHECK (terminal_classification IN ('not_observed','exit_code','receipt_missing','launch_failed','timed_out','cancelled','recovered_unobserved','cleanup_failed','legacy_unverified')),
  terminal_exit_code INTEGER,
  receipt_observed INTEGER NOT NULL CHECK (receipt_observed IN (0,1)),
  cleanup_disposition TEXT NOT NULL CHECK (cleanup_disposition IN ('pending','removed','failed','not_observed')),
  CHECK (state <> 'passed' OR (launch_accepted_at IS NOT NULL AND settled_at IS NOT NULL AND process_activity='terminal_observed' AND receipt_observed=1 AND cleanup_disposition='removed')),
  CHECK (state <> 'launch_failed' OR (launch_accepted_at IS NULL AND settled_at IS NOT NULL AND terminal_classification='launch_failed' AND terminal_exit_code IS NULL AND receipt_observed=0)),
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
INSERT INTO native_codex_profile_full_access_canaries (attempt_id,profile_id,filesystem_identity,mode,executable,version,sentinel_path,authorization_correlation_id,authorization_version,correlation_id,state,requested_at,launch_accepted_at,deadline_at,settled_at,process_activity,provider_activity,terminal_classification,terminal_exit_code,receipt_observed,cleanup_disposition)
SELECT attempt_id,profile_id,filesystem_identity,mode,executable,version,sentinel_path,NULL,NULL,NULL,
  CASE state WHEN 'pending' THEN 'recovered_unobserved' ELSE 'legacy_unverified' END,
  started_at,NULL,NULL,completed_at,
  'unobserved','unobserved',
  CASE state WHEN 'pending' THEN 'recovered_unobserved' ELSE 'legacy_unverified' END,
  NULL,0,'not_observed'
FROM native_codex_profile_full_access_canaries_v33;
DROP TABLE native_codex_profile_full_access_canaries_v33;
CREATE UNIQUE INDEX IF NOT EXISTS ux_native_codex_profile_full_access_canary_pending
ON native_codex_profile_full_access_canaries(profile_id) WHERE state='pending';
UPDATE native_codex_profile_readiness SET danger_full_access_canary='blocked'
WHERE danger_full_access_canary='passed';
"#;

pub const NATIVE_PROFILE_V35_MCP_DISPATCH_CLAIM_MIGRATION: &str = r#"
DROP INDEX ux_native_codex_profile_mcp_probe_pending;
ALTER TABLE native_codex_profile_mcp_probes RENAME TO native_codex_profile_mcp_probes_v34;
CREATE TABLE native_codex_profile_mcp_probes (
  request_id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  correlation_id TEXT NOT NULL UNIQUE,
  expected_capability TEXT NOT NULL,
  expected_server TEXT NOT NULL,
  expected_tool TEXT NOT NULL,
  expected_probe_root TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('pending','dispatching','received','expired','cancelled')),
  requested_at TEXT NOT NULL,
  deadline_at TEXT NOT NULL,
  dispatch_claim_id TEXT,
  dispatch_claimed_at TEXT,
  received_at TEXT,
  FOREIGN KEY(profile_id) REFERENCES native_codex_profiles(id) ON DELETE RESTRICT
);
INSERT INTO native_codex_profile_mcp_probes (request_id,profile_id,correlation_id,expected_capability,expected_server,expected_tool,expected_probe_root,state,requested_at,deadline_at,dispatch_claim_id,dispatch_claimed_at,received_at)
SELECT request_id,profile_id,correlation_id,expected_capability,expected_server,expected_tool,expected_probe_root,state,requested_at,deadline_at,NULL,NULL,received_at
FROM native_codex_profile_mcp_probes_v34;
DROP TABLE native_codex_profile_mcp_probes_v34;
CREATE UNIQUE INDEX ux_native_codex_profile_mcp_probe_pending
ON native_codex_profile_mcp_probes(profile_id) WHERE state='pending';
"#;
