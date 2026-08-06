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
    sync::Mutex,
    thread,
    time::{Duration, Instant},
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

const MARKER_FILE: &str = ".codex-orchestrator-profile.json";
const PROFILE_QUERY_CONTRACT: &str = "native-codex-profile-query/v1";

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
    attention: Option<String>,
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
    codex_program: Result<String, String>,
    login_children: Mutex<HashMap<String, Child>>,
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
            codex_program: crate::runtime::codex::resolve_program("codex".into()),
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
        let program = self.codex_program.as_ref().map_err(|_| {
            self.set_attention(id, Some("codex_cli_unavailable"), true)
                .ok();
            "Codex CLI is unavailable for this profile".to_string()
        })?;
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
            children.remove(id);
            self.set_attention(id, None, false)?;
        }
        let mut command = Command::new(program);
        command
            // Default login is the supported browser flow. Device authentication is deliberately
            // not hidden behind nulled output because its link and one-time code are user-facing.
            .arg("login")
            .env("CODEX_HOME", &profile.home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = command
            .spawn()
            .map_err(|error| format!("Unable to start supported Codex browser login: {error}"))?;
        children.insert(id.to_string(), child);
        self.set_attention(id, Some("browser_login_in_progress"), false)?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn refresh_readiness(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_active(id)?;
        let program = self.codex_program.as_ref().map_err(|_| {
            self.set_attention(id, Some("codex_cli_unavailable"), true)
                .ok();
            "Codex CLI is unavailable for this profile".to_string()
        })?;
        let authenticated = run_status(program, &["login", "status"], &profile.home)?;
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
        let profile = self.require_active(id)?;
        let program = self.codex_program.as_ref().map_err(|_| {
            self.set_attention(id, Some("codex_cli_unavailable"), false)
                .ok();
            "Codex CLI is unavailable for this profile".to_string()
        })?;
        // The probe has no network or provider interaction: it only verifies the installed CLI's
        // MCP command against the selected home and records an application-owned result file.
        let ready = run_status(program, &["mcp", "list"], &profile.home)?;
        let report = self.probe_root(id).join("mcp-reporting-probe.json");
        fs::create_dir_all(report.parent().expect("probe parent"))
            .map_err(|error| error.to_string())?;
        fs::write(&report, br#"{"contract":"native-mcp-reporting-probe/v1"}"#)
            .map_err(|error| error.to_string())?;
        self.update_readiness(
            id,
            None,
            None,
            None,
            Some(if ready { "ready" } else { "probe_failed" }),
            None,
        )?;
        self.profile(id).map(Into::into)
    }

    pub(crate) fn resolve_selected_home(&self) -> Result<ResolvedNativeCodexHome, String> {
        let mut connection = self.connection()?;
        let profile = load_profiles(&mut connection)?
            .into_iter()
            .find(|profile| profile.selected)
            .ok_or("No native Codex home is selected")?;
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
        let program = self.codex_program.as_ref().map_err(|_| {
            self.set_attention(id, Some("codex_cli_unavailable"), false)
                .ok();
            "Codex CLI is unavailable for this profile".to_string()
        })?;
        let root = self.probe_root(id);
        fs::create_dir_all(&root).map_err(|error| {
            format!("Unable to create application-owned sandbox probe root: {error}")
        })?;
        let output = root.join(format!("{purpose}.txt"));
        let command = format!(
            "echo native-codex-profile-{purpose}>\"{}\"",
            output.display()
        );
        let success = run_status(
            program,
            &["sandbox", "--", "cmd.exe", "/d", "/s", "/c", &command],
            &profile.home,
        )?;
        let expected = format!("native-codex-profile-{purpose}");
        let wrote_expected = fs::read_to_string(&output)
            .map(|value| value.trim() == expected)
            .unwrap_or(false);
        if success && wrote_expected {
            self.update_readiness(id, None, Some("initialized"), Some("passed"), None, None)?;
        } else {
            self.update_readiness(
                id,
                None,
                Some("attention_required"),
                Some("blocked"),
                None,
                Some("sandbox_probe_failed_or_uac_attention_required"),
            )?;
        }
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
        let connection = self.connection()?;
        connection.execute("UPDATE native_codex_profiles SET lifecycle=?2,selected_at=NULL,updated_at=?3 WHERE id=?1", params![id, lifecycle.database(), Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        connection.execute("UPDATE native_codex_profile_readiness SET authentication='unknown',sandbox_initialization='unknown',workspace_write_canary='not_run',mcp_reporting='not_assessed',attention='profile_continuity_lost',observed_at=?2 WHERE profile_id=?1", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn update_readiness(
        &self,
        id: &str,
        authentication: Option<&str>,
        sandbox: Option<&str>,
        canary: Option<&str>,
        mcp: Option<&str>,
        attention: Option<&str>,
    ) -> Result<(), String> {
        let connection = self.connection()?;
        connection.execute(
            "UPDATE native_codex_profile_readiness SET authentication=COALESCE(?2,authentication),sandbox_initialization=COALESCE(?3,sandbox_initialization),workspace_write_canary=COALESCE(?4,workspace_write_canary),mcp_reporting=COALESCE(?5,mcp_reporting),attention=?6,observed_at=?7 WHERE profile_id=?1",
            params![id, authentication, sandbox, canary, mcp, attention, Utc::now().to_rfc3339()],
        ).map_err(|error| format!("Unable to record native profile readiness: {error}"))?;
        Ok(())
    }

    fn set_attention(
        &self,
        id: &str,
        attention: Option<&str>,
        reset_readiness: bool,
    ) -> Result<(), String> {
        let connection = self.connection()?;
        if reset_readiness {
            connection.execute("UPDATE native_codex_profile_readiness SET authentication='unknown',sandbox_initialization='unknown',workspace_write_canary='not_run',mcp_reporting='not_assessed',attention=?2,observed_at=?3 WHERE profile_id=?1", params![id, attention, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        } else {
            connection.execute("UPDATE native_codex_profile_readiness SET attention=?2,observed_at=?3 WHERE profile_id=?1", params![id, attention, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        }
        Ok(())
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

fn run_status(program: &str, args: &[&str], home: &Path) -> Result<bool, String> {
    let mut child = Command::new(program)
        .args(args)
        .env("CODEX_HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Unable to probe Codex authentication status: {error}"))?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("Unable to observe Codex authentication status: {error}"))?
        {
            return Ok(status.success());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Codex authentication status probe exceeded its bounded duration".into());
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn load_profiles(connection: &mut Connection) -> Result<Vec<StoredProfile>, String> {
    let mut statement = connection.prepare("SELECT p.id,p.canonical_home_path,p.filesystem_identity,p.ownership,p.lifecycle,p.selected_at,r.authentication,r.sandbox_initialization,r.workspace_write_canary,r.mcp_reporting,r.attention FROM native_codex_profiles p JOIN native_codex_profile_readiness r ON r.profile_id=p.id ORDER BY p.created_at").map_err(|error| error.to_string())?;
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
                    attention: row.get(10)?,
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

    fn service() -> (tempfile::TempDir, NativeProfileService) {
        let directory = tempfile::tempdir().unwrap();
        let service = NativeProfileService::open(
            directory.path().join("active.sqlite"),
            directory.path().join("app"),
        )
        .unwrap();
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
                Some("sandbox_probe_failed_or_uac_attention_required"),
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
            22
        );
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
    fn attention_can_be_cleared_without_implying_readiness() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        service
            .set_attention(&profile.id, Some("browser_login_in_progress"), false)
            .unwrap();
        service.set_attention(&profile.id, None, false).unwrap();
        assert_eq!(
            service.profile(&profile.id).unwrap().readiness.attention,
            None
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
    }

    #[test]
    fn mcp_reporting_probe_changes_only_its_own_readiness_fact() {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        let result = service.probe_mcp_reporting(&profile.id).unwrap();
        assert_eq!(result.readiness.authentication, "unknown");
        assert_eq!(result.readiness.sandbox_initialization, "unknown");
        assert_eq!(result.readiness.workspace_write_canary, "not_run");
        assert_eq!(result.readiness.mcp_reporting, "ready");
    }

    #[test]
    fn unavailable_cli_is_profile_attention_not_composition_failure() {
        let (_directory, mut service) = service();
        service.codex_program = Err("missing".into());
        let profile = service.create_dedicated().unwrap();
        assert!(service.refresh_readiness(&profile.id).is_err());
        assert_eq!(
            service.profile(&profile.id).unwrap().readiness.attention,
            Some("codex_cli_unavailable".into())
        );
    }
}
