//! Product-owned Codex home profiles. This module deliberately records only filesystem identity
//! and bounded setup observations; it never reads authentication, sandbox, or provider payloads.

mod discovery;
pub use discovery::DiscoveredNativeCodexHome;
mod readiness;
pub mod session_binding;

use crate::persistence::ActiveDatabase;
use axum::http::{header, StatusCode};
use bytes::Bytes;
use chrono::{DateTime, Duration, Utc};
use http_body_util::Empty;
use hyper::{server::conn::http1, service::service_fn, Response};
use hyper_util::rt::TokioIo;
use orchid_engine::providers::codex::options::CodexPersonality;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use rusqlite::{params, Connection, OptionalExtension};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::sync::OnceLock;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
};

use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

const FULL_ACCESS_CANARY_RECEIPT: &str = "native-codex-profile-canary";
const FULL_ACCESS_CANARY_RECEIPT_FILE: &str = "native-full-access-canary.txt";
const FULL_ACCESS_CANARY_RECEIPT_RELATIVE_PATH: &str = "..\\receipt\\native-full-access-canary.txt";

mod schema;
pub use schema::*;

const MARKER_FILE: &str = ".codex-orchestrator-profile.json";
const PROFILE_QUERY_CONTRACT: &str = "native-codex-profile-query/v1";
const MCP_REPORTING_CAPABILITY: &str = "native-codex-profile-reporting/v1";
const MCP_REPORTING_SERVER: &str = "codex-orchestrator-reporting";
const MCP_REPORTING_TOOL: &str = "report_native_profile_readiness";
const SETUP_ATTEMPT_TIMEOUT_SECONDS: i64 = 120;
const MCP_PROBE_TIMEOUT_SECONDS: i64 = 300;
const WORKSPACE_WRITE_CANARY_COMMAND_FILE: &str = "native-codex-profile-canary.cmd";
const FULL_ACCESS_CANARY_TIMEOUT_SECONDS: i64 = 120;
const DANGER_AUTHORITY_SCOPE: &str = "full_machine_filesystem_and_unrestricted_network";
const DANGER_AUTHORITY_VERSION: &str = "danger-full-access/unrestricted-network/v1";
#[cfg(test)]
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
    danger_unrestricted_network_supported: bool,
    non_interactive_approval_supported: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeCliSurface {
    provenance: NativeCliProvenance,
    windows_sandbox_setup_supported: bool,
    workspace_launch_flags_supported: bool,
    workspace_launch_project_config_isolated: bool,
}

fn danger_launch_semantic_capability(version: &str, exec_help: &str) -> bool {
    version == "codex-cli 0.144.0"
        && [
            "--json",
            "--strict-config",
            "--ignore-user-config",
            "--ignore-rules",
            "--config",
            "--sandbox",
            "--cd",
            "--skip-git-repo-check",
            "danger-full-access",
            "--dangerously-bypass-approvals-and-sandbox",
        ]
        .iter()
        .all(|token| exec_help.contains(token))
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
    authorization_correlation_id: String,
    authorization_version: String,
    correlation_id: String,
    deadline_at: DateTime<Utc>,
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
                    .map(|value| value.trim() == FULL_ACCESS_CANARY_RECEIPT)
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

fn discard_native_cli_stream(
    mut stream: impl Read + Send + 'static,
) -> std::thread::JoinHandle<()> {
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
        Ok(Box::new(spawn_system_native_cli_child(
            program, invocation,
        )?))
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
        let sandbox_setup_help = native_cli_help_output(program, &["sandbox", "setup", "--help"])?;
        let (workspace_sandbox_supported, windows_sandbox_setup_supported) =
            windows_semantic_sandbox_capabilities(&sandbox_help, &sandbox_setup_help);
        let (workspace_launch_flags_supported, workspace_launch_project_config_isolated) =
            workspace_launch_semantic_capabilities(version.trim(), &exec_help);
        let danger_full_access_supported =
            danger_launch_semantic_capability(version.trim(), &exec_help);
        let danger_unrestricted_network_supported = danger_full_access_supported;
        let non_interactive_approval_supported = danger_full_access_supported;
        Ok(NativeCliSurface {
            provenance: NativeCliProvenance {
                executable: program.clone(),
                version: version.trim().to_string(),
                workspace_sandbox_supported,
                danger_full_access_supported,
                danger_unrestricted_network_supported,
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

fn workspace_launch_semantic_capabilities(version: &str, exec_help: &str) -> (bool, bool) {
    let launch_flags_supported = [
        "--json",
        "--strict-config",
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
    // The v0.144 public source establishes the exact config-override ordering needed here:
    // runtime overrides participate in project-trust selection before project layers load.
    // Do not infer that contract from matching help text on another version.
    let project_config_isolated = version == "codex-cli 0.144.0" && launch_flags_supported;
    (launch_flags_supported, project_config_isolated)
}

fn workspace_project_trust_override(working_root: &Path) -> Result<String, String> {
    let working_root = working_root.to_str().ok_or_else(|| {
        "The application-owned working root is not safely representable".to_string()
    })?;
    if working_root.chars().any(char::is_control) {
        return Err("The application-owned working root is not safely representable".into());
    }
    // Keep the path inside one TOML inline-table value. CliConfigOverrides splits only the
    // outer `projects` key on dots, so this remains safe for Windows paths containing dots.
    let escaped_root = working_root.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!(
        "projects={{\"{escaped_root}\"={{trust_level=\"untrusted\"}}}}"
    ))
}

fn observe_elevated_windows_sandbox_mode(home: &Path) -> Result<bool, String> {
    let file = fs::File::open(home.join("config.toml")).map_err(|_| {
        "The selected profile has no readable external sandbox configuration".to_string()
    })?;
    let mut in_windows = false;
    let mut windows_table_count = 0;
    let mut elevated_assignments = 0;
    let mut non_elevated_assignments = 0;
    for line in BufReader::new(file).lines() {
        let line = line
            .map_err(|_| "The selected profile sandbox configuration cannot be read".to_string())?;
        if line.len() > 1024 {
            return Err(
                "The selected profile sandbox configuration is not safely observable".into(),
            );
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
pub enum ExecutionMode {
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
pub struct NativeProfileReadiness {
    authentication: String,
    sandbox_initialization: String,
    workspace_write_canary: String,
    danger_full_access_canary: String,
    mcp_reporting: String,
    attentions: NativeProfileAttentions,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProfileExecution {
    selected_mode: ExecutionMode,
    danger_full_access_authorized: bool,
    danger_authorization: NativeProfileDangerAuthorization,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProfileDangerAuthorization {
    disposition: String,
    authority_scope: Option<String>,
    authority_version: Option<String>,
    correlation_id: Option<String>,
    authorized_at: Option<String>,
    revoked_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProfileLoginAttempt {
    disposition: String,
    browser_handoff: String,
    requested_at: Option<String>,
    launch_accepted_at: Option<String>,
    settled_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProfileSetupAttempt {
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
pub struct NativeProfileSandboxAdoption {
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
pub struct NativeProfileSandboxAdoptionConfirmation {
    disposition: String,
    correlation_id: Option<String>,
    confirmed_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProfileFullAccessCanaryAttempt {
    disposition: String,
    authorization_version: Option<String>,
    authorization_correlation_id: Option<String>,
    correlation_id: Option<String>,
    requested_at: Option<String>,
    launch_accepted_at: Option<String>,
    deadline_at: Option<String>,
    settled_at: Option<String>,
    process_activity: String,
    provider_activity: String,
    terminal_classification: String,
    terminal_exit_code: Option<i32>,
    receipt_observed: bool,
    cleanup_disposition: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProfileAttentions {
    authentication: Option<String>,
    sandbox: Option<String>,
    canary: Option<String>,
    mcp_reporting: Option<String>,
    continuity: Option<String>,
    cli: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProfileDto {
    id: String,
    home_path: String,
    ownership: Ownership,
    lifecycle: Lifecycle,
    selected: bool,
    personality: Option<CodexPersonality>,
    execution: NativeProfileExecution,
    login_attempt: NativeProfileLoginAttempt,
    setup_attempt: NativeProfileSetupAttempt,
    sandbox_adoption: NativeProfileSandboxAdoption,
    sandbox_adoption_confirmation: NativeProfileSandboxAdoptionConfirmation,
    full_access_canary_attempt: NativeProfileFullAccessCanaryAttempt,
    readiness: NativeProfileReadiness,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProfileQueryDto {
    contract: &'static str,
    profiles: Vec<NativeProfileDto>,
}

/// A validated command description, not a launch request, acceptance, provider observation, or
/// workflow outcome. Its working root and network restriction come from a separate application
/// target and are never profile-derived.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeLaunchProjectionDto {
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
pub struct NativeLaunchTarget {
    working_root: PathBuf,
    network_disabled: bool,
}

impl NativeLaunchTarget {
    pub fn application_owned(
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
pub struct NativeFullAccessCanaryProjectionDto {
    launch: NativeLaunchProjectionDto,
    sentinel_path: String,
    evidence_state: &'static str,
}

/// NCHP-03 supplies this only after its bounded, application-owned MCP action receives a
/// correlated receipt. It is deliberately not inferred from `codex mcp list` or a file write.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeMcpReportingReceipt {
    pub capability: String,
    pub server: String,
    pub tool: String,
    pub correlation_id: String,
    pub probe_root: PathBuf,
}

/// Private application authority for NCHP-03. This never appears in settings DTOs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeMcpReportingProbeAuthority {
    pub profile_id: String,
    pub correlation_id: String,
    pub capability: String,
    pub server: String,
    pub tool: String,
    pub probe_root: PathBuf,
}

struct ClaimedNativeMcpReportingProbe {
    authority: NativeMcpReportingProbeAuthority,
    claim_id: String,
}

/// An application-owned MCP server for one already-durable reporting request. It accepts no caller
/// supplied identity or correlation; those stay bound to the pending application authority.
#[derive(Clone)]
struct NativeMcpReportingMcp {
    authority: NativeMcpReportingProbeAuthority,
    receipt: mpsc::SyncSender<NativeMcpReportingReceipt>,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NativeMcpReportingInput {}

impl NativeMcpReportingMcp {
    fn new(
        authority: NativeMcpReportingProbeAuthority,
        receipt: mpsc::SyncSender<NativeMcpReportingReceipt>,
    ) -> Self {
        Self {
            authority,
            receipt,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl NativeMcpReportingMcp {
    #[tool(
        annotations(
            title = "Report native profile readiness",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        ),
        description = "Record the one application-bound native-profile reporting receipt. Input is ONLY {}. It reports MCP exposure and this exact tool call only; readiness, provider activity, and application-consumer resolution remain separate facts."
    )]
    fn report_native_profile_readiness(
        &self,
        Parameters(_): Parameters<NativeMcpReportingInput>,
    ) -> CallToolResult {
        let receipt = NativeMcpReportingReceipt {
            capability: self.authority.capability.clone(),
            server: self.authority.server.clone(),
            tool: self.authority.tool.clone(),
            correlation_id: self.authority.correlation_id.clone(),
            probe_root: self.authority.probe_root.clone(),
        };
        match self.receipt.try_send(receipt) {
            Ok(()) => CallToolResult::success(vec![ContentBlock::text(
                "{\"status\":\"mcp_reporting_receipt_reported\",\"ready\":false}",
            )]),
            Err(mpsc::TrySendError::Full(_)) => CallToolResult::success(vec![ContentBlock::text(
                "{\"status\":\"mcp_reporting_receipt_already_reported\",\"ready\":false}",
            )]),
            Err(mpsc::TrySendError::Disconnected(_)) => {
                CallToolResult::error(vec![ContentBlock::text("{\"code\":\"unavailable\"}")])
            }
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for NativeMcpReportingMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Call report_native_profile_readiness exactly once with {}. The application validates and settles the correlated receipt separately.",
        )
    }
}

struct NativeMcpReportingServer {
    address: SocketAddr,
    bearer: String,
    receipt: mpsc::Receiver<NativeMcpReportingReceipt>,
    cancellation: CancellationToken,
    join: Option<thread::JoinHandle<()>>,
}

impl NativeMcpReportingServer {
    fn start(authority: NativeMcpReportingProbeAuthority) -> Result<Self, String> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|_| {
            "Unable to expose the application-owned MCP reporting server".to_string()
        })?;
        listener.set_nonblocking(true).map_err(|_| {
            "Unable to expose the application-owned MCP reporting server".to_string()
        })?;
        let address = listener.local_addr().map_err(|_| {
            "Unable to expose the application-owned MCP reporting server".to_string()
        })?;
        let bearer = uuid::Uuid::new_v4().simple().to_string();
        let (receipt_tx, receipt) = mpsc::sync_channel(1);
        let cancellation = CancellationToken::new();
        let server_cancel = cancellation.clone();
        let expected_bearer = bearer.clone();
        let join = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .expect("native MCP reporting runtime");
            runtime.block_on(async move {
                let allowed_host = format!("127.0.0.1:{}", address.port());
                let config =
                    rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default()
                        .with_allowed_hosts([allowed_host.clone()])
                        .with_cancellation_token(server_cancel.clone());
                let service: rmcp::transport::streamable_http_server::StreamableHttpService<
                    NativeMcpReportingMcp,
                    rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
                > = rmcp::transport::streamable_http_server::StreamableHttpService::new(
                    move || {
                        Ok(NativeMcpReportingMcp::new(
                            authority.clone(),
                            receipt_tx.clone(),
                        ))
                    },
                    Default::default(),
                    config,
                );
                let expected_bearer = Arc::new(expected_bearer);
                let listener = tokio::net::TcpListener::from_std(listener)
                    .expect("native MCP reporting listener");
                loop {
                    let accepted = tokio::select! {
                        _ = server_cancel.cancelled() => break,
                        accepted = listener.accept() => accepted,
                    };
                    let Ok((stream, _)) = accepted else { continue };
                    let service = service.clone();
                    let expected_bearer = expected_bearer.clone();
                    let allowed_host = allowed_host.clone();
                    tokio::spawn(async move {
                        let guard = service_fn(move |request| {
                            let service = service.clone();
                            let expected_bearer = expected_bearer.clone();
                            let allowed_host = allowed_host.clone();
                            async move {
                                if let Some(status) = native_mcp_transport_denial(
                                    &expected_bearer,
                                    &allowed_host,
                                    &request,
                                ) {
                                    return Ok::<_, std::convert::Infallible>(
                                        Response::builder()
                                            .status(status)
                                            .body(Empty::<Bytes>::new())
                                            .expect("native MCP denial response")
                                            .map(axum::body::Body::new),
                                    );
                                }
                                let response = service
                                    .oneshot(request)
                                    .await
                                    .expect("native MCP service response");
                                Ok::<_, std::convert::Infallible>(
                                    response.map(axum::body::Body::new),
                                )
                            }
                        });
                        let _ = http1::Builder::new()
                            .serve_connection(TokioIo::new(stream), guard)
                            .await;
                    });
                }
            });
        });
        Ok(Self {
            address,
            bearer,
            receipt,
            cancellation,
            join: Some(join),
        })
    }

    fn url(&self) -> String {
        format!("http://{}/mcp", self.address)
    }

    fn take_receipt(&self) -> Option<NativeMcpReportingReceipt> {
        self.receipt.try_recv().ok()
    }

    fn stop(mut self) {
        self.cancellation.cancel();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn native_mcp_transport_denial<B>(
    expected_bearer: &str,
    allowed_host: &str,
    request: &hyper::Request<B>,
) -> Option<StatusCode> {
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.strip_prefix("Bearer ") == Some(expected_bearer));
    if !authorized {
        return Some(StatusCode::UNAUTHORIZED);
    }
    (request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(allowed_host))
    .then_some(StatusCode::FORBIDDEN)
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
    personality: Option<CodexPersonality>,
    execution: NativeProfileExecution,
    login_attempt: NativeProfileLoginAttempt,
    setup_attempt: NativeProfileSetupAttempt,
    sandbox_adoption: NativeProfileSandboxAdoption,
    sandbox_adoption_confirmation: NativeProfileSandboxAdoptionConfirmation,
    full_access_canary_attempt: NativeProfileFullAccessCanaryAttempt,
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
            personality: value.personality,
            execution: value.execution,
            login_attempt: value.login_attempt,
            setup_attempt: value.setup_attempt,
            sandbox_adoption: value.sandbox_adoption,
            sandbox_adoption_confirmation: value.sandbox_adoption_confirmation,
            full_access_canary_attempt: value.full_access_canary_attempt,
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

pub struct NativeProfileService {
    database: Arc<ActiveDatabase>,
    #[cfg(test)]
    database_path: Option<PathBuf>,
    dedicated_root: PathBuf,
    cli: Arc<dyn NativeCliPort>,
    login_children: Mutex<HashMap<String, Box<dyn NativeCliChild>>>,
    setup_children: Mutex<HashMap<String, Box<dyn NativeCliChild>>>,
    full_access_canary_children: Mutex<HashMap<String, Box<dyn NativeCliChild>>>,
    mcp_dispatch_claims: Mutex<HashMap<String, String>>,
    operation_gate: Mutex<()>,
}

impl NativeProfileService {
    pub fn new(database: Arc<ActiveDatabase>, app_data_dir: PathBuf) -> Self {
        Self {
            database,
            #[cfg(test)]
            database_path: None,
            dedicated_root: app_data_dir.join("native-codex-homes"),
            cli: Arc::new(SystemNativeCliPort {
                program: crate::runtime::providers::codex::resolve_program("codex".into()),
            }),
            login_children: Mutex::new(HashMap::new()),
            setup_children: Mutex::new(HashMap::new()),
            full_access_canary_children: Mutex::new(HashMap::new()),
            mcp_dispatch_claims: Mutex::new(HashMap::new()),
            operation_gate: Mutex::new(()),
        }
    }

    #[cfg(test)]
    pub fn open(database_path: PathBuf, app_data_dir: PathBuf) -> Result<Self, String> {
        let _gate = NATIVE_PROFILE_OPEN_GATE
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| "Native profile initialization supervision is unavailable")?;
        let database = ActiveDatabase::open(&database_path, |connection| {
            connection
                .execute_batch(NATIVE_PROFILE_SCHEMA)
                .map_err(|error| format!("Unable to initialize native profile schema: {error}"))
        })
        .map_err(|error| error.to_string())?;
        let mut service = Self::new(Arc::new(database), app_data_dir);
        service.database_path = Some(database_path);
        Ok(service)
    }

    #[cfg(test)]

    fn connection(&self) -> Result<Connection, String> {
        let connection = Connection::open(
            self.database_path
                .as_ref()
                .ok_or("Native profile test database path is unavailable")?,
        )
        .map_err(|error| error.to_string())?;
        crate::persistence::configure_writer_connection(&connection)
            .map_err(|error| error.to_string())?;
        Ok(connection)
    }

    fn read<T>(
        &self,
        operation: &'static str,
        read: impl FnOnce(&Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        self.database
            .read(operation, read)
            .map_err(|error| error.into_string())
    }

    fn write<T>(
        &self,
        operation: &'static str,
        write: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        self.database
            .write(operation, write)
            .map_err(|error| error.into_string())
    }

    pub fn query(&self) -> Result<NativeProfileQueryDto, String> {
        let profiles = self.read("load native profiles for reconciliation", |connection| {
            load_profiles(connection)
        })?;
        for profile in profiles {
            self.revalidate(&profile)?;
            self.reconcile_login_attempt(&profile.id)?;
        }
        let profiles = self.read("load native profiles", |connection| {
            load_profiles(connection)
        })?;
        Ok(NativeProfileQueryDto {
            contract: PROFILE_QUERY_CONTRACT,
            profiles: profiles.into_iter().map(Into::into).collect(),
        })
    }

    pub fn register_existing(&self, supplied_home: &str) -> Result<NativeProfileDto, String> {
        let home = validated_absolute_directory(supplied_home)?;
        if home.join(MARKER_FILE).exists() {
            return Err(
                "An application-owned Codex home cannot be registered as user-owned".into(),
            );
        }
        self.insert_profile(home, Ownership::RegisteredExisting)
    }

    pub fn create_dedicated(&self) -> Result<NativeProfileDto, String> {
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
        let inserted = self.write("register native profile", |transaction| {
        let inserted = transaction
            .execute(
                "INSERT OR IGNORE INTO native_codex_profiles (id,canonical_home_path,filesystem_identity,ownership,lifecycle,created_at,updated_at) VALUES (?1,?2,?3,?4,'active',?5,?5)",
                params![id, home.to_string_lossy(), identity, ownership.database(), now],
            )
            .map_err(|error| format!("Unable to register Codex home: {error}"))?;
        if inserted == 0 {
            return Ok(false);
        }
            transaction.execute(
                "INSERT INTO native_codex_profile_readiness (profile_id,authentication,sandbox_initialization,workspace_write_canary,mcp_reporting,observed_at) VALUES (?1,'unknown','unknown','not_run','not_assessed',?2)",
                params![id, now],).map_err(|error| format!("Unable to initialize profile readiness: {error}"))?;
        transaction
            .execute(
                "INSERT INTO native_codex_profile_execution_modes (profile_id,selected_mode,updated_at) VALUES (?1,'danger_full_access',?2)",
                params![id, now],
            )
            .map_err(| error| format!("Unable to initialize profile execution mode: {error}"))?;
        Ok(true)
        })?;
        if !inserted {
            return self
                .profile_by_home(&home)?
                .map(Into::into)
                .ok_or_else(|| "Concurrent profile registration did not produce a profile".into());
        }
        self.profile(&id).map(Into::into)
    }

    pub fn select(&self, id: &str) -> Result<NativeProfileDto, String> {
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
        self.write("select native profile", |transaction| {
            transaction
                .execute("UPDATE native_codex_profiles SET selected_at=NULL", [])
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "UPDATE native_codex_profiles SET selected_at=?2,updated_at=?2 WHERE id=?1",
                    params![id, now],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        })?;
        self.profile(id).map(Into::into)
    }

    pub fn select_execution_mode(
        &self,
        id: &str,
        mode: ExecutionMode,
    ) -> Result<NativeProfileDto, String> {
        self.require_active(id)?;
        self.write("select native execution mode", |transaction| {
            transaction.execute(
                "UPDATE native_codex_profile_execution_modes SET selected_mode=?2,updated_at=?3 WHERE profile_id=?1",
                params![id, mode.database(), Utc::now().to_rfc3339()],
            )
            .map_err(|error| format!("Unable to select native execution mode: {error}"))?;
            Ok(())
        })?;
        self.profile(id).map(Into::into)
    }

    pub fn set_personality(
        &self,
        id: &str,
        personality: Option<CodexPersonality>,
    ) -> Result<NativeProfileDto, String> {
        self.require_active(id)?;
        self.write("set native Codex personality", |transaction| {
            transaction.execute(
                "INSERT INTO native_codex_profile_preferences (profile_id,personality,updated_at) VALUES (?1,?2,?3) ON CONFLICT(profile_id) DO UPDATE SET personality=excluded.personality,updated_at=excluded.updated_at",
                params![id, personality_database(personality), Utc::now().to_rfc3339()],
            )
            .map_err(|error| format!("Unable to set native Codex personality: {error}"))?;
            Ok(())
        })?;
        self.profile(id).map(Into::into)
    }

    pub fn profile_personality(&self, id: &str) -> Result<Option<CodexPersonality>, String> {
        Ok(self.profile(id)?.personality)
    }

    /// This is the only durable opt-in for the dangerous mode. Authentication, readiness,
    /// ownership, and an earlier launch never create it.
    pub fn authorize_danger_full_access(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_selected_active(id)?;
        if profile.execution.selected_mode != ExecutionMode::DangerFullAccess {
            return Err("Danger full access must be selected before it can be authorized".into());
        }
        let surface = self.cli.surface().map_err(|_| {
            self.set_attention(id, "cli", Some("codex_cli_surface_unsupported"), false)
                .ok();
            "The resolved Codex CLI surface is unsupported for danger authorization".to_string()
        })?;
        if !surface.provenance.danger_full_access_supported
            || !surface.provenance.danger_unrestricted_network_supported
            || !surface.provenance.non_interactive_approval_supported
        {
            self.set_attention(
                id,
                "cli",
                Some("codex_cli_danger_launch_surface_unsupported"),
                false,
            )?;
            return Err(
                "The resolved Codex CLI cannot establish the authorized danger launch contract"
                    .into(),
            );
        }
        self.write("authorize native danger full access", |transaction| {
            transaction.execute(
                "INSERT INTO native_codex_profile_mode_authorizations (profile_id,mode,filesystem_identity,authority_scope,authority_version,authorization_correlation_id,authorized_at,revoked_at) VALUES (?1,'danger_full_access',?2,?3,?4,?5,?6,NULL) ON CONFLICT(profile_id,mode) DO UPDATE SET filesystem_identity=excluded.filesystem_identity,authority_scope=excluded.authority_scope,authority_version=excluded.authority_version,authorization_correlation_id=excluded.authorization_correlation_id,authorized_at=excluded.authorized_at,revoked_at=NULL",
                params![id, profile.identity, DANGER_AUTHORITY_SCOPE, DANGER_AUTHORITY_VERSION, format!("native-danger-authorization-{}", Uuid::new_v4()), Utc::now().to_rfc3339()],
            )
            .map_err(|error| format!("Unable to record danger full access authorization: {error}"))?;
            Ok(())
        })?;
        self.profile(id).map(Into::into)
    }

    pub fn revoke_danger_full_access(&self, id: &str) -> Result<NativeProfileDto, String> {
        self.require_selected_active(id)?;
        self.write("revoke native danger full access", |transaction| {
            transaction.execute(
                "UPDATE native_codex_profile_mode_authorizations SET revoked_at=?2 WHERE profile_id=?1 AND mode='danger_full_access' AND revoked_at IS NULL",
                params![id, Utc::now().to_rfc3339()],
            )
            .map_err(|error| format!("Unable to revoke danger full access authorization: {error}"))?;
            Ok(())
        })?;
        self.profile(id).map(Into::into)
    }

    pub fn request_login(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_selected_active(id)?;
        self.reconcile_login_attempt(id)?;
        if self
            .read("load pending native login attempt", |connection| {
                load_pending_login_attempt(connection, id)
            })?
            .is_some()
        {
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
        let inserted = self.write("create native login attempt", |transaction| transaction.execute(
            "INSERT OR IGNORE INTO native_codex_profile_login_attempts (attempt_id,profile_id,filesystem_identity,executable,version,correlation_id,state,browser_handoff,requested_at) VALUES (?1,?2,?3,?4,?5,?6,'pending','unobserved',?7)",
            params![attempt.attempt_id, attempt.profile_id, attempt.filesystem_identity, surface.provenance.executable, surface.provenance.version, format!("native-login-{}", Uuid::new_v4()), now.to_rfc3339()],
        ).map_err(|error| format!("Unable to persist native browser login attempt: {error}")))?;
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
        self.write("accept native login launch", |transaction| {
            transaction.execute(
            "UPDATE native_codex_profile_login_attempts SET launch_accepted_at=?2 WHERE attempt_id=?1 AND state='pending'",
            params![attempt.attempt_id, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string())?;
            Ok(())
        })?;
        self.set_attention(
            id,
            "authentication",
            Some("browser_login_attempt_pending"),
            false,
        )?;
        self.profile(id).map(Into::into)
    }

    pub fn refresh_readiness(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_active(id)?;
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

    /// Opens only a registered profile directory through the platform's normal file manager.
    /// The caller supplies an opaque profile id, never an arbitrary filesystem path.
    pub fn open_in_explorer(&self, id: &str) -> Result<(), String> {
        let profile = self.require_active(id)?;
        Command::new("explorer.exe")
            .arg(profile.home)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Unable to open the Codex profile in Explorer: {error}"))
    }

    pub fn request_sandbox_initialization(&self, id: &str) -> Result<NativeProfileDto, String> {
        self.require_selected_active(id)?;
        self.reconcile_setup_attempts(id)?;
        let profile = self.require_selected_active(id)?;
        self.start_setup_attempt(&profile, SetupPhase::SandboxInitialization)?;
        self.profile(id).map(Into::into)
    }

    /// A person must explicitly confirm the Windows/UAC stage. A successful setup process only
    /// records that the application-owned request completed; it never establishes this fact.
    pub fn confirm_sandbox_initialization(&self, id: &str) -> Result<NativeProfileDto, String> {
        self.require_selected_active(id)?;
        self.reconcile_setup_attempts(id)?;
        self.require_selected_active(id)?;
        let latest_state = self.read("load latest native sandbox setup", |connection| connection
            .query_row(
                "SELECT state FROM native_codex_profile_setup_attempts WHERE profile_id=?1 AND phase='sandbox_initialization' ORDER BY requested_at DESC,attempt_id DESC LIMIT 1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string()))?;
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
    pub fn verify_preprovisioned_sandbox(&self, id: &str) -> Result<NativeProfileDto, String> {
        let profile = self.require_selected_active(id)?;
        let surface = self.cli.surface().map_err(|_| {
            self.set_attention(id, "cli", Some("codex_cli_surface_unsupported"), false)
                .ok();
            "The resolved Codex CLI surface is unsupported for external sandbox verification"
                .to_string()
        })?;
        let elevated_mode_observed = observe_elevated_windows_sandbox_mode(&profile.home)?;
        let verified = surface.provenance.workspace_sandbox_supported
            && surface.windows_sandbox_setup_supported
            && elevated_mode_observed;
        self.write("record native sandbox verification", |transaction| {
        transaction.execute(
            "INSERT INTO native_codex_profile_sandbox_adoptions (profile_id,filesystem_identity,executable,version,workspace_sandbox_supported,windows_sandbox_setup_supported,correlation_id,observed_at,state,elevated_mode_observed) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(profile_id) DO UPDATE SET filesystem_identity=excluded.filesystem_identity,executable=excluded.executable,version=excluded.version,workspace_sandbox_supported=excluded.workspace_sandbox_supported,windows_sandbox_setup_supported=excluded.windows_sandbox_setup_supported,correlation_id=excluded.correlation_id,observed_at=excluded.observed_at,state=excluded.state,elevated_mode_observed=excluded.elevated_mode_observed",
            params![id, profile.identity, surface.provenance.executable, surface.provenance.version, surface.provenance.workspace_sandbox_supported as i64, surface.windows_sandbox_setup_supported as i64, format!("native-sandbox-adoption-{}", Uuid::new_v4()), Utc::now().to_rfc3339(), if verified { "verified" } else { "not_verified" }, elevated_mode_observed as i64],
        ).map_err(|error| format!("Unable to persist external sandbox verification: {error}"))?;
        transaction.execute(
            "UPDATE native_codex_profile_sandbox_adoption_confirmations SET state='invalidated',invalidated_at=COALESCE(invalidated_at,?2) WHERE profile_id=?1 AND state='confirmed'",
            params![id, Utc::now().to_rfc3339()],
        ).map_err(|error| format!("Unable to invalidate superseded external sandbox adoption confirmation: {error}"))?;
        Ok(())
        })?;
        self.update_readiness(
            id,
            None,
            Some("attention_required"),
            Some("blocked"),
            None,
            Some((
                "sandbox",
                Some(if verified {
                    "external_sandbox_provisioning_verified_explicit_adoption_confirmation_required"
                } else {
                    "external_sandbox_provisioning_not_verified"
                }),
            )),
        )?;
        self.profile(id).map(Into::into)
    }

    /// This is an explicit product acknowledgment of a verified external postcondition. It does
    /// not claim that this product requested setup or observed a UAC interaction.
    pub fn confirm_preprovisioned_sandbox_adoption(
        &self,
        id: &str,
    ) -> Result<NativeProfileDto, String> {
        let profile = self.require_selected_active(id)?;
        let adoption = self.read("load native sandbox adoption", |connection| {
            load_sandbox_adoption(connection, id, &profile.identity)
        })?;
        let surface = self.cli.surface().map_err(|_| {
            "The resolved Codex CLI surface is unsupported for external sandbox adoption"
                .to_string()
        })?;
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
        let adoption_correlation = adoption
            .correlation_id
            .ok_or("The external sandbox observation has no durable correlation")?;
        let now = Utc::now().to_rfc3339();
        self.write("confirm native sandbox adoption", |transaction| transaction.execute(
            "INSERT INTO native_codex_profile_sandbox_adoption_confirmations (profile_id,filesystem_identity,adoption_correlation_id,confirmation_correlation_id,confirmed_at,state,invalidated_at) VALUES (?1,?2,?3,?4,?5,'confirmed',NULL) ON CONFLICT(profile_id) DO UPDATE SET filesystem_identity=excluded.filesystem_identity,adoption_correlation_id=excluded.adoption_correlation_id,confirmation_correlation_id=excluded.confirmation_correlation_id,confirmed_at=excluded.confirmed_at,state='confirmed',invalidated_at=NULL",
            params![id, profile.identity, adoption_correlation, format!("native-sandbox-adoption-confirmation-{}", Uuid::new_v4()), now],
        ).map_err(|error| format!("Unable to persist external sandbox adoption confirmation: {error}")))?;
        self.update_readiness(
            id,
            None,
            Some("initialized"),
            None,
            None,
            Some((
                "sandbox",
                Some("external_sandbox_adoption_confirmed_product_uac_unobserved"),
            )),
        )?;
        self.profile(id).map(Into::into)
    }

    pub fn run_workspace_write_canary(&self, id: &str) -> Result<NativeProfileDto, String> {
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

    pub fn probe_mcp_reporting(&self, id: &str) -> Result<NativeProfileDto, String> {
        self.begin_mcp_reporting_probe(id)?;
        self.profile(id).map(Into::into)
    }

    /// Performs one scoped reporting exchange only for an already-durable pending request. It
    /// never creates a request, retries a failed exchange, or infers a receipt from process exit.
    pub fn reconcile_pending_mcp_reporting(&self, id: &str) -> Result<NativeProfileDto, String> {
        let gate = self
            .operation_gate
            .lock()
            .map_err(|_| "Native profile operation supervision is unavailable")?;
        let profile = self.require_selected_active_while_gated(id)?;
        self.expire_mcp_probe(id)?;
        let Some(claimed) = self.claim_pending_mcp_reporting_probe(id)? else {
            return self.profile(id).map(Into::into);
        };
        let authority = claimed.authority;
        if authority.profile_id != profile.id {
            return Err("The application-owned MCP reporting request has invalid authority".into());
        }
        self.mcp_dispatch_claims
            .lock()
            .map_err(|_| "Native MCP reporting receipt supervision is unavailable")?
            .insert(id.into(), claimed.claim_id.clone());
        let server = match NativeMcpReportingServer::start(authority.clone()) {
            Ok(server) => server,
            Err(error) => {
                self.release_mcp_dispatch_claim(id);
                self.cancel_pending_mcp_reporting_probe(
                    id,
                    &authority,
                    &claimed.claim_id,
                    "mcp_reporting_dispatch_unavailable",
                )?;
                return Err(error);
            }
        };
        let variable = format!(
            "CODEX_ORCHESTRATOR_NATIVE_MCP_REPORTING_{}",
            Uuid::new_v4().simple()
        );
        let mut args = vec![
            "exec".into(),
            "--json".into(),
            "--strict-config".into(),
            "--ignore-user-config".into(),
            "--ignore-rules".into(),
            "--sandbox".into(),
            "workspace-write".into(),
            "--cd".into(),
            authority.probe_root.to_string_lossy().into_owned(),
            "--skip-git-repo-check".into(),
        ];
        let configuration = [
            format!("mcp_servers.{MCP_REPORTING_SERVER}.url={:?}", server.url()),
            format!("mcp_servers.{MCP_REPORTING_SERVER}.bearer_token_env_var={variable:?}"),
            format!("mcp_servers.{MCP_REPORTING_SERVER}.enabled_tools=[{MCP_REPORTING_TOOL:?}]"),
            format!("mcp_servers.{MCP_REPORTING_SERVER}.required=true"),
            format!("mcp_servers.{MCP_REPORTING_SERVER}.default_tools_approval_mode=\"approve\""),
            format!("mcp_servers.{MCP_REPORTING_SERVER}.startup_timeout_sec=10"),
            format!("mcp_servers.{MCP_REPORTING_SERVER}.tool_timeout_sec=300"),
        ];
        for value in configuration {
            args.push("-c".into());
            args.push(value);
        }
        args.push(format!(
            "Use only the {MCP_REPORTING_CAPABILITY} MCP capability. Call {MCP_REPORTING_SERVER}.{MCP_REPORTING_TOOL} exactly once with {{}}. Do not read or write files, report profile content, or perform any other work."
        ));
        let outcome = self.cli.run(&NativeCliInvocation {
            args,
            cwd: authority.probe_root.clone(),
            codex_home: profile.home.clone(),
            environment: {
                let mut environment = native_windows_cli_environment(&profile.home);
                environment.push((variable, server.bearer.clone()));
                environment
            },
            sandbox_receipt: None,
            sandbox_command_file: None,
        });
        let receipt = server.take_receipt();
        server.stop();

        if let Some(receipt) = receipt {
            self.require_selected_active_while_gated(id)?;
            let result = self.record_mcp_reporting_receipt(id, &receipt);
            self.release_mcp_dispatch_claim(id);
            drop(gate);
            return result;
        }
        let attention = if outcome.is_err() {
            "mcp_reporting_dispatch_unavailable"
        } else {
            "mcp_reporting_dispatch_completed_without_receipt"
        };
        self.release_mcp_dispatch_claim(id);
        self.cancel_pending_mcp_reporting_probe(id, &authority, &claimed.claim_id, attention)?;
        drop(gate);
        if outcome.is_err() {
            return Err("The application-owned MCP reporting exchange is unavailable".into());
        }
        Err("The MCP reporting exchange completed without a correlated receipt".into())
    }

    pub fn begin_mcp_reporting_probe(
        &self,
        id: &str,
    ) -> Result<NativeMcpReportingProbeAuthority, String> {
        let gate = self
            .operation_gate
            .lock()
            .map_err(|_| "Native profile operation supervision is unavailable")?;
        self.require_selected_active_while_gated(id)?;
        let root = self.probe_root(id);
        let authority = self.write("begin native MCP reporting probe", |transaction| {
        let now = Utc::now();
        transaction.execute(
            "UPDATE native_codex_profile_mcp_probes SET state='expired' WHERE profile_id=?1 AND state IN ('pending','dispatching') AND deadline_at <= ?2",
            params![id, now.to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        if let Some(authority) = load_pending_mcp_probe(&transaction, id)? {
            return Ok(authority);
        }
        if self.has_current_mcp_reporting_probe(transaction, id)? {
            return Err(
                "The application-owned MCP reporting probe is already pending or claimed".into(),
            );
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
        self.write_attention(transaction,
            id,
            "mcp_reporting",
            Some("mcp_reporting_probe_pending_application_receipt"),
        )?;
        Ok(authority)
        })?;
        drop(gate);
        Ok(authority)
    }

    pub fn record_mcp_reporting_receipt(
        &self,
        id: &str,
        receipt: &NativeMcpReportingReceipt,
    ) -> Result<NativeProfileDto, String> {
        self.require_selected_active(id)?;
        let dispatch_claim_id = self
            .mcp_dispatch_claims
            .lock()
            .map_err(|_| "Native MCP reporting receipt supervision is unavailable")?
            .get(id)
            .cloned()
            .ok_or("The application-owned MCP reporting receipt has no current dispatch claim")?;
        self.write("settle native MCP reporting receipt", |transaction| {
        let now = Utc::now();
        let expired = transaction.execute(
            "UPDATE native_codex_profile_mcp_probes SET state='expired' WHERE profile_id=?1 AND state IN ('pending','dispatching') AND deadline_at <= ?2",
            params![id, now.to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        if expired != 0 {
            transaction.execute(
                "UPDATE native_codex_profile_readiness SET mcp_reporting='not_assessed',observed_at=?2 WHERE profile_id=?1",
                params![id, now.to_rfc3339()],
            ).map_err(|error| error.to_string())?;
            self.write_attention(transaction,
                id,
                "mcp_reporting",
                Some("mcp_reporting_probe_expired"),)?;
            return Err("The application-owned MCP reporting probe has expired".into());
        }
        let transitioned = transaction.execute(
            "UPDATE native_codex_profile_mcp_probes SET state='received',received_at=?2 WHERE profile_id=?1 AND state='dispatching' AND dispatch_claim_id=?8 AND correlation_id=?3 AND expected_capability=?4 AND expected_server=?5 AND expected_tool=?6 AND expected_probe_root=?7 AND deadline_at > ?2",
            params![id, now.to_rfc3339(), receipt.correlation_id, receipt.capability, receipt.server, receipt.tool, receipt.probe_root.to_string_lossy(), dispatch_claim_id],
        ).map_err(|error| error.to_string())?;
        if transitioned != 1 {
            return Err(
                "MCP reporting receipt does not match one current application-owned probe"
                    .into(),
            );
        }
        transaction.execute(
            "UPDATE native_codex_profile_readiness SET mcp_reporting='ready',observed_at=?2 WHERE profile_id=?1",
            params![id, now.to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        self.write_attention(transaction, id, "mcp_reporting", None)?;
        Ok(())
        })?;
        self.profile(id).map(Into::into)
    }

    fn claim_pending_mcp_reporting_probe(
        &self,
        id: &str,
    ) -> Result<Option<ClaimedNativeMcpReportingProbe>, String> {
        self.write("claim native MCP reporting probe", |transaction| {
        let Some(authority) = load_pending_mcp_probe(&transaction, id)? else {
            return Ok(None);
        };
        let claim_id = format!("native-mcp-dispatch-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        let claimed = transaction
            .execute(
                "UPDATE native_codex_profile_mcp_probes SET state='dispatching',dispatch_claim_id=?2,dispatch_claimed_at=?3 WHERE profile_id=?1 AND state='pending' AND correlation_id=?4 AND expected_capability=?5 AND expected_server=?6 AND expected_tool=?7 AND expected_probe_root=?8",
                params![id, claim_id, now, authority.correlation_id, authority.capability, authority.server, authority.tool, authority.probe_root.to_string_lossy()],
            )
            .map_err(|error| error.to_string())?;
        if claimed != 1 {
            return Err("The application-owned MCP reporting probe could not be claimed".into());
        }
        self.write_attention(transaction,
            id,
            "mcp_reporting",
            Some("mcp_reporting_dispatch_claimed_receipt_pending"),
        )?;
        Ok(Some(ClaimedNativeMcpReportingProbe {
            authority,
            claim_id,
        }))
        })
    }

    fn has_current_mcp_reporting_probe(
        &self,
        connection: &Connection,
        id: &str,
    ) -> Result<bool, String> {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM native_codex_profile_mcp_probes WHERE profile_id=?1 AND state IN ('pending','dispatching'))",
                params![id],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value != 0)
            .map_err(|error| error.to_string())
    }

    fn release_mcp_dispatch_claim(&self, id: &str) {
        if let Ok(mut claims) = self.mcp_dispatch_claims.lock() {
            claims.remove(id);
        }
    }

    /// Terminalizes an attempted bridge that supplied no receipt. A later invocation observes
    /// this durable cancellation rather than starting another exchange or replacement request.
    fn cancel_pending_mcp_reporting_probe(
        &self,
        id: &str,
        authority: &NativeMcpReportingProbeAuthority,
        claim_id: &str,
        attention: &'static str,
    ) -> Result<(), String> {
        self.write("cancel native MCP reporting probe", |transaction| {
        let now = Utc::now().to_rfc3339();
        let transitioned = transaction
            .execute(
                "UPDATE native_codex_profile_mcp_probes SET state='cancelled' WHERE profile_id=?1 AND state='dispatching' AND dispatch_claim_id=?2 AND correlation_id=?3 AND expected_capability=?4 AND expected_server=?5 AND expected_tool=?6 AND expected_probe_root=?7",
                params![id, claim_id, authority.correlation_id, authority.capability, authority.server, authority.tool, authority.probe_root.to_string_lossy()],
            )
            .map_err(|error| error.to_string())?;
        if transitioned != 1 {
            return Err("The application-owned MCP reporting probe could not be terminalized".into(),);
        }
        transaction
            .execute(
                "UPDATE native_codex_profile_readiness SET mcp_reporting='probe_failed',observed_at=?2 WHERE profile_id=?1",
                params![id, now],
            )
            .map_err(|error| error.to_string())?;
        self.write_attention(transaction, id, "mcp_reporting", Some(attention))?;
        Ok(())
        })
    }

    /// Produces a command only after all mode-specific authority is independently valid. It does
    /// not start Codex and consequently cannot be mistaken for launch acceptance or activity.
    pub fn project_launch(
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
        let surface = self.cli.surface().map_err(|_| {
            self.set_attention(id, "cli", Some("codex_cli_surface_unsupported"), false)
                .ok();
            "The resolved Codex CLI surface is unsupported for native execution".to_string()
        })?;
        let mode = profile.execution.selected_mode;
        match mode {
            ExecutionMode::WorkspaceWrite => {
                if !target.network_disabled {
                    return Err("Workspace-write execution requires an application-owned target with network disabled".into());
                }
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
                if target.network_disabled {
                    return Err("Danger full access requires the explicitly authorized unrestricted-network target".into());
                }
                if !surface.provenance.danger_full_access_supported
                    || !surface.provenance.danger_unrestricted_network_supported
                    || !surface.provenance.non_interactive_approval_supported
                    || !profile.execution.danger_full_access_authorized
                {
                    self.set_attention(
                        id,
                        "cli",
                        Some("codex_cli_danger_launch_surface_unsupported"),
                        false,
                    )?;
                    return Err(
                        "Danger full access launch authority is not currently established".into(),
                    );
                }
                if !surface.workspace_launch_flags_supported
                    || !surface.workspace_launch_project_config_isolated
                {
                    self.set_attention(
                        id,
                        "cli",
                        Some("codex_cli_danger_launch_project_config_unsupported"),
                        false,
                    )?;
                    return Err(
                        "The resolved Codex CLI cannot exclude application-unauthorized project configuration from a danger launch".into(),
                    );
                }
            }
        }
        let workspace_project_trust_override =
            workspace_project_trust_override(&target.working_root)?;
        let mut arguments = vec![
            "exec".into(),
            "--json".into(),
            "--strict-config".into(),
            "--sandbox".into(),
            mode.codex_sandbox().into(),
            "--cd".into(),
            target.working_root.to_string_lossy().into_owned(),
            "--skip-git-repo-check".into(),
            "--ignore-user-config".into(),
            "--ignore-rules".into(),
        ];
        for value in [
            workspace_project_trust_override,
            "project_root_markers=[]".into(),
            "project_doc_max_bytes=0".into(),
            "mcp_servers={}".into(),
            "features.hooks=false".into(),
            "features.plugins=false".into(),
            "features.apps=false".into(),
        ] {
            arguments.push("--config".into());
            arguments.push(value);
        }
        if mode == ExecutionMode::WorkspaceWrite {
            for value in [
                "sandbox_workspace_write.network_access=false",
                "sandbox_workspace_write.writable_roots=[]",
                "sandbox_workspace_write.exclude_tmpdir_env_var=true",
                "sandbox_workspace_write.exclude_slash_tmp=true",
            ] {
                arguments.push("--config".into());
                arguments.push(value.into());
            }
        } else {
            arguments.push("--dangerously-bypass-approvals-and-sandbox".into());
        }
        Ok(NativeLaunchProjectionDto {
            profile_id: profile.id,
            mode,
            executable: surface.provenance.executable,
            version: surface.provenance.version,
            arguments,
            working_root: target.working_root.to_string_lossy().into_owned(),
            requested_network_disabled: target.network_disabled,
            effective_network_enforced: mode == ExecutionMode::WorkspaceWrite,
            non_interactive_approval: mode == ExecutionMode::DangerFullAccess,
            windows_uac_authority: "not_granted",
        })
    }

    pub fn project_full_access_canary(
        &self,
        id: &str,
    ) -> Result<NativeFullAccessCanaryProjectionDto, String> {
        let target =
            NativeLaunchTarget::application_owned(self.full_access_canary_work_root(id)?, false)?;
        let launch = self.project_launch(id, &target)?;
        if launch.mode != ExecutionMode::DangerFullAccess {
            return Err(
                "The full-access canary requires the selected danger full access mode".into(),
            );
        }
        let sentinel_path = self
            .full_access_canary_receipt_root(id)?
            .join(FULL_ACCESS_CANARY_RECEIPT_FILE);
        Ok(NativeFullAccessCanaryProjectionDto {
            launch,
            sentinel_path: sentinel_path.to_string_lossy().into_owned(),
            evidence_state: "not_run",
        })
    }

    /// Starts only the bounded, application-owned canary. Provider activity and semantic workflow
    /// completion remain outside this receipt; only the exact sentinel settles the canary fact.
    pub fn run_danger_full_access_canary(&self, id: &str) -> Result<NativeProfileDto, String> {
        self.reconcile_full_access_canary(id)?;
        if self
            .read("load pending native full access canary", |connection| {
                load_pending_full_access_canary(connection, id)
            })?
            .is_some()
        {
            return self.profile(id).map(Into::into);
        }
        let profile = self.require_selected_active(id)?;
        let projection = self.project_full_access_canary(id)?;
        let sentinel = PathBuf::from(&projection.sentinel_path);
        if let Err(error) = fs::remove_file(&sentinel) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err("Unable to clear the owned full-access canary receipt".into());
            }
        }
        let authorization = &profile.execution.danger_authorization;
        let authorization_correlation_id = authorization
            .correlation_id
            .clone()
            .ok_or("The current danger authorization has no durable correlation")?;
        if authorization.disposition != "authorized"
            || authorization.authority_scope.as_deref() != Some(DANGER_AUTHORITY_SCOPE)
            || authorization.authority_version.as_deref() != Some(DANGER_AUTHORITY_VERSION)
        {
            return Err(
                "A current explicit danger authorization is required before the canary".into(),
            );
        }
        let prompt = format!(
            "Create only the application-owned sentinel file at {FULL_ACCESS_CANARY_RECEIPT_RELATIVE_PATH} with the exact contents {FULL_ACCESS_CANARY_RECEIPT}, then stop."
        );
        let now = Utc::now();
        let attempt = PendingFullAccessCanary {
            attempt_id: format!("native-full-access-canary-{}", Uuid::new_v4()),
            profile_id: profile.id.clone(),
            filesystem_identity: profile.identity,
            executable: projection.launch.executable.clone(),
            version: projection.launch.version.clone(),
            sentinel_path: sentinel.clone(),
            authorization_correlation_id,
            authorization_version: DANGER_AUTHORITY_VERSION.into(),
            correlation_id: format!("native-full-access-canary-correlation-{}", Uuid::new_v4()),
            deadline_at: now + Duration::seconds(FULL_ACCESS_CANARY_TIMEOUT_SECONDS),
        };
        self.write("create native full access canary", |transaction| transaction.execute(
            "INSERT INTO native_codex_profile_full_access_canaries (attempt_id,profile_id,filesystem_identity,mode,executable,version,sentinel_path,authorization_correlation_id,authorization_version,correlation_id,state,requested_at,deadline_at,process_activity,provider_activity,terminal_classification,receipt_observed,cleanup_disposition) VALUES (?1,?2,?3,'danger_full_access',?4,?5,?6,?7,?8,?9,'pending',?10,?11,'unobserved','unobserved','not_observed',0,'pending')",
            params![attempt.attempt_id, attempt.profile_id, attempt.filesystem_identity, attempt.executable, attempt.version, attempt.sentinel_path.to_string_lossy(), attempt.authorization_correlation_id, attempt.authorization_version, attempt.correlation_id, now.to_rfc3339(), attempt.deadline_at.to_rfc3339()],
        ).map_err(|error| format!("Unable to persist full-access canary request: {error}")))?;
        let mut args = projection.launch.arguments;
        args.push(prompt);
        let invocation = NativeCliInvocation {
            args,
            cwd: PathBuf::from(&projection.launch.working_root),
            codex_home: profile.home.clone(),
            environment: native_windows_cli_environment(&profile.home),
            sandbox_receipt: Some(sentinel),
            sandbox_command_file: None,
        };
        match self.cli.start(&invocation) {
            Ok(mut child) => match self.full_access_canary_children.lock() {
                Ok(mut children) => {
                    children.insert(attempt.attempt_id.clone(), child);
                    self.write("accept native full access canary launch", |transaction| {
                        transaction.execute(
                        "UPDATE native_codex_profile_full_access_canaries SET launch_accepted_at=?2,process_activity='launch_accepted' WHERE attempt_id=?1 AND state='pending'",
                        params![attempt.attempt_id, Utc::now().to_rfc3339()],
                    ).map_err(|error| error.to_string())?;
                        Ok(())
                    })?;
                    self.profile(id).map(Into::into)
                }
                Err(_) => {
                    let _ = child.terminate();
                    self.settle_full_access_canary(
                        &attempt,
                        "cancelled",
                        "cancelled",
                        None,
                        false,
                    )?;
                    Err("Native full-access canary supervision is unavailable".into())
                }
            },
            Err(_) => {
                self.settle_full_access_canary(
                    &attempt,
                    "launch_failed",
                    "launch_failed",
                    None,
                    false,
                )?;
                Err("Unable to start the supported full-access canary".into())
            }
        }
    }

    fn reconcile_full_access_canary(&self, id: &str) -> Result<(), String> {
        let Some(attempt) = self.read("load pending native full access canary", |connection| {
            load_pending_full_access_canary(connection, id)
        })?
        else {
            return Ok(());
        };
        let current = self.require_selected_active(id);
        let surface = self.cli.surface();
        let authority_valid = current.as_ref().is_ok_and(|profile| {
            profile.execution.selected_mode == ExecutionMode::DangerFullAccess
                && profile.execution.danger_full_access_authorized
                && profile.identity == attempt.filesystem_identity
                && profile
                    .execution
                    .danger_authorization
                    .correlation_id
                    .as_deref()
                    == Some(attempt.authorization_correlation_id.as_str())
                && profile
                    .execution
                    .danger_authorization
                    .authority_version
                    .as_deref()
                    == Some(attempt.authorization_version.as_str())
        }) && surface.as_ref().is_ok_and(|surface| {
            surface.provenance.danger_full_access_supported
                && surface.provenance.danger_unrestricted_network_supported
                && surface.provenance.non_interactive_approval_supported
                && surface.provenance.executable == attempt.executable
                && surface.provenance.version == attempt.version
        });
        if !authority_valid {
            if let Some(mut child) = self
                .full_access_canary_children
                .lock()
                .map_err(|_| "Native full-access canary supervision is unavailable")?
                .remove(&attempt.attempt_id)
            {
                let _ = child.terminate();
            }
            self.settle_full_access_canary(&attempt, "cancelled", "cancelled", None, false)?;
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
                self.full_access_canary_children
                    .lock()
                    .ok()
                    .and_then(|mut children| children.remove(&attempt.attempt_id));
                self.settle_full_access_canary(
                    &attempt,
                    "passed",
                    if receipt.exit_code.is_some() {
                        "exit_code"
                    } else {
                        "not_observed"
                    },
                    receipt.exit_code,
                    true,
                )?;
            }
            Some(receipt) => {
                self.full_access_canary_children
                    .lock()
                    .ok()
                    .and_then(|mut children| children.remove(&attempt.attempt_id));
                let classification = if !receipt.sandbox_receipt_observed {
                    "receipt_missing"
                } else if receipt.exit_code.is_some() {
                    "exit_code"
                } else {
                    "not_observed"
                };
                self.settle_full_access_canary(
                    &attempt,
                    "terminal_failed",
                    classification,
                    receipt.exit_code,
                    receipt.sandbox_receipt_observed,
                )?;
            }
            None if Utc::now() >= attempt.deadline_at => {
                if let Some(mut child) = self
                    .full_access_canary_children
                    .lock()
                    .map_err(|_| "Native full-access canary supervision is unavailable")?
                    .remove(&attempt.attempt_id)
                {
                    let _ = child.terminate();
                }
                self.settle_full_access_canary(&attempt, "timed_out", "timed_out", None, false)?;
            }
            None if !self
                .full_access_canary_children
                .lock()
                .map_err(|_| "Native full-access canary supervision is unavailable")?
                .contains_key(&attempt.attempt_id) =>
            {
                self.settle_full_access_canary(
                    &attempt,
                    "recovered_unobserved",
                    "recovered_unobserved",
                    None,
                    false,
                )?;
            }
            None => {}
        }
        Ok(())
    }

    fn settle_full_access_canary(
        &self,
        attempt: &PendingFullAccessCanary,
        state: &str,
        classification: &str,
        exit_code: Option<i32>,
        receipt_observed: bool,
    ) -> Result<(), String> {
        let cleanup_disposition = match fs::remove_file(&attempt.sentinel_path) {
            Ok(()) => "removed",
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => "removed",
            Err(_) => "failed",
        };
        let state = if state == "passed" && cleanup_disposition != "removed" {
            "cleanup_failed"
        } else {
            state
        };
        let classification = if state == "cleanup_failed" {
            "cleanup_failed"
        } else {
            classification
        };
        let terminal_observed = matches!(state, "passed" | "terminal_failed" | "cleanup_failed");
        self.write("settle native full access canary", |transaction| {
        transaction.execute(
            "UPDATE native_codex_profile_full_access_canaries SET state=?2,settled_at=?3,process_activity=CASE WHEN ?8 THEN 'terminal_observed' ELSE process_activity END,terminal_classification=?4,terminal_exit_code=?5,receipt_observed=?6,cleanup_disposition=?7 WHERE attempt_id=?1 AND state='pending'",
            params![attempt.attempt_id, state, Utc::now().to_rfc3339(), classification, exit_code, receipt_observed as i64, cleanup_disposition, terminal_observed],
        ).map_err(|error| error.to_string())?;
        transaction.execute(
            "UPDATE native_codex_profile_readiness SET danger_full_access_canary=?2,observed_at=?3 WHERE profile_id=?1",
            params![attempt.profile_id, if state == "passed" { "passed" } else { "blocked" }, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        Ok(())
        })?;
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

    /// Caller owns `operation_gate`; avoid recursively acquiring it if continuity must be
    /// recorded while adopting a pending reporting request.
    fn require_selected_active_while_gated(&self, id: &str) -> Result<StoredProfile, String> {
        let profile = self.profile(id)?;
        let lifecycle = validate_profile(&profile);
        if profile.lifecycle != Lifecycle::Active || lifecycle != Lifecycle::Active {
            if lifecycle != Lifecycle::Active {
                self.record_lifecycle_while_gated(id, lifecycle)?;
            }
            return Err(
                "Only a currently validated native profile can perform this operation".into(),
            );
        }
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

    fn full_access_canary_parent(&self, id: &str) -> Result<PathBuf, String> {
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

    fn full_access_canary_work_root(&self, id: &str) -> Result<PathBuf, String> {
        self.ensure_full_access_canary_child(id, "work")
    }

    fn full_access_canary_receipt_root(&self, id: &str) -> Result<PathBuf, String> {
        self.ensure_full_access_canary_child(id, "receipt")
    }

    fn ensure_full_access_canary_child(&self, id: &str, child: &str) -> Result<PathBuf, String> {
        let root = self.full_access_canary_parent(id)?.join(child);
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
        if current.lifecycle != Lifecycle::Active
            || lifecycle != Lifecycle::Active
            || !current.selected
        {
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
        let inserted =
            self.persist_setup_attempt(&attempt, &surface.provenance, "pending", "not_observed")?;
        if inserted == 0 {
            return self.set_attention(
                id,
                phase.attention_concern(),
                Some("native_sandbox_attempt_pending_human_or_uac_attention"),
                false,
            );
        }
        let (cwd, args, sandbox_receipt, sandbox_command_file) =
            match phase {
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
        let attempts = self.read("load pending native setup attempts", |connection| {
            load_pending_setup_attempts(connection, id)
        })?;
        for attempt in attempts {
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
                        if receipt.exit_code.is_some() {
                            "exit_code"
                        } else {
                            "not_observed"
                        },
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
                    let terminal_classification = if attempt.phase
                        == SetupPhase::WorkspaceWriteCanary
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
                Err(state) => self.settle_failed_setup_attempt(&attempt, state, state, None)?,
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
                    "recovered_unobserved" => {
                        "native_sandbox_attempt_recovered_without_owned_process"
                    }
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
        self.write("accept native setup attempt launch", |transaction| transaction.execute(
            "UPDATE native_codex_profile_setup_attempts SET launch_accepted_at=?2 WHERE attempt_id=?1 AND state='pending'",
            params![attempt_id, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string()))?;
        Ok(())
    }

    fn persist_setup_attempt(
        &self,
        attempt: &PendingSetupAttempt,
        provenance: &NativeCliProvenance,
        state: &str,
        terminal_classification: &str,
    ) -> Result<usize, String> {
        self.write("persist native setup attempt", |transaction| transaction.execute(
            "INSERT OR IGNORE INTO native_codex_profile_setup_attempts (attempt_id,profile_id,filesystem_identity,phase,state,executable,version,workspace_sandbox_supported,correlation_id,requested_at,deadline_at,settled_at,terminal_classification) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,CASE WHEN ?5='pending' THEN NULL ELSE ?10 END,?12)",
            params![attempt.attempt_id, attempt.profile_id, attempt.filesystem_identity, attempt.phase.database(), state, provenance.executable, provenance.version, provenance.workspace_sandbox_supported, format!("native-setup-{}", Uuid::new_v4()), Utc::now().to_rfc3339(), attempt.deadline_at.to_rfc3339(), terminal_classification],
        ).map_err(|error| format!("Unable to persist native sandbox attempt: {error}")))
    }

    fn set_setup_attempt_state(
        &self,
        attempt_id: &str,
        state: &str,
        terminal_classification: &str,
        terminal_exit_code: Option<i32>,
    ) -> Result<(), String> {
        self.write("settle native setup attempt", |transaction| transaction.execute(
            "UPDATE native_codex_profile_setup_attempts SET state=?2,settled_at=?3,terminal_classification=?4,terminal_exit_code=?5 WHERE attempt_id=?1 AND state='pending'",
            params![attempt_id, state, Utc::now().to_rfc3339(), terminal_classification, terminal_exit_code],
        ).map_err(|error| error.to_string()))?;
        Ok(())
    }

    fn expire_mcp_probe(&self, id: &str) -> Result<(), String> {
        let expired = self.write("expire native MCP reporting probe", |transaction| transaction
            .execute(
                "UPDATE native_codex_profile_mcp_probes SET state='expired' WHERE profile_id=?1 AND state IN ('pending','dispatching') AND deadline_at <= ?2",
                params![id, Utc::now().to_rfc3339()],
            )
            .map_err(|error| error.to_string()))?;
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
        let Some(attempt) = self.read("load pending native login attempt", |connection| {
            load_pending_login_attempt(connection, id)
        })?
        else {
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
        self.write("settle native login attempt", |transaction| transaction.execute(
            "UPDATE native_codex_profile_login_attempts SET state=?2,settled_at=?3 WHERE attempt_id=?1 AND state='pending'",
            params![attempt_id, state, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string()))?;
        Ok(())
    }

    fn profile(&self, id: &str) -> Result<StoredProfile, String> {
        self.read("load native profile", |connection| {
            Ok(load_profiles(connection)?
                .into_iter()
                .find(|profile| profile.id == id)
                .ok_or_else(|| "Native Codex profile was not found".to_string())?)
        })
    }

    fn profile_by_home(&self, home: &Path) -> Result<Option<StoredProfile>, String> {
        self.read("load native profile by home", |connection| {
            Ok(load_profiles(connection)?
                .into_iter()
                .find(|profile| profile.home == home))
        })
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
        let attempts = self.read("load pending native setup attempts", |connection| {
            load_pending_setup_attempts(connection, id)
        })?;
        let mut children = self
            .setup_children
            .lock()
            .map_err(|_| "Native sandbox child supervision is unavailable")?;
        for attempt in attempts {
            if let Some(mut child) = children.remove(&attempt.attempt_id) {
                let _ = child.terminate();
            }
            // This path may be reconciling a durable pending attempt after a cold reopen, when
            // no child remains in memory. Clean the bounded command file independently of the
            // child outcome before recording its durable cancellation.
            if attempt.phase == SetupPhase::WorkspaceWriteCanary {
                self.remove_workspace_write_canary_command(&attempt.profile_id);
            }
        }
        drop(children);
        if let Some(attempt) = self
            .read("load pending native full access canary", |connection| {
                load_pending_full_access_canary(connection, id)
            })?
        {
            if let Ok(mut children) = self.full_access_canary_children.lock() {
                if let Some(mut child) = children.remove(&attempt.attempt_id) {
                    let _ = child.terminate();
                }
            }
            // Cold reopen or a lost child cannot leave the one bounded receipt behind. Settling
            // owns both exact-file cleanup and its durable outcome; it never claims removal on
            // an I/O failure.
            self.settle_full_access_canary(&attempt, "cancelled", "cancelled", None, false)?;
        }
        self.write("record native profile lifecycle", |transaction| {
        transaction.execute("UPDATE native_codex_profiles SET lifecycle=?2,selected_at=NULL,updated_at=?3 WHERE id=?1", params![id, lifecycle.database(), Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        transaction.execute("UPDATE native_codex_profile_mode_authorizations SET revoked_at=COALESCE(revoked_at,?2) WHERE profile_id=?1 AND mode='danger_full_access'", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        transaction.execute("UPDATE native_codex_profile_readiness SET authentication='unknown',sandbox_initialization='unknown',workspace_write_canary='not_run',danger_full_access_canary='blocked',mcp_reporting='not_assessed',observed_at=?2 WHERE profile_id=?1", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        transaction.execute("UPDATE native_codex_profile_setup_attempts SET state='cancelled',settled_at=?2,terminal_classification='cancelled' WHERE profile_id=?1 AND state='pending'", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        transaction.execute("UPDATE native_codex_profile_mcp_probes SET state='cancelled' WHERE profile_id=?1 AND state IN ('pending','dispatching')", params![id]).map_err(|error| error.to_string())?;
        transaction.execute("UPDATE native_codex_profile_login_attempts SET state='cancelled',settled_at=?2 WHERE profile_id=?1 AND state='pending'", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        transaction.execute("UPDATE native_codex_profile_sandbox_adoptions SET state='invalidated' WHERE profile_id=?1", params![id]).map_err(|error| error.to_string())?;
        transaction.execute("UPDATE native_codex_profile_sandbox_adoption_confirmations SET state='invalidated',invalidated_at=COALESCE(invalidated_at,?2) WHERE profile_id=?1 AND state='confirmed'", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        self.write_attention(
            transaction,
            id,
            "continuity",
            Some("profile_continuity_lost"),)?;
        Ok(())
        }
        )?;
        Ok(())
    }

    fn invalidate_sandbox_adoption(&self, id: &str) -> Result<(), String> {
        self.write("invalidate native sandbox adoption", |transaction| {
        transaction.execute(
            "UPDATE native_codex_profile_sandbox_adoptions SET state='invalidated' WHERE profile_id=?1",
            params![id],
        ).map_err(|error| error.to_string())?;
        transaction.execute(
            "UPDATE native_codex_profile_sandbox_adoption_confirmations SET state='invalidated',invalidated_at=COALESCE(invalidated_at,?2) WHERE profile_id=?1 AND state='confirmed'",
            params![id, Utc::now().to_rfc3339()],
        ).map_err(|error| error.to_string())?;
        Ok(())
        })?;
        self.update_readiness(
            id,
            None,
            Some("attention_required"),
            Some("blocked"),
            None,
            Some((
                "sandbox",
                Some("external_sandbox_adoption_evidence_invalidated"),
            )),
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
        if !profile.selected
            || profile.lifecycle != Lifecycle::Active
            || adoption.disposition != "verified"
        {
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
                Some((
                    "sandbox",
                    Some("external_sandbox_adoption_evidence_invalidated"),
                )),
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
        self.write("update native profile readiness", |transaction| {
        transaction.execute(
            "UPDATE native_codex_profile_readiness SET authentication=COALESCE(?2,authentication),sandbox_initialization=COALESCE(?3,sandbox_initialization),workspace_write_canary=COALESCE(?4,workspace_write_canary),mcp_reporting=COALESCE(?5,mcp_reporting),observed_at=?6 WHERE profile_id=?1",
            params![id, authentication, sandbox, canary, mcp, Utc::now().to_rfc3339()],
        ).map_err(|error| format!("Unable to record native profile readiness: {error}"))?;
        if let Some((concern, detail)) = attention {
            self.write_attention(transaction, id, concern, detail)?;
        }
        Ok(())
        })
    }

    fn set_attention(
        &self,
        id: &str,
        concern: &str,
        attention: Option<&str>,
        reset_readiness: bool,
    ) -> Result<(), String> {
        self.write("set native profile attention", |transaction| {
        if reset_readiness {
            transaction.execute("UPDATE native_codex_profile_readiness SET authentication='unknown',sandbox_initialization='unknown',workspace_write_canary='not_run',mcp_reporting='not_assessed',observed_at=?2 WHERE profile_id=?1", params![id, Utc::now().to_rfc3339()]).map_err(|error| error.to_string())?;
        } else {
            transaction
                .execute(
                    "UPDATE native_codex_profile_readiness SET observed_at=?2 WHERE profile_id=?1",
                    params![id, Utc::now().to_rfc3339()],
                )
                .map_err(|error| error.to_string())?;
        }
        self.write_attention(transaction, id, concern, attention)?;
        Ok(())
        })
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
pub struct ResolvedNativeCodexHome {
    pub profile_id: String,
    pub filesystem_identity: String,
    pub home: PathBuf,
    pub execution_mode: ExecutionMode,
    pub readiness: NativeProfileReadiness,
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

mod repository;
use repository::*;

#[cfg(test)]
mod tests;

impl NativeProfileService {
    pub fn discover_homes(&self) -> Result<Vec<DiscoveredNativeCodexHome>, String> {
        discovery::discover_native_codex_homes(
            &self
                .query()?
                .profiles
                .into_iter()
                .map(|profile| discovery::RegisteredNativeCodexHome {
                    profile_id: profile.id,
                    home_path: PathBuf::from(profile.home_path),
                    selected: profile.selected,
                })
                .collect::<Vec<_>>(),
        )
    }
}
