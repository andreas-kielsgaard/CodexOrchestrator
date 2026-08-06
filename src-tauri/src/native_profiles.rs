//! Product-owned Codex home profiles. This module deliberately records only filesystem identity
//! and bounded setup observations; it never reads authentication, sandbox, or provider payloads.

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
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

const MARKER_FILE: &str = ".codex-orchestrator-profile.json";
const PROFILE_QUERY_CONTRACT: &str = "native-codex-profile-query/v1";
const MCP_REPORTING_CAPABILITY: &str = "native-codex-profile-reporting/v1";
const MCP_REPORTING_SERVER: &str = "codex-orchestrator-reporting";
const MCP_REPORTING_TOOL: &str = "report_native_profile_readiness";

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeCliInvocation {
    args: Vec<String>,
    cwd: PathBuf,
    codex_home: PathBuf,
    environment: Vec<(String, String)>,
    sandbox_receipt: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NativeCliReceipt {
    succeeded: bool,
    sandbox_receipt_observed: bool,
}

trait NativeCliChild: Send {
    fn try_wait(&mut self) -> Result<Option<bool>, String>;
    fn terminate(&mut self) -> Result<(), String>;
}

trait NativeCliPort: Send + Sync {
    fn run(&self, invocation: &NativeCliInvocation) -> Result<NativeCliReceipt, String>;
    fn start(&self, invocation: &NativeCliInvocation) -> Result<Box<dyn NativeCliChild>, String>;
}

struct SystemNativeCliPort {
    program: Result<String, String>,
}
struct SystemNativeCliChild(Child);

impl NativeCliChild for SystemNativeCliChild {
    fn try_wait(&mut self) -> Result<Option<bool>, String> {
        self.0
            .try_wait()
            .map(|status| status.map(|status| status.success()))
            .map_err(|error| error.to_string())
    }
    fn terminate(&mut self) -> Result<(), String> {
        self.0
            .kill()
            .map_err(|error| error.to_string())
            .and_then(|_| self.0.wait().map(|_| ()).map_err(|error| error.to_string()))
    }
}

impl NativeCliPort for SystemNativeCliPort {
    fn run(&self, invocation: &NativeCliInvocation) -> Result<NativeCliReceipt, String> {
        let program = self
            .program
            .as_ref()
            .map_err(|_| "Codex CLI is unavailable for this profile".to_string())?;
        let status = Command::new(program)
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
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|error| error.to_string())?;
        let sandbox_receipt_observed = invocation.sandbox_receipt.as_ref().is_some_and(|path| {
            fs::read_to_string(path)
                .map(|value| value.trim() == "native-codex-profile-canary")
                .unwrap_or(false)
        });
        Ok(NativeCliReceipt {
            succeeded: status.success(),
            sandbox_receipt_observed,
        })
    }
    fn start(&self, invocation: &NativeCliInvocation) -> Result<Box<dyn NativeCliChild>, String> {
        let program = self
            .program
            .as_ref()
            .map_err(|_| "Codex CLI is unavailable for this profile".to_string())?;
        let child = Command::new(program)
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
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(Box::new(SystemNativeCliChild(child)))
    }
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
    mcp_reporting: String,
    attentions: NativeProfileAttentions,
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
    readiness: NativeProfileReadiness,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeProfileQueryDto {
    contract: &'static str,
    profiles: Vec<NativeProfileDto>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredProfile {
    id: String,
    home: PathBuf,
    identity: String,
    ownership: Ownership,
    lifecycle: Lifecycle,
    selected: bool,
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
}

impl NativeProfileService {
    pub(crate) fn open(database_path: PathBuf, app_data_dir: PathBuf) -> Result<Self, String> {
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
        })
    }

    fn connection(&self) -> Result<Connection, String> {
        crate::storage::open_active_database(&self.database_path)
    }

    pub(crate) fn query(&self) -> Result<NativeProfileQueryDto, String> {
        let mut connection = self.connection()?;
        for profile in load_profiles(&mut connection)? {
            self.revalidate(&profile)?;
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
            .commit()
            .map_err(|error| format!("Unable to commit profile registration: {error}"))?;
        self.profile(&id).map(Into::into)
    }

    pub(crate) fn select(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.profile(id)?;
        if profile.lifecycle != Lifecycle::Active {
            return Err("Native Codex home lost continuity and must be registered again".into());
        }
        let lifecycle = validate_profile(&profile);
        if lifecycle != Lifecycle::Active {
            self.record_lifecycle(id, lifecycle)?;
            return Err("Only a currently validated native profile can be selected".into());
        }
        let now = Utc::now().to_rfc3339();
        let connection = self.connection()?;
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
        self.profile(id).map(Into::into)
    }

    pub(crate) fn request_login(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_active(id)?;
        let mut children = self
            .login_children
            .lock()
            .map_err(|_| "Native profile login supervision is unavailable")?;
        if let Some(child) = children.get_mut(id) {
            if child
                .try_wait()
                .map_err(|error| format!("Unable to observe browser login: {error}"))?
                .is_none()
            {
                return self.profile(id).map(Into::into);
            }
            let succeeded = children
                .remove(id)
                .expect("present")
                .try_wait()?
                .unwrap_or(false);
            self.update_readiness(
                id,
                Some(if succeeded {
                    "authenticated"
                } else {
                    "unauthenticated"
                }),
                None,
                None,
                None,
                Some((
                    "authentication",
                    Some(if succeeded {
                        "login_completed_refresh_required"
                    } else {
                        "browser_login_not_completed"
                    }),
                )),
            )?;
        }
        let root = self.probe_root(id);
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        let child = self
            .cli
            .start(&NativeCliInvocation {
                args: vec!["login".into()],
                cwd: root,
                codex_home: profile.home.clone(),
                environment: native_profile_environment(&profile.home),
                sandbox_receipt: None,
            })
            .map_err(|_| {
                self.set_attention(id, "cli", Some("codex_cli_unavailable"), true)
                    .ok();
                "Unable to start supported Codex browser login".to_string()
            })?;
        children.insert(id.to_string(), child);
        self.set_attention(
            id,
            "authentication",
            Some("browser_login_in_progress"),
            false,
        )?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn refresh_readiness(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_active(id)?;
        self.reap_login(id)?;
        let authenticated = self
            .cli
            .run(&NativeCliInvocation {
                args: vec!["login".into(), "status".into()],
                cwd: self.probe_root(id),
                codex_home: profile.home.clone(),
                environment: native_profile_environment(&profile.home),
                sandbox_receipt: None,
            })
            .map_err(|_| {
                self.set_attention(id, "cli", Some("codex_cli_unavailable"), true)
                    .ok();
                "Codex CLI is unavailable for this profile".to_string()
            })?
            .succeeded;
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
            Some(("authentication", None)),
        )?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn request_sandbox_initialization(
        &self,
        id: &str,
    ) -> Result<NativeProfileDto, String> {
        let profile = self.require_active(id)?;
        self.run_bounded_sandbox_probe(id, &profile, "initialize")?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn run_workspace_write_canary(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_active(id)?;
        self.run_bounded_sandbox_probe(id, &profile, "canary")?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn probe_mcp_reporting(&self, id: &str) -> Result<NativeProfileDto, String> {
        self.require_active(id)?;
        self.update_readiness(
            id,
            None,
            None,
            None,
            Some("not_assessed"),
            Some((
                "mcp_reporting",
                Some("mcp_reporting_probe_requires_correlated_receipt"),
            )),
        )?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn record_mcp_reporting_receipt(
        &self,
        id: &str,
        receipt: &NativeMcpReportingReceipt,
    ) -> Result<NativeProfileDto, String> {
        self.require_active(id)?;
        let valid = receipt.capability == MCP_REPORTING_CAPABILITY
            && receipt.server == MCP_REPORTING_SERVER
            && receipt.tool == MCP_REPORTING_TOOL
            && !receipt.correlation_id.trim().is_empty()
            && receipt.probe_root == self.probe_root(id);
        self.update_readiness(
            id,
            None,
            None,
            None,
            Some(if valid { "ready" } else { "probe_failed" }),
            Some((
                "mcp_reporting",
                (!valid).then_some("mcp_reporting_receipt_invalid"),
            )),
        )?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn resolve_selected_home(&self) -> Result<ResolvedNativeCodexHome, String> {
        let mut connection = self.connection()?;
        let profile = load_profiles(&mut connection)?
            .into_iter()
            .find(|profile| profile.selected)
            .ok_or("No native Codex home is selected")?;
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

    fn run_bounded_sandbox_probe(
        &self,
        id: &str,
        profile: &StoredProfile,
        purpose: &str,
    ) -> Result<(), String> {
        let root = self.probe_root(id);
        fs::create_dir_all(&root).map_err(|error| {
            format!("Unable to create application-owned sandbox probe root: {error}")
        })?;
        let output = root.join(format!("{purpose}.txt"));
        let command = format!("echo native-codex-profile-canary>\"{}\"", output.display());
        let invocation = NativeCliInvocation {
            args: vec![
                "--cd".into(),
                root.to_string_lossy().into_owned(),
                "sandbox".into(),
                "--sandbox-state-disable-network".into(),
                "--sandbox-state-readable-root".into(),
                root.to_string_lossy().into_owned(),
                "--".into(),
                "cmd.exe".into(),
                "/d".into(),
                "/s".into(),
                "/c".into(),
                command,
            ],
            cwd: root.clone(),
            codex_home: profile.home.clone(),
            environment: native_profile_environment(&profile.home),
            sandbox_receipt: (purpose == "canary").then_some(output),
        };
        let receipt = self.cli.run(&invocation).map_err(|_| {
            self.set_attention(id, "cli", Some("codex_cli_unavailable"), false)
                .ok();
            "Codex CLI is unavailable for this profile".to_string()
        })?;
        if purpose == "initialize" && receipt.succeeded {
            self.update_readiness(
                id,
                None,
                Some("initialized"),
                None,
                None,
                Some(("sandbox", None)),
            )?;
        } else if purpose == "canary" && receipt.succeeded && receipt.sandbox_receipt_observed {
            self.update_readiness(id, None, None, Some("passed"), None, Some(("canary", None)))?;
        } else {
            self.update_readiness(
                id,
                None,
                (purpose == "initialize").then_some("attention_required"),
                (purpose == "canary").then_some("blocked"),
                None,
                Some((
                    if purpose == "initialize" {
                        "sandbox"
                    } else {
                        "canary"
                    },
                    Some(if purpose == "initialize" {
                        "sandbox_setup_failed_or_uac_attention_required"
                    } else {
                        "workspace_write_canary_failed"
                    }),
                )),
            )?;
        }
        Ok(())
    }

    fn reap_login(&self, id: &str) -> Result<(), String> {
        let mut children = self
            .login_children
            .lock()
            .map_err(|_| "Native profile login supervision is unavailable")?;
        let Some(child) = children.get_mut(id) else {
            return Ok(());
        };
        let Some(succeeded) = child.try_wait()? else {
            return Ok(());
        };
        children.remove(id);
        self.update_readiness(
            id,
            Some(if succeeded {
                "authenticated"
            } else {
                "unauthenticated"
            }),
            None,
            None,
            None,
            Some((
                "authentication",
                Some(if succeeded {
                    "login_completed_refresh_required"
                } else {
                    "browser_login_not_completed"
                }),
            )),
        )
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
        let connection = self.connection()?;
        connection.execute("UPDATE native_codex_profiles SET lifecycle=?2,selected_at=NULL,updated_at=?3 WHERE id=?1", params![id, lifecycle.database(), Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        connection.execute("UPDATE native_codex_profile_readiness SET authentication='unknown',sandbox_initialization='unknown',workspace_write_canary='not_run',mcp_reporting='not_assessed',observed_at=?2 WHERE profile_id=?1", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        self.write_attention(
            &connection,
            id,
            "continuity",
            Some("profile_continuity_lost"),
        )?;
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

fn load_profiles(connection: &mut Connection) -> Result<Vec<StoredProfile>, String> {
    let mut statement = connection.prepare("SELECT p.id,p.canonical_home_path,p.filesystem_identity,p.ownership,p.lifecycle,p.selected_at,r.authentication,r.sandbox_initialization,r.workspace_write_canary,r.mcp_reporting,(SELECT detail FROM native_codex_profile_attentions a WHERE a.profile_id=p.id AND a.concern='authentication'),(SELECT detail FROM native_codex_profile_attentions a WHERE a.profile_id=p.id AND a.concern='sandbox'),(SELECT detail FROM native_codex_profile_attentions a WHERE a.profile_id=p.id AND a.concern='canary'),(SELECT detail FROM native_codex_profile_attentions a WHERE a.profile_id=p.id AND a.concern='mcp_reporting'),(SELECT detail FROM native_codex_profile_attentions a WHERE a.profile_id=p.id AND a.concern='continuity'),(SELECT detail FROM native_codex_profile_attentions a WHERE a.profile_id=p.id AND a.concern='cli') FROM native_codex_profiles p JOIN native_codex_profile_readiness r ON r.profile_id=p.id ORDER BY p.created_at").map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(StoredProfile {
                id: row.get(0)?,
                home: PathBuf::from(row.get::<_, String>(1)?),
                identity: row.get(2)?,
                ownership: Ownership::parse(&row.get::<_, String>(3)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                lifecycle: Lifecycle::parse(&row.get::<_, String>(4)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                selected: row.get::<_, Option<String>>(5)?.is_some(),
                readiness: NativeProfileReadiness {
                    authentication: row.get(6)?,
                    sandbox_initialization: row.get(7)?,
                    workspace_write_canary: row.get(8)?,
                    mcp_reporting: row.get(9)?,
                    attentions: NativeProfileAttentions {
                        authentication: row.get(10)?,
                        sandbox: row.get(11)?,
                        canary: row.get(12)?,
                        mcp_reporting: row.get(13)?,
                        continuity: row.get(14)?,
                        cli: row.get(15)?,
                    },
                },
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
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
pub(crate) fn run_native_profile_workspace_write_canary(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.run_workspace_write_canary(&input.profile_id)
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

    struct FakeChild {
        result: Option<bool>,
        terminated: Arc<Mutex<usize>>,
    }
    impl NativeCliChild for FakeChild {
        fn try_wait(&mut self) -> Result<Option<bool>, String> {
            Ok(self.result.take())
        }
        fn terminate(&mut self) -> Result<(), String> {
            self.result = Some(false);
            *self.terminated.lock().unwrap() += 1;
            Ok(())
        }
    }
    struct FakeCli {
        receipt: NativeCliReceipt,
        calls: Mutex<Vec<NativeCliInvocation>>,
        starts: Mutex<usize>,
        terminated: Arc<Mutex<usize>>,
    }
    impl FakeCli {
        fn succeeding() -> Self {
            Self {
                receipt: NativeCliReceipt {
                    succeeded: true,
                    sandbox_receipt_observed: true,
                },
                calls: Mutex::new(vec![]),
                starts: Mutex::new(0),
                terminated: Arc::new(Mutex::new(0)),
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
            Ok(Box::new(FakeChild {
                result: None,
                terminated: self.terminated.clone(),
            }))
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
        fs::write(
            Path::new(&dedicated.home_path).join(MARKER_FILE),
            b"malformed",
        )
        .unwrap();
        assert!(service.select(&dedicated.id).is_err());
        let query = service.query().unwrap();
        assert_eq!(query.profiles[0].lifecycle, Lifecycle::Malformed);
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
        let first = service.request_sandbox_initialization(&profile.id).unwrap();
        let second = service.request_sandbox_initialization(&profile.id).unwrap();
        assert_eq!(first.readiness, second.readiness);
        assert_eq!(
            service
                .run_workspace_write_canary(&profile.id)
                .unwrap()
                .readiness
                .workspace_write_canary,
            "passed"
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
            23
        );
    }

    #[test]
    fn sandbox_port_receives_only_the_profile_probe_root_and_separate_phase_receipts() {
        let directory = tempfile::tempdir().unwrap();
        let mut service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
        let fake = Arc::new(FakeCli::succeeding());
        service.cli = fake.clone();
        let profile = service.create_dedicated().unwrap();
        let initialized = service.request_sandbox_initialization(&profile.id).unwrap();
        assert_eq!(initialized.readiness.sandbox_initialization, "initialized");
        assert_eq!(initialized.readiness.workspace_write_canary, "not_run");
        let canaried = service.run_workspace_write_canary(&profile.id).unwrap();
        assert_eq!(canaried.readiness.workspace_write_canary, "passed");

        let calls = fake.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        for call in calls.iter() {
            assert_eq!(call.cwd, service.probe_root(&profile.id));
            assert_eq!(
                call.environment,
                native_profile_environment(Path::new(&profile.home_path))
            );
            assert!(call.args.windows(2).any(|window| window
                .iter()
                .map(String::as_str)
                .eq(["--cd", call.cwd.to_string_lossy().as_ref()])));
            assert!(call.args.windows(2).any(|window| {
                window.iter().map(String::as_str).eq([
                    "--sandbox-state-disable-network",
                    "--sandbox-state-readable-root",
                ])
            }));
            assert!(!call
                .args
                .iter()
                .any(|argument| argument.contains("dangerously")));
        }
        assert!(calls[0].sandbox_receipt.is_none());
        assert!(calls[1].sandbox_receipt.is_some());
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
            Some("browser_login_in_progress".into())
        );
        drop(service);
        assert_eq!(*fake.terminated.lock().unwrap(), 1);
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
            Some("mcp_reporting_probe_requires_correlated_receipt".into())
        );
    }

    #[test]
    fn mcp_list_style_presence_never_becomes_ready_without_a_correlated_receipt() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        let rejected = service
            .record_mcp_reporting_receipt(
                &profile.id,
                &NativeMcpReportingReceipt {
                    capability: MCP_REPORTING_CAPABILITY.into(),
                    server: MCP_REPORTING_SERVER.into(),
                    tool: MCP_REPORTING_TOOL.into(),
                    correlation_id: String::new(),
                    probe_root: service.probe_root(&profile.id),
                },
            )
            .unwrap();
        assert_eq!(rejected.readiness.mcp_reporting, "probe_failed");
        let ready = service
            .record_mcp_reporting_receipt(
                &profile.id,
                &NativeMcpReportingReceipt {
                    capability: MCP_REPORTING_CAPABILITY.into(),
                    server: MCP_REPORTING_SERVER.into(),
                    tool: MCP_REPORTING_TOOL.into(),
                    correlation_id: "application-owned-correlation".into(),
                    probe_root: service.probe_root(&profile.id),
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
        }
        service.cli = Arc::new(UnavailableCli);
        let profile = service.create_dedicated().unwrap();
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
