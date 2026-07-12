use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

mod agent_sessions;
mod runtime;
mod storage;

const APP_DATABASE_FILE_NAME: &str = "codex-orchestrator.sqlite";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppMetadata {
    app_name: &'static str,
    storage_mode: &'static str,
    codex_runtime: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateOpenTaskCommandInput {
    project_id: String,
    repo_id: Option<String>,
    branch_id: Option<String>,
    worktree_id: Option<String>,
    title: String,
    summary: String,
    execution_state: Option<String>,
    attention_state: Option<String>,
    priority: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterTaskWorktreeCommandInput {
    project_name: String,
    repo_name: Option<String>,
    repo_root_path: String,
    branch_name: Option<String>,
    worktree_path: String,
    is_main: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterTaskRepoCommandInput {
    repo_root_path: String,
    project_name: Option<String>,
    repo_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscoverTaskReposCommandInput {
    root_path: String,
    max_depth: Option<usize>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DiscoveredTaskRepo {
    name: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateOpenTaskCommandInput {
    title: Option<String>,
    summary: Option<String>,
    execution_state: Option<String>,
    attention_state: Option<String>,
    priority: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartCodexTaskRunCommandInput {
    task_id: String,
    prompt: String,
    cwd: Option<String>,
    worktree_id: Option<String>,
    conversation_title: Option<String>,
    conversation_summary: Option<String>,
    additional_args: Option<Vec<String>>,
    env: Option<BTreeMap<String, Option<String>>>,
    post_run_capture: Option<StartCodexTaskRunPostRunCaptureInput>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartCodexTaskRunPostRunCaptureInput {
    collect_diff: Option<bool>,
    validation_command: Option<StartCodexTaskRunValidationCommandInput>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartCodexTaskRunValidationCommandInput {
    command: String,
    args: Option<Vec<String>>,
    cwd: Option<String>,
    env: Option<BTreeMap<String, Option<String>>>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskDashboardSnapshot {
    groups: Vec<DashboardGroup>,
    projects: Vec<TaskDashboardProject>,
    repos: Vec<TaskDashboardRepo>,
    worktree_anchors: Vec<TaskDashboardWorktreeAnchor>,
    total_open_tasks: usize,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskDashboardProject {
    id: String,
    name: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskDashboardRepo {
    id: String,
    project_id: String,
    project: String,
    name: String,
    root_path: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskDashboardWorktreeAnchor {
    id: String,
    project_id: String,
    project: String,
    repo_id: String,
    repo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    path: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DashboardGroup {
    id: &'static str,
    title: &'static str,
    tasks: Vec<DashboardTask>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DashboardTask {
    id: String,
    title: String,
    summary: String,
    project: String,
    execution_state: String,
    attention_state: String,
    priority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worktree_path: Option<String>,
    updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartCodexTaskRunCommandResult {
    status: String,
    task_id: String,
    task_run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_event_stream_artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    final_response_artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    post_run_capture: Option<StartCodexTaskRunPostRunCaptureResult>,
    task: StartCodexTaskRunTaskState,
    task_run: StartCodexTaskRunTaskRunState,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartCodexTaskRunPostRunCaptureResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<StartCodexTaskRunDiffCaptureResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation: Option<StartCodexTaskRunValidationCaptureResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped_reason: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "status")]
enum StartCodexTaskRunDiffCaptureResult {
    #[serde(rename = "captured")]
    Captured {
        artifact_id: String,
        event_id: String,
        diff_length: i64,
        is_empty_diff: bool,
        worktree_path: String,
    },
    #[serde(rename = "failed")]
    Failed { error: String },
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartCodexTaskRunValidationCaptureResult {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_created_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartCodexTaskRunTaskState {
    id: String,
    execution_state: String,
    attention_state: String,
    conversation_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worktree_id: Option<String>,
    updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartCodexTaskRunTaskRunState {
    id: String,
    execution_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worktree_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i64>,
    updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TaskRunDetailSnapshot {
    task: TaskRunDetailTaskAnchor,
    runs: Vec<TaskRunDetailRun>,
    unlinked_artifacts: TaskRunDetailArtifactGroups,
    unlinked_validation_runs: Vec<TaskRunDetailValidationRun>,
    event_timeline: Vec<DetailEvent>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskRunDetailTaskAnchor {
    record: DetailTask,
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<DetailProject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo: Option<DetailRepo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<DetailBranch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worktree: Option<DetailWorktree>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TaskRunDetailRun {
    run: DetailTaskRun,
    artifacts: TaskRunDetailArtifactGroups,
    validation_runs: Vec<TaskRunDetailValidationRun>,
    events: Vec<DetailEvent>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskRunDetailValidationRun {
    run: DetailValidationRun,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_artifact: Option<DetailArtifact>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskRunDetailArtifactGroups {
    final_responses: Vec<DetailArtifact>,
    raw_event_streams: Vec<DetailArtifact>,
    diffs: Vec<DetailArtifact>,
    validation_logs: Vec<DetailArtifact>,
    notes: Vec<DetailArtifact>,
    screenshots: Vec<DetailArtifact>,
    handoffs: Vec<DetailArtifact>,
    summaries: Vec<DetailArtifact>,
    other: Vec<DetailArtifact>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DetailProject {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DetailRepo {
    id: String,
    project_id: String,
    name: String,
    root_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remote_url: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DetailBranch {
    id: String,
    repo_id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    head_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    intent: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DetailWorktree {
    id: String,
    repo_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch_id: Option<String>,
    path: String,
    is_main: bool,
    is_dirty: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    lock_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_scanned_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DetailTask {
    id: String,
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worktree_id: Option<String>,
    conversation_ids: Vec<String>,
    title: String,
    summary: String,
    execution_state: String,
    attention_state: String,
    priority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    due_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snoozed_until: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DetailTaskRun {
    id: String,
    task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worktree_id: Option<String>,
    execution_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i64>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DetailArtifact {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversation_id: Option<String>,
    kind: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    created_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DetailValidationRun {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_run_id: Option<String>,
    command: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_artifact_id: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct DetailEvent {
    id: String,
    kind: String,
    occurred_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation_run_id: Option<String>,
    payload: Map<String, Value>,
}

#[derive(Debug)]
struct TaskRow {
    id: String,
    project_id: String,
    repo_id: Option<String>,
    branch_id: Option<String>,
    worktree_id: Option<String>,
    title: String,
    summary: String,
    execution_state: String,
    attention_state: String,
    priority: String,
    updated_at: String,
}

#[derive(Debug)]
struct ProjectRow {
    id: String,
    name: String,
}

#[derive(Debug)]
struct RepoRow {
    id: String,
    project_id: String,
    name: String,
    root_path: String,
}

#[derive(Debug)]
struct BranchRow {
    id: String,
    name: String,
}

#[derive(Debug)]
struct WorktreeRow {
    id: String,
    path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CodexCommandRunInput {
    command: String,
    args: Vec<String>,
    cwd: Option<String>,
    env: Option<BTreeMap<String, Option<String>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CodexCommandRunResult {
    stdout: String,
    stderr: String,
    exit_code: Option<i64>,
    signal: Option<String>,
}

trait CodexCommandRunner {
    fn run(&self, input: CodexCommandRunInput) -> Result<CodexCommandRunResult, String>;
}

struct SystemCodexCommandRunner;

#[derive(Clone, Debug, PartialEq, Eq)]
struct GitDiffRunInput {
    worktree_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GitDiffRunResult {
    diff: String,
}

trait GitDiffRunner {
    fn collect_tracked_diff(&self, input: GitDiffRunInput) -> Result<GitDiffRunResult, String>;
}

struct SystemGitDiffRunner;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ValidationCommandRunInput {
    command: String,
    args: Vec<String>,
    cwd: String,
    env: Option<BTreeMap<String, Option<String>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ValidationCommandRunResult {
    stdout: String,
    stderr: String,
    exit_code: Option<i64>,
    signal: Option<String>,
}

trait ValidationCommandRunner {
    fn run(&self, input: ValidationCommandRunInput) -> Result<ValidationCommandRunResult, String>;
}

struct SystemValidationCommandRunner;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedPostRunWorktreePath {
    path: String,
    worktree_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartedCodexTaskRun {
    task_id: String,
    project_id: String,
    task_run_id: String,
    conversation_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CodexRuntimeStatus {
    Completed,
    Failed,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
struct CodexRuntimeResult {
    exit_code: Option<i64>,
    signal: Option<String>,
    status: CodexRuntimeStatus,
    status_reason: String,
    stdout_jsonl: String,
    stderr: String,
    summary: CodexJsonlSummary,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct CodexJsonlSummary {
    thread_id: Option<String>,
    final_agent_message_text: Option<String>,
    terminal_status: Option<CodexJsonlTerminalStatus>,
    token_usage: Option<Map<String, Value>>,
    item_counts_by_type: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CodexJsonlTerminalStatus {
    Completed { line_number: usize },
    Failed { line_number: usize },
    Error { line_number: usize },
}

const DASHBOARD_GROUPS: [(&str, &str); 5] = [
    ("needs_action_now", "Needs action now"),
    ("review_decide", "Review / decide"),
    ("working", "Working"),
    ("waiting", "Waiting"),
    ("later", "Later"),
];

const EXECUTION_STATES: [&str; 8] = [
    "draft",
    "queued",
    "running",
    "blocked",
    "completed",
    "failed",
    "abandoned",
    "archived",
];

const ATTENTION_STATES: [&str; 7] = [
    "needs_action_now",
    "needs_review",
    "waiting_on_agent",
    "waiting_on_external",
    "consider_later",
    "snoozed",
    "reference_only",
];

const PRIORITIES: [&str; 3] = ["low", "normal", "high"];

#[tauri::command]
fn app_metadata() -> AppMetadata {
    AppMetadata {
        app_name: "Codex Orchestrator",
        storage_mode: "local-first",
        codex_runtime: "tauri-codex-exec",
    }
}

#[tauri::command]
fn load_open_task_dashboard(app: AppHandle) -> Result<TaskDashboardSnapshot, String> {
    ensure_legacy_tasks_available()?;
    with_app_database(&app, |conn| load_dashboard_snapshot(conn))
}

#[tauri::command]
fn register_task_worktree(
    app: AppHandle,
    input: RegisterTaskWorktreeCommandInput,
) -> Result<TaskDashboardSnapshot, String> {
    ensure_legacy_tasks_available()?;
    with_app_database(&app, |conn| {
        register_task_worktree_anchor(conn, input)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
fn register_task_repo(
    app: AppHandle,
    input: RegisterTaskRepoCommandInput,
) -> Result<TaskDashboardSnapshot, String> {
    ensure_legacy_tasks_available()?;
    with_app_database(&app, |conn| {
        register_task_repo_anchor(conn, input)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
fn discover_task_repos(
    input: DiscoverTaskReposCommandInput,
) -> Result<Vec<DiscoveredTaskRepo>, String> {
    ensure_legacy_tasks_available()?;
    discover_git_repos(input)
}

#[tauri::command]
fn create_open_task(
    app: AppHandle,
    input: CreateOpenTaskCommandInput,
) -> Result<TaskDashboardSnapshot, String> {
    ensure_legacy_tasks_available()?;
    with_app_database(&app, |conn| {
        create_task(conn, input)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
fn update_open_task(
    app: AppHandle,
    task_id: String,
    input: UpdateOpenTaskCommandInput,
) -> Result<TaskDashboardSnapshot, String> {
    ensure_legacy_tasks_available()?;
    with_app_database(&app, |conn| {
        update_task(conn, &task_id, input)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
fn archive_open_task(app: AppHandle, task_id: String) -> Result<TaskDashboardSnapshot, String> {
    ensure_legacy_tasks_available()?;
    with_app_database(&app, |conn| {
        archive_task(conn, &task_id)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
fn load_task_run_detail(app: AppHandle, task_id: String) -> Result<TaskRunDetailSnapshot, String> {
    ensure_legacy_tasks_available()?;
    with_app_database(&app, |conn| load_task_run_detail_snapshot(conn, &task_id))
}

#[tauri::command]
fn start_codex_task_run(
    app: AppHandle,
    input: StartCodexTaskRunCommandInput,
) -> Result<StartCodexTaskRunCommandResult, String> {
    ensure_legacy_tasks_available()?;
    with_app_database(&app, |conn| {
        start_codex_task_run_with_runners(
            conn,
            input,
            &SystemCodexCommandRunner,
            &SystemGitDiffRunner,
            &SystemValidationCommandRunner,
        )
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let database_path = app_database_path(app.handle())?;
            let connection = open_initialized_database(database_path)?;
            let repository = Arc::new(
                agent_sessions::repository::SqliteAgentSessionRepository::new(connection)
                    .map_err(|error| error.to_string())?,
            );
            // Durable history must not wait on a provider executable. Capability discovery remains
            // available only to explicitly opted-in verification; normal execution starts with
            // unknown capabilities and degrades at invocation time when Codex is unavailable.
            let runtime = Arc::new(runtime::codex::CodexCliRuntime::system("codex", None));
            let notifier = Arc::new(agent_sessions::transport::TauriAgentSessionNotifier::new(
                app.handle().clone(),
            ));
            let providers = Arc::new(agent_sessions::application::SystemAgentSessionProviders);
            let application = agent_sessions::application::AgentSessionApplication::new(
                repository,
                runtime,
                notifier,
                providers.clone(),
                providers,
                None,
            );
            application
                .reconcile_startup()
                .map_err(|error| error.to_string())?;
            app.manage(agent_sessions::transport::AgentSessionTauriState::new(
                application,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_metadata,
            // Compatibility command names remain registered so older callers receive a deliberate
            // quarantine error. Every legacy handler rejects before database or process work.
            load_open_task_dashboard,
            register_task_worktree,
            register_task_repo,
            discover_task_repos,
            create_open_task,
            update_open_task,
            archive_open_task,
            load_task_run_detail,
            start_codex_task_run,
            agent_sessions::transport::create_agent_session,
            agent_sessions::transport::list_agent_sessions,
            agent_sessions::transport::load_agent_session,
            agent_sessions::transport::send_agent_session_message,
            agent_sessions::transport::cancel_agent_invocation
        ])
        .build(tauri::generate_context!())
        .expect("error while building Codex Orchestrator");

    app.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            if let Some(state) =
                app_handle.try_state::<agent_sessions::transport::AgentSessionTauriState>()
            {
                let _ = state.application().shutdown_runtime();
            }
        }
    });
}

fn ensure_legacy_tasks_available() -> Result<(), String> {
    Err("Legacy Tasks are quarantined in the Agent Session reset baseline".to_string())
}

fn with_app_database<T>(
    app: &AppHandle,
    operation: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let database_path = app_database_path(app)?;
    let conn = open_initialized_database(database_path)?;
    operation(&conn)
}

fn app_database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|error| format!("Unable to create app data directory: {error}"))?;

    Ok(app_data_dir.join(APP_DATABASE_FILE_NAME))
}

fn open_initialized_database(database_path: PathBuf) -> Result<Connection, String> {
    let conn = Connection::open(database_path)
        .map_err(|error| format!("Unable to open app SQLite database: {error}"))?;
    storage::configure_sqlite_connection(&conn)
        .map_err(|error| format!("Unable to configure app SQLite database: {error}"))?;
    initialize_database(&conn)?;
    Ok(conn)
}

fn initialize_database(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE (position)
);
",
    )
    .map_err(sql_error("initialize app database"))?;

    apply_registered_migrations(conn, &app_migrations())
}

fn apply_registered_migrations(conn: &Connection, migrations: &[Migration]) -> Result<(), String> {
    validate_migration_registration(migrations)?;

    let mut migrations = migrations.iter().collect::<Vec<_>>();
    migrations.sort_by_key(|migration| migration.position);

    for migration in migrations {
        let applied_position = conn
            .query_row(
                "SELECT position FROM schema_migrations WHERE id = ?1",
                params![migration.id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(sql_error("read schema migration state"))?;

        if let Some(applied_position) = applied_position {
            if applied_position != migration.position {
                return Err(format!(
                    "SQLite migration {} is recorded at position {}; expected {}",
                    migration.id, applied_position, migration.position
                ));
            }
        }

        let position_owner = conn
            .query_row(
                "SELECT id FROM schema_migrations WHERE position = ?1",
                params![migration.position],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_error("read schema migration position"))?;

        if let Some(position_owner) = position_owner {
            if position_owner != migration.id {
                return Err(format!(
                    "SQLite migration position {} is already recorded for {}; cannot apply {}",
                    migration.position, position_owner, migration.id
                ));
            }
        }

        if applied_position.is_some() {
            continue;
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(sql_error("begin schema migration"))?;
        if let Some(prepare) = migration.prepare {
            prepare(&tx)?;
        }
        tx.execute_batch(migration.sql)
            .map_err(|error| format!("Unable to apply migration {}: {error}", migration.id))?;
        tx.execute(
            "INSERT INTO schema_migrations (id, applied_at, position) VALUES (?1, ?2, ?3)",
            params![migration.id, now_iso(), migration.position],
        )
        .map_err(sql_error("record schema migration"))?;
        tx.commit().map_err(sql_error("commit schema migration"))?;
    }

    Ok(())
}

impl CodexCommandRunner for SystemCodexCommandRunner {
    fn run(&self, input: CodexCommandRunInput) -> Result<CodexCommandRunResult, String> {
        let mut command = Command::new(&input.command);
        command.args(&input.args);

        if let Some(cwd) = &input.cwd {
            command.current_dir(cwd);
        }

        if let Some(env) = &input.env {
            for (key, value) in env {
                match value {
                    Some(value) => {
                        command.env(key, value);
                    }
                    None => {
                        command.env_remove(key);
                    }
                }
            }
        }

        let output = command
            .output()
            .map_err(|error| format!("Unable to launch Codex: {error}"))?;

        Ok(CodexCommandRunResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().map(i64::from),
            signal: process_exit_signal(&output.status),
        })
    }
}

impl GitDiffRunner for SystemGitDiffRunner {
    fn collect_tracked_diff(&self, input: GitDiffRunInput) -> Result<GitDiffRunResult, String> {
        let output = Command::new("git")
            .args(["diff", "--binary", "HEAD", "--"])
            .current_dir(&input.worktree_path)
            .output()
            .map_err(|error| format!("Unable to launch Git diff: {error}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let exit_code = output.status.code().map(i64::from);
        let signal = process_exit_signal(&output.status);

        if exit_code != Some(0) || signal.is_some() {
            return Err(format!(
                "Git diff failed {}: {}",
                process_failure_reason(exit_code, signal.as_deref()),
                stderr
            ));
        }

        Ok(GitDiffRunResult { diff: stdout })
    }
}

impl ValidationCommandRunner for SystemValidationCommandRunner {
    fn run(&self, input: ValidationCommandRunInput) -> Result<ValidationCommandRunResult, String> {
        let mut command = Command::new(&input.command);
        command.args(&input.args).current_dir(&input.cwd);

        if let Some(env) = &input.env {
            for (key, value) in env {
                match value {
                    Some(value) => {
                        command.env(key, value);
                    }
                    None => {
                        command.env_remove(key);
                    }
                }
            }
        }

        let output = command
            .output()
            .map_err(|error| format!("Unable to launch validation command: {error}"))?;

        Ok(ValidationCommandRunResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().map(i64::from),
            signal: process_exit_signal(&output.status),
        })
    }
}

fn process_failure_reason(exit_code: Option<i64>, signal: Option<&str>) -> String {
    if let Some(signal) = signal {
        return format!("on signal {signal}");
    }

    match exit_code {
        Some(exit_code) => format!("with exit code {exit_code}"),
        None => "without an exit code".to_string(),
    }
}

#[cfg(unix)]
fn process_exit_signal(status: &std::process::ExitStatus) -> Option<String> {
    use std::os::unix::process::ExitStatusExt;

    status.signal().map(|signal| signal.to_string())
}

#[cfg(not(unix))]
fn process_exit_signal(_status: &std::process::ExitStatus) -> Option<String> {
    None
}

#[cfg(test)]
fn start_codex_task_run_with_runner(
    conn: &Connection,
    input: StartCodexTaskRunCommandInput,
    runner: &impl CodexCommandRunner,
) -> Result<StartCodexTaskRunCommandResult, String> {
    start_codex_task_run_with_runners(
        conn,
        input,
        runner,
        &SystemGitDiffRunner,
        &SystemValidationCommandRunner,
    )
}

fn start_codex_task_run_with_runners(
    conn: &Connection,
    input: StartCodexTaskRunCommandInput,
    codex_runner: &impl CodexCommandRunner,
    git_diff_runner: &impl GitDiffRunner,
    validation_runner: &impl ValidationCommandRunner,
) -> Result<StartCodexTaskRunCommandResult, String> {
    validate_start_codex_task_run_input(&input)?;
    let started = start_codex_task_run_lifecycle(conn, &input)?;
    let command_input = CodexCommandRunInput {
        command: "codex".to_string(),
        args: build_codex_exec_args(&input),
        cwd: input.cwd.clone(),
        env: input.env.clone(),
    };

    match codex_runner.run(command_input) {
        Ok(process_result) => finish_codex_task_run_from_process_result(
            conn,
            &input,
            &started,
            process_result,
            git_diff_runner,
            validation_runner,
        ),
        Err(error) => {
            let mut result = fail_started_codex_task_run(
                conn,
                &started,
                None,
                None,
                Some(error.clone()),
                error,
            )?;
            attach_skipped_post_run_capture_if_requested(&input, &mut result);
            Ok(result)
        }
    }
}

fn validate_start_codex_task_run_input(
    input: &StartCodexTaskRunCommandInput,
) -> Result<(), String> {
    validate_non_empty("taskId", &input.task_id)?;
    validate_non_empty("prompt", &input.prompt)?;

    if let Some(cwd) = &input.cwd {
        validate_non_empty("cwd", cwd)?;
    }

    if let Some(worktree_id) = &input.worktree_id {
        validate_non_empty("worktreeId", worktree_id)?;
    }

    if let Some(additional_args) = &input.additional_args {
        for (index, arg) in additional_args.iter().enumerate() {
            validate_non_empty(&format!("additionalArgs[{index}]"), arg)?;
        }
    }

    if let Some(env) = &input.env {
        for key in env.keys() {
            validate_non_empty("env key", key)?;
        }
    }

    if let Some(post_run_capture) = &input.post_run_capture {
        if let Some(validation_command) = &post_run_capture.validation_command {
            validate_non_empty(
                "postRunCapture.validationCommand.command",
                &validation_command.command,
            )?;

            if let Some(args) = &validation_command.args {
                for (index, arg) in args.iter().enumerate() {
                    validate_non_empty(
                        &format!("postRunCapture.validationCommand.args[{index}]"),
                        arg,
                    )?;
                }
            }

            if let Some(cwd) = &validation_command.cwd {
                validate_non_empty("postRunCapture.validationCommand.cwd", cwd)?;
            }

            if let Some(env) = &validation_command.env {
                for key in env.keys() {
                    validate_non_empty("postRunCapture.validationCommand.env key", key)?;
                }
            }
        }
    }

    Ok(())
}

fn build_codex_exec_args(input: &StartCodexTaskRunCommandInput) -> Vec<String> {
    let mut args = vec!["exec".to_string(), "--json".to_string()];

    if let Some(additional_args) = &input.additional_args {
        args.extend(additional_args.iter().cloned());
    }

    args.push(input.prompt.clone());
    args
}

fn start_codex_task_run_lifecycle(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
) -> Result<StartedCodexTaskRun, String> {
    let existing_task = select_detail_task(conn, &input.task_id)?
        .ok_or_else(|| task_detail_not_found(&input.task_id))?;
    let timestamp = now_iso();
    let task_run_id = Uuid::new_v4().to_string();
    let conversation_id = Uuid::new_v4().to_string();
    let conversation_title = input.conversation_title.as_deref().unwrap_or("Codex run");

    conn.execute(
        "
INSERT INTO task_runs (
  id, task_id, conversation_id, worktree_id, execution_state, started_at, completed_at,
  exit_code, created_at, updated_at
) VALUES (?1, ?2, NULL, ?3, 'running', ?4, NULL, NULL, ?4, ?4)
",
        params![
            task_run_id,
            existing_task.id,
            input.worktree_id.as_deref(),
            timestamp
        ],
    )
    .map_err(sql_error("create task run"))?;

    conn.execute(
        "
INSERT INTO conversations (
  id, task_id, task_run_id, provider, external_thread_id, title, summary, created_at, updated_at
) VALUES (?1, ?2, ?3, 'codex', NULL, ?4, ?5, ?6, ?6)
",
        params![
            conversation_id,
            existing_task.id,
            task_run_id,
            conversation_title,
            input.conversation_summary.as_deref(),
            timestamp
        ],
    )
    .map_err(sql_error("create Codex conversation"))?;

    conn.execute(
        "UPDATE task_runs SET conversation_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![conversation_id, timestamp, task_run_id],
    )
    .map_err(sql_error("link task run conversation"))?;

    let position = next_task_conversation_position(conn, &existing_task.id)?;
    conn.execute(
        "
INSERT INTO task_conversation_links (task_id, conversation_id, position, created_at)
VALUES (?1, ?2, ?3, ?4)
",
        params![existing_task.id, conversation_id, position, timestamp],
    )
    .map_err(sql_error("link task conversation"))?;

    conn.execute(
        "
UPDATE tasks
SET execution_state = 'running', attention_state = 'waiting_on_agent', updated_at = ?1
WHERE id = ?2
",
        params![timestamp, existing_task.id],
    )
    .map_err(sql_error("mark task run running"))?;

    let mut payload = Map::new();
    insert_string(&mut payload, "taskId", &existing_task.id);
    insert_string(&mut payload, "taskRunId", &task_run_id);
    if let Some(worktree_id) = &input.worktree_id {
        insert_string(&mut payload, "worktreeId", worktree_id);
    }
    insert_string(&mut payload, "startedAt", &timestamp);
    insert_string(&mut payload, "conversationId", &conversation_id);
    create_event(
        conn,
        "run_started",
        &timestamp,
        Some(&existing_task.project_id),
        Some(&existing_task.id),
        Some(&task_run_id),
        Some(&conversation_id),
        None,
        None,
        payload,
    )?;

    Ok(StartedCodexTaskRun {
        task_id: existing_task.id,
        project_id: existing_task.project_id,
        task_run_id,
        conversation_id,
    })
}

fn finish_codex_task_run_from_process_result(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    process_result: CodexCommandRunResult,
    git_diff_runner: &impl GitDiffRunner,
    validation_runner: &impl ValidationCommandRunner,
) -> Result<StartCodexTaskRunCommandResult, String> {
    let stdout_jsonl_length = process_result.stdout.len();
    let exit_code = process_result.exit_code;
    let signal = process_result.signal.clone();
    let raw_event_stream_artifact_id =
        create_raw_event_stream_artifact(conn, started, &process_result.stdout)?;
    let runtime_result = match codex_runtime_result_from_process_result(process_result.clone()) {
        Ok(runtime_result) => runtime_result,
        Err(error) => {
            append_raw_event_stream_created_event(
                conn,
                started,
                &raw_event_stream_artifact_id,
                "error",
                stdout_jsonl_length,
                exit_code,
                signal.as_deref(),
                Some(&error),
            )?;

            let mut result = fail_started_codex_task_run(
                conn,
                started,
                Some(raw_event_stream_artifact_id),
                exit_code,
                Some("Codex JSONL parse failed".to_string()),
                error,
            )?;
            attach_skipped_post_run_capture_if_requested(input, &mut result);
            return Ok(result);
        }
    };

    append_raw_event_stream_created_event(
        conn,
        started,
        &raw_event_stream_artifact_id,
        runtime_result.status.as_str(),
        runtime_result.stdout_jsonl.len(),
        runtime_result.exit_code,
        runtime_result.signal.as_deref(),
        None,
    )?;
    update_conversation_from_runtime_result(conn, started, &runtime_result)?;

    if runtime_result.status == CodexRuntimeStatus::Completed {
        let mut result = complete_started_codex_task_run(
            conn,
            started,
            raw_event_stream_artifact_id,
            runtime_result,
        )?;
        attach_post_run_capture_if_requested(
            conn,
            input,
            started,
            git_diff_runner,
            validation_runner,
            &mut result,
        );
        Ok(result)
    } else {
        let error = codex_failure_reason(&runtime_result);
        let mut result = fail_started_codex_task_run(
            conn,
            started,
            Some(raw_event_stream_artifact_id),
            runtime_result.exit_code,
            Some(runtime_result.status_reason),
            error,
        )?;
        attach_skipped_post_run_capture_if_requested(input, &mut result);
        Ok(result)
    }
}

fn codex_runtime_result_from_process_result(
    process_result: CodexCommandRunResult,
) -> Result<CodexRuntimeResult, String> {
    let summary = parse_codex_jsonl_summary(&process_result.stdout)?;
    let classification = classify_codex_exec_result(&process_result, &summary);

    Ok(CodexRuntimeResult {
        exit_code: process_result.exit_code,
        signal: process_result.signal,
        status: classification.0,
        status_reason: classification.1,
        stdout_jsonl: process_result.stdout,
        stderr: process_result.stderr,
        summary,
    })
}

fn complete_started_codex_task_run(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    raw_event_stream_artifact_id: String,
    runtime_result: CodexRuntimeResult,
) -> Result<StartCodexTaskRunCommandResult, String> {
    let completed_at = now_iso();

    conn.execute(
        "
UPDATE task_runs
SET execution_state = 'completed', completed_at = ?1, exit_code = ?2, updated_at = ?1
WHERE id = ?3
",
        params![completed_at, runtime_result.exit_code, started.task_run_id],
    )
    .map_err(sql_error("complete task run"))?;

    let final_response_artifact_id = match &runtime_result.summary.final_agent_message_text {
        Some(final_response) => Some(create_artifact(
            conn,
            Some(&started.task_id),
            Some(&started.task_run_id),
            Some(&started.conversation_id),
            "final_response",
            "Final Codex response",
            Some(final_response),
        )?),
        None => None,
    };

    conn.execute(
        "
UPDATE tasks
SET execution_state = 'completed', attention_state = 'needs_review', updated_at = ?1
WHERE id = ?2
",
        params![completed_at, started.task_id],
    )
    .map_err(sql_error("mark task run completed"))?;

    let mut payload = Map::new();
    insert_string(&mut payload, "outcome", "completed");
    insert_string(&mut payload, "taskId", &started.task_id);
    insert_string(&mut payload, "taskRunId", &started.task_run_id);
    insert_string(&mut payload, "completedAt", &completed_at);
    if let Some(exit_code) = runtime_result.exit_code {
        insert_i64(&mut payload, "exitCode", exit_code);
    }
    if let Some(artifact_id) = &final_response_artifact_id {
        insert_string(&mut payload, "artifactId", artifact_id);
    }
    create_event(
        conn,
        "run_completed",
        &completed_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        Some(&started.conversation_id),
        final_response_artifact_id.as_deref(),
        None,
        payload,
    )?;

    build_start_codex_task_run_result(
        conn,
        "completed",
        started,
        Some(raw_event_stream_artifact_id),
        final_response_artifact_id,
        runtime_result.exit_code,
        Some(runtime_result.status_reason),
        None,
    )
}

fn fail_started_codex_task_run(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    raw_event_stream_artifact_id: Option<String>,
    exit_code: Option<i64>,
    status_reason: Option<String>,
    error: String,
) -> Result<StartCodexTaskRunCommandResult, String> {
    let completed_at = now_iso();

    conn.execute(
        "
UPDATE task_runs
SET execution_state = 'failed', completed_at = ?1, exit_code = ?2, updated_at = ?1
WHERE id = ?3
",
        params![completed_at, exit_code, started.task_run_id],
    )
    .map_err(sql_error("fail task run"))?;

    conn.execute(
        "
UPDATE tasks
SET execution_state = 'failed', attention_state = 'needs_action_now', updated_at = ?1
WHERE id = ?2
",
        params![completed_at, started.task_id],
    )
    .map_err(sql_error("mark task run failed"))?;

    let mut payload = Map::new();
    insert_string(&mut payload, "outcome", "failed");
    insert_string(&mut payload, "taskId", &started.task_id);
    insert_string(&mut payload, "taskRunId", &started.task_run_id);
    insert_string(&mut payload, "completedAt", &completed_at);
    if let Some(exit_code) = exit_code {
        insert_i64(&mut payload, "exitCode", exit_code);
    }
    insert_string(&mut payload, "error", &error);
    create_event(
        conn,
        "run_completed",
        &completed_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        Some(&started.conversation_id),
        None,
        None,
        payload,
    )?;

    build_start_codex_task_run_result(
        conn,
        "failed",
        started,
        raw_event_stream_artifact_id,
        None,
        exit_code,
        status_reason,
        Some(error),
    )
}

fn attach_skipped_post_run_capture_if_requested(
    input: &StartCodexTaskRunCommandInput,
    result: &mut StartCodexTaskRunCommandResult,
) {
    if input.post_run_capture.is_some() {
        result.post_run_capture = Some(StartCodexTaskRunPostRunCaptureResult {
            skipped_reason: Some("run_failed".to_string()),
            ..StartCodexTaskRunPostRunCaptureResult::default()
        });
    }
}

fn attach_post_run_capture_if_requested(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    git_diff_runner: &impl GitDiffRunner,
    validation_runner: &impl ValidationCommandRunner,
    result: &mut StartCodexTaskRunCommandResult,
) {
    let Some(options) = &input.post_run_capture else {
        return;
    };

    result.post_run_capture = Some(run_post_run_capture(
        conn,
        input,
        started,
        options,
        git_diff_runner,
        validation_runner,
    ));
}

fn run_post_run_capture(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    options: &StartCodexTaskRunPostRunCaptureInput,
    git_diff_runner: &impl GitDiffRunner,
    validation_runner: &impl ValidationCommandRunner,
) -> StartCodexTaskRunPostRunCaptureResult {
    let diff = if options.collect_diff.unwrap_or(false) {
        Some(
            match collect_post_run_diff(conn, input, started, git_diff_runner) {
                Ok(result) => result,
                Err(error) => StartCodexTaskRunDiffCaptureResult::Failed { error },
            },
        )
    } else {
        None
    };

    let validation = options
        .validation_command
        .as_ref()
        .map(|validation_command| {
            match run_post_run_validation_command(
                conn,
                input,
                started,
                validation_command,
                validation_runner,
            ) {
                Ok(result) => result,
                Err(error) => StartCodexTaskRunValidationCaptureResult {
                    status: "failed".to_string(),
                    validation_run_id: None,
                    output_artifact_id: None,
                    started_event_id: None,
                    artifact_created_event_id: None,
                    completed_event_id: None,
                    exit_code: None,
                    signal: None,
                    error: Some(error),
                },
            }
        });

    StartCodexTaskRunPostRunCaptureResult {
        diff,
        validation,
        skipped_reason: None,
    }
}

fn collect_post_run_diff(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    git_diff_runner: &impl GitDiffRunner,
) -> Result<StartCodexTaskRunDiffCaptureResult, String> {
    let resolved = resolve_post_run_worktree_path(
        conn,
        started,
        input.cwd.as_deref(),
        input.worktree_id.as_deref(),
    )?;
    let diff_result = git_diff_runner.collect_tracked_diff(GitDiffRunInput {
        worktree_path: resolved.path.clone(),
    })?;
    let is_empty_diff = diff_result.diff.is_empty();
    let artifact_id = create_artifact(
        conn,
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        "diff",
        "Post-run diff",
        Some(&diff_result.diff),
    )?;
    let occurred_at = now_iso();
    let mut payload = Map::new();
    insert_string(&mut payload, "artifactKind", "diff");
    insert_string(&mut payload, "artifactId", &artifact_id);
    insert_i64(&mut payload, "diffLength", diff_result.diff.len() as i64);
    insert_bool(&mut payload, "isEmptyDiff", is_empty_diff);
    insert_string(&mut payload, "worktreePath", &resolved.path);
    if let Some(worktree_id) = &resolved.worktree_id {
        insert_string(&mut payload, "worktreeId", worktree_id);
    }
    let event_id = create_event(
        conn,
        "artifact_created",
        &occurred_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        Some(&artifact_id),
        None,
        payload,
    )?;

    Ok(StartCodexTaskRunDiffCaptureResult::Captured {
        artifact_id,
        event_id,
        diff_length: diff_result.diff.len() as i64,
        is_empty_diff,
        worktree_path: resolved.path,
    })
}

fn run_post_run_validation_command(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    validation_command: &StartCodexTaskRunValidationCommandInput,
    validation_runner: &impl ValidationCommandRunner,
) -> Result<StartCodexTaskRunValidationCaptureResult, String> {
    let resolved = resolve_post_run_validation_cwd(conn, input, started, validation_command)?;
    let args = validation_command.args.clone().unwrap_or_default();
    let display_command = render_validation_command(&validation_command.command, &args);
    let started_at = now_iso();
    let validation_run_id = Uuid::new_v4().to_string();

    conn.execute(
        "
INSERT INTO validation_runs (
  id, task_id, task_run_id, command, status, started_at, completed_at, exit_code,
  output_artifact_id, created_at, updated_at
) VALUES (?1, ?2, ?3, ?4, 'running', ?5, NULL, NULL, NULL, ?5, ?5)
",
        params![
            validation_run_id,
            started.task_id,
            started.task_run_id,
            display_command,
            started_at
        ],
    )
    .map_err(sql_error("create validation run"))?;

    let started_event_id = append_validation_started_event(
        conn,
        started,
        &validation_run_id,
        validation_command,
        &args,
        &resolved,
        &started_at,
    )?;

    let (runtime_result, runtime_error) = match validation_runner.run(ValidationCommandRunInput {
        command: validation_command.command.clone(),
        args: args.clone(),
        cwd: resolved.path.clone(),
        env: validation_command.env.clone(),
    }) {
        Ok(result) => (Some(result), None),
        Err(error) => (None, Some(error)),
    };

    finish_post_run_validation_command(
        conn,
        started,
        validation_command,
        &args,
        &display_command,
        &validation_run_id,
        &started_event_id,
        &resolved,
        &started_at,
        runtime_result,
        runtime_error,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_post_run_validation_command(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    validation_command: &StartCodexTaskRunValidationCommandInput,
    args: &[String],
    display_command: &str,
    validation_run_id: &str,
    started_event_id: &str,
    resolved: &ResolvedPostRunWorktreePath,
    started_at: &str,
    runtime_result: Option<ValidationCommandRunResult>,
    runtime_error: Option<String>,
) -> Result<StartCodexTaskRunValidationCaptureResult, String> {
    let status = classify_validation_command_status(runtime_result.as_ref());
    let completed_at = now_iso();
    let output_content = create_validation_log_content(
        started,
        validation_run_id,
        status,
        validation_command,
        args,
        resolved,
        started_at,
        &completed_at,
        runtime_result.as_ref(),
        runtime_error.as_deref(),
    )?;
    let output_artifact_id = create_artifact(
        conn,
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        "validation_log",
        &format!("Validation log: {display_command}"),
        Some(&output_content),
    )?;
    let artifact_created_event_id = append_validation_artifact_created_event(
        conn,
        started,
        validation_run_id,
        &output_artifact_id,
        status,
        runtime_result.as_ref(),
        runtime_error.as_deref(),
    )?;

    conn.execute(
        "
UPDATE validation_runs
SET status = ?1, completed_at = ?2, exit_code = ?3, output_artifact_id = ?4, updated_at = ?2
WHERE id = ?5
",
        params![
            status,
            completed_at,
            runtime_result.as_ref().and_then(|result| result.exit_code),
            output_artifact_id,
            validation_run_id
        ],
    )
    .map_err(sql_error("complete validation run"))?;

    let completed_event_id = append_validation_completed_event(
        conn,
        started,
        validation_run_id,
        &output_artifact_id,
        status,
        &completed_at,
        runtime_result.as_ref(),
        runtime_error.as_deref(),
    )?;

    Ok(StartCodexTaskRunValidationCaptureResult {
        status: status.to_string(),
        validation_run_id: Some(validation_run_id.to_string()),
        output_artifact_id: Some(output_artifact_id),
        started_event_id: Some(started_event_id.to_string()),
        artifact_created_event_id: Some(artifact_created_event_id),
        completed_event_id: Some(completed_event_id),
        exit_code: runtime_result.as_ref().and_then(|result| result.exit_code),
        signal: runtime_result
            .as_ref()
            .and_then(|result| result.signal.clone()),
        error: runtime_error,
    })
}

fn resolve_post_run_validation_cwd(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    validation_command: &StartCodexTaskRunValidationCommandInput,
) -> Result<ResolvedPostRunWorktreePath, String> {
    if let Some(cwd) = &validation_command.cwd {
        return Ok(ResolvedPostRunWorktreePath {
            path: cwd.clone(),
            worktree_id: input.worktree_id.clone(),
        });
    }

    resolve_post_run_worktree_path(
        conn,
        started,
        input.cwd.as_deref(),
        input.worktree_id.as_deref(),
    )
}

fn resolve_post_run_worktree_path(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    explicit_cwd: Option<&str>,
    input_worktree_id: Option<&str>,
) -> Result<ResolvedPostRunWorktreePath, String> {
    if let Some(cwd) = explicit_cwd {
        return Ok(ResolvedPostRunWorktreePath {
            path: cwd.to_string(),
            worktree_id: input_worktree_id.map(ToString::to_string),
        });
    }

    let worktree_id = match input_worktree_id {
        Some(worktree_id) => Some(worktree_id.to_string()),
        None => select_task_run_worktree_id(conn, &started.task_run_id)?
            .or(select_task_worktree_id(conn, &started.task_id)?),
    }
    .ok_or_else(|| {
        format!(
            "Post-run capture could not resolve a cwd or linked worktree path for task: {}",
            started.task_id
        )
    })?;

    let path = select_worktree_path(conn, &worktree_id)?.ok_or_else(|| {
        format!(
            "Post-run capture worktree not found for task {}: {}",
            started.task_id, worktree_id
        )
    })?;

    Ok(ResolvedPostRunWorktreePath {
        path,
        worktree_id: Some(worktree_id),
    })
}

fn select_task_run_worktree_id(
    conn: &Connection,
    task_run_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT worktree_id FROM task_runs WHERE id = ?1",
        params![task_run_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_error("read task run worktree id"))
    .map(Option::flatten)
}

fn select_task_worktree_id(conn: &Connection, task_id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT worktree_id FROM tasks WHERE id = ?1",
        params![task_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_error("read task worktree id"))
    .map(Option::flatten)
}

fn select_worktree_path(conn: &Connection, worktree_id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT path FROM worktrees WHERE id = ?1",
        params![worktree_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_error("read worktree path"))
}

fn append_validation_started_event(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    validation_run_id: &str,
    validation_command: &StartCodexTaskRunValidationCommandInput,
    args: &[String],
    resolved: &ResolvedPostRunWorktreePath,
    started_at: &str,
) -> Result<String, String> {
    let mut payload = Map::new();
    insert_string(&mut payload, "taskId", &started.task_id);
    insert_string(&mut payload, "validationRunId", validation_run_id);
    insert_string(&mut payload, "command", &validation_command.command);
    insert_string_array(&mut payload, "args", args);
    insert_string(&mut payload, "cwd", &resolved.path);
    if let Some(worktree_id) = &resolved.worktree_id {
        insert_string(&mut payload, "worktreeId", worktree_id);
    }
    insert_string(&mut payload, "startedAt", started_at);

    create_event(
        conn,
        "validation_started",
        started_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        None,
        Some(validation_run_id),
        payload,
    )
}

#[allow(clippy::too_many_arguments)]
fn create_validation_log_content(
    started: &StartedCodexTaskRun,
    validation_run_id: &str,
    status: &str,
    validation_command: &StartCodexTaskRunValidationCommandInput,
    args: &[String],
    resolved: &ResolvedPostRunWorktreePath,
    started_at: &str,
    completed_at: &str,
    runtime_result: Option<&ValidationCommandRunResult>,
    runtime_error: Option<&str>,
) -> Result<String, String> {
    let mut payload = Map::new();
    insert_string(&mut payload, "taskId", &started.task_id);
    insert_string(&mut payload, "validationRunId", validation_run_id);
    insert_string(&mut payload, "status", status);
    insert_string(&mut payload, "command", &validation_command.command);
    insert_string_array(&mut payload, "args", args);
    insert_string(&mut payload, "cwd", &resolved.path);
    if let Some(worktree_id) = &resolved.worktree_id {
        insert_string(&mut payload, "worktreeId", worktree_id);
    }
    insert_string(&mut payload, "startedAt", started_at);
    insert_string(&mut payload, "completedAt", completed_at);
    payload.insert(
        "process".to_string(),
        Value::Object(validation_process_payload(runtime_result, runtime_error)),
    );

    serde_json::to_string_pretty(&Value::Object(payload))
        .map_err(|error| format!("Unable to encode validation log artifact: {error}"))
}

fn validation_process_payload(
    runtime_result: Option<&ValidationCommandRunResult>,
    runtime_error: Option<&str>,
) -> Map<String, Value> {
    let mut process = Map::new();

    match runtime_result {
        Some(result) => {
            insert_string(&mut process, "stdout", &result.stdout);
            insert_string(&mut process, "stderr", &result.stderr);
            insert_nullable_i64(&mut process, "exitCode", result.exit_code);
            insert_nullable_string(&mut process, "signal", result.signal.as_deref());
        }
        None => {
            insert_string(&mut process, "stdout", "");
            insert_string(&mut process, "stderr", "");
            insert_nullable_i64(&mut process, "exitCode", None);
            insert_nullable_string(&mut process, "signal", None);
            insert_string(
                &mut process,
                "error",
                runtime_error.unwrap_or("Validation command did not return a process result"),
            );
        }
    }

    process
}

fn append_validation_artifact_created_event(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    validation_run_id: &str,
    output_artifact_id: &str,
    status: &str,
    runtime_result: Option<&ValidationCommandRunResult>,
    runtime_error: Option<&str>,
) -> Result<String, String> {
    let occurred_at = now_iso();
    let mut payload = Map::new();
    insert_string(&mut payload, "artifactKind", "validation_log");
    insert_string(&mut payload, "artifactId", output_artifact_id);
    insert_string(&mut payload, "validationRunId", validation_run_id);
    insert_string(&mut payload, "validationStatus", status);
    if let Some(result) = runtime_result {
        insert_i64(&mut payload, "stdoutLength", result.stdout.len() as i64);
        insert_i64(&mut payload, "stderrLength", result.stderr.len() as i64);
        if let Some(exit_code) = result.exit_code {
            insert_i64(&mut payload, "exitCode", exit_code);
        }
        if let Some(signal) = &result.signal {
            insert_string(&mut payload, "signal", signal);
        }
    }
    if let Some(error) = runtime_error {
        insert_string(&mut payload, "error", error);
    }

    create_event(
        conn,
        "artifact_created",
        &occurred_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        Some(output_artifact_id),
        Some(validation_run_id),
        payload,
    )
}

fn append_validation_completed_event(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    validation_run_id: &str,
    output_artifact_id: &str,
    status: &str,
    completed_at: &str,
    runtime_result: Option<&ValidationCommandRunResult>,
    runtime_error: Option<&str>,
) -> Result<String, String> {
    let mut payload = Map::new();
    insert_string(&mut payload, "outcome", status);
    insert_string(&mut payload, "taskId", &started.task_id);
    insert_string(&mut payload, "validationRunId", validation_run_id);
    insert_string(&mut payload, "artifactId", output_artifact_id);
    insert_string(&mut payload, "completedAt", completed_at);
    if let Some(result) = runtime_result {
        if let Some(exit_code) = result.exit_code {
            insert_i64(&mut payload, "exitCode", exit_code);
        }
        if let Some(signal) = &result.signal {
            insert_string(&mut payload, "signal", signal);
        }
    }
    if let Some(error) = runtime_error {
        insert_string(&mut payload, "error", error);
    }

    create_event(
        conn,
        "validation_completed",
        completed_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        Some(output_artifact_id),
        Some(validation_run_id),
        payload,
    )
}

fn classify_validation_command_status(result: Option<&ValidationCommandRunResult>) -> &'static str {
    match result {
        Some(result) if result.exit_code == Some(0) && result.signal.is_none() => "passed",
        _ => "failed",
    }
}

fn render_validation_command(command: &str, args: &[String]) -> String {
    std::iter::once(command.to_string())
        .chain(args.iter().map(|arg| render_validation_command_arg(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_validation_command_arg(arg: &str) -> String {
    if arg.chars().all(is_plain_validation_command_arg_char) {
        return arg.to_string();
    }

    serde_json::to_string(arg).unwrap_or_else(|_| arg.to_string())
}

fn is_plain_validation_command_arg_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || "_./:=@+-".contains(character)
}

fn create_raw_event_stream_artifact(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    stdout_jsonl: &str,
) -> Result<String, String> {
    create_artifact(
        conn,
        Some(&started.task_id),
        Some(&started.task_run_id),
        Some(&started.conversation_id),
        "raw_event_stream",
        "Raw Codex JSONL",
        Some(stdout_jsonl),
    )
}

#[allow(clippy::too_many_arguments)]
fn append_raw_event_stream_created_event(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    artifact_id: &str,
    codex_status: &str,
    stdout_jsonl_length: usize,
    exit_code: Option<i64>,
    signal: Option<&str>,
    parse_error: Option<&str>,
) -> Result<String, String> {
    let occurred_at = now_iso();
    let mut payload = Map::new();
    insert_string(&mut payload, "artifactKind", "raw_event_stream");
    insert_string(&mut payload, "artifactId", artifact_id);
    insert_string(&mut payload, "codexStatus", codex_status);
    insert_i64(
        &mut payload,
        "stdoutJsonlLength",
        stdout_jsonl_length as i64,
    );
    if let Some(exit_code) = exit_code {
        insert_i64(&mut payload, "exitCode", exit_code);
    }
    if let Some(signal) = signal {
        insert_string(&mut payload, "signal", signal);
    }
    if let Some(parse_error) = parse_error {
        insert_string(&mut payload, "parseError", parse_error);
    }

    create_event(
        conn,
        "artifact_created",
        &occurred_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        Some(&started.conversation_id),
        Some(artifact_id),
        None,
        payload,
    )
}

fn update_conversation_from_runtime_result(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    runtime_result: &CodexRuntimeResult,
) -> Result<(), String> {
    conn.execute(
        "
UPDATE conversations
SET external_thread_id = COALESCE(?1, external_thread_id), summary = ?2, updated_at = ?3
WHERE id = ?4
",
        params![
            runtime_result.summary.thread_id.as_deref(),
            summarize_conversation(runtime_result),
            now_iso(),
            started.conversation_id
        ],
    )
    .map_err(sql_error("update Codex conversation"))?;

    Ok(())
}

fn summarize_conversation(runtime_result: &CodexRuntimeResult) -> String {
    let prefix = if runtime_result.status == CodexRuntimeStatus::Completed {
        "Codex completed".to_string()
    } else {
        format!(
            "Codex {}: {}",
            runtime_result.status.as_str(),
            runtime_result.status_reason
        )
    };

    match runtime_result.summary.final_agent_message_text.as_deref() {
        Some(final_message) if !final_message.trim().is_empty() => {
            truncate(&format!("{prefix}: {}", final_message.trim()), 240)
        }
        _ => prefix,
    }
}

fn codex_failure_reason(runtime_result: &CodexRuntimeResult) -> String {
    let stderr = runtime_result.stderr.trim();

    if stderr.is_empty() {
        return runtime_result.status_reason.clone();
    }

    truncate(&format!("{}: {stderr}", runtime_result.status_reason), 500)
}

fn classify_codex_exec_result(
    process_result: &CodexCommandRunResult,
    summary: &CodexJsonlSummary,
) -> (CodexRuntimeStatus, String) {
    match summary.terminal_status {
        Some(CodexJsonlTerminalStatus::Error { .. }) => {
            return (
                CodexRuntimeStatus::Error,
                "Codex emitted an error event".to_string(),
            );
        }
        Some(CodexJsonlTerminalStatus::Failed { .. }) => {
            return (
                CodexRuntimeStatus::Failed,
                "Codex emitted a turn.failed event".to_string(),
            );
        }
        _ => {}
    }

    if let Some(signal) = &process_result.signal {
        return (
            CodexRuntimeStatus::Failed,
            format!("Codex process exited on signal {signal}"),
        );
    }

    if process_result.exit_code != Some(0) {
        return (
            CodexRuntimeStatus::Failed,
            match process_result.exit_code {
                Some(exit_code) => format!("Codex process exited with code {exit_code}"),
                None => "Codex process exited without an exit code".to_string(),
            },
        );
    }

    if matches!(
        summary.terminal_status,
        Some(CodexJsonlTerminalStatus::Completed { .. })
    ) {
        return (
            CodexRuntimeStatus::Completed,
            "Codex emitted a turn.completed event".to_string(),
        );
    }

    (
        CodexRuntimeStatus::Failed,
        "Codex output did not include a terminal event".to_string(),
    )
}

fn parse_codex_jsonl_summary(jsonl: &str) -> Result<CodexJsonlSummary, String> {
    let mut summary = CodexJsonlSummary::default();
    let normalized_jsonl = jsonl.replace("\r\n", "\n").replace('\r', "\n");

    for (index, line) in normalized_jsonl.split('\n').enumerate() {
        let line_number = index + 1;

        if line.trim().is_empty() {
            continue;
        }

        let parsed = serde_json::from_str::<Value>(line)
            .map_err(|error| format!("Line {line_number}: Invalid JSON: {error}"))?;
        let object = parsed
            .as_object()
            .ok_or_else(|| format!("Line {line_number}: Event line must be a JSON object"))?;
        let event_type = object
            .get("type")
            .ok_or_else(|| format!("Line {line_number}: Event type is required"))?
            .as_str()
            .ok_or_else(|| format!("Line {line_number}: Event type must be a string"))?;

        match event_type {
            "thread.started" => {
                let thread_id = object
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .filter(|thread_id| !thread_id.is_empty())
                    .ok_or_else(|| {
                        format!("Line {line_number}: thread.started thread_id must be a string")
                    })?;
                summary.thread_id = Some(thread_id.to_string());
            }
            "turn.completed" => {
                summary.terminal_status = Some(CodexJsonlTerminalStatus::Completed { line_number });
                if let Some(usage) = object.get("usage") {
                    summary.token_usage = Some(usage.as_object().cloned().ok_or_else(|| {
                        format!("Line {line_number}: turn.completed usage must be a JSON object")
                    })?);
                }
            }
            "turn.failed" => {
                summary.terminal_status = Some(CodexJsonlTerminalStatus::Failed { line_number });
            }
            "error" => {
                summary.terminal_status = Some(CodexJsonlTerminalStatus::Error { line_number });
            }
            _ if event_type.starts_with("item.") => {
                let item = object
                    .get("item")
                    .and_then(Value::as_object)
                    .ok_or_else(|| {
                        format!("Line {line_number}: {event_type} item must be a JSON object")
                    })?;
                let item_type = item
                    .get("type")
                    .ok_or_else(|| format!("Line {line_number}: Item type is required"))?
                    .as_str()
                    .ok_or_else(|| format!("Line {line_number}: Item type must be a string"))?;
                *summary
                    .item_counts_by_type
                    .entry(item_type.to_string())
                    .or_insert(0) += 1;

                if event_type == "item.completed" && item_type == "agent_message" {
                    if let Some(text) = item.get("text") {
                        summary.final_agent_message_text = Some(
                            text.as_str()
                                .ok_or_else(|| {
                                    format!(
                                        "Line {line_number}: agent_message text must be a string"
                                    )
                                })?
                                .to_string(),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    Ok(summary)
}

fn build_start_codex_task_run_result(
    conn: &Connection,
    status: &str,
    started: &StartedCodexTaskRun,
    raw_event_stream_artifact_id: Option<String>,
    final_response_artifact_id: Option<String>,
    exit_code: Option<i64>,
    status_reason: Option<String>,
    error: Option<String>,
) -> Result<StartCodexTaskRunCommandResult, String> {
    let task = select_detail_task(conn, &started.task_id)?
        .ok_or_else(|| task_detail_not_found(&started.task_id))?;
    let task_run = select_detail_task_runs(conn, &started.task_id)?
        .into_iter()
        .find(|task_run| task_run.id == started.task_run_id)
        .ok_or_else(|| format!("Task run not found: {}", started.task_run_id))?;

    Ok(StartCodexTaskRunCommandResult {
        status: status.to_string(),
        task_id: started.task_id.clone(),
        task_run_id: started.task_run_id.clone(),
        conversation_id: Some(started.conversation_id.clone()),
        raw_event_stream_artifact_id,
        final_response_artifact_id,
        exit_code,
        status_reason,
        error,
        post_run_capture: None,
        task: start_codex_task_run_task_state(task),
        task_run: start_codex_task_run_task_run_state(task_run),
    })
}

fn start_codex_task_run_task_state(task: DetailTask) -> StartCodexTaskRunTaskState {
    StartCodexTaskRunTaskState {
        id: task.id,
        execution_state: task.execution_state,
        attention_state: task.attention_state,
        conversation_ids: task.conversation_ids,
        repo_id: task.repo_id,
        branch_id: task.branch_id,
        worktree_id: task.worktree_id,
        updated_at: task.updated_at,
    }
}

fn start_codex_task_run_task_run_state(task_run: DetailTaskRun) -> StartCodexTaskRunTaskRunState {
    StartCodexTaskRunTaskRunState {
        id: task_run.id,
        execution_state: task_run.execution_state,
        conversation_id: task_run.conversation_id,
        worktree_id: task_run.worktree_id,
        started_at: task_run.started_at,
        completed_at: task_run.completed_at,
        exit_code: task_run.exit_code,
        updated_at: task_run.updated_at,
    }
}

impl CodexRuntimeStatus {
    fn as_str(self) -> &'static str {
        match self {
            CodexRuntimeStatus::Completed => "completed",
            CodexRuntimeStatus::Failed => "failed",
            CodexRuntimeStatus::Error => "error",
        }
    }
}

fn register_task_worktree_anchor(
    conn: &Connection,
    input: RegisterTaskWorktreeCommandInput,
) -> Result<(), String> {
    let project_name = input.project_name.trim().to_string();
    let worktree_path = input.worktree_path.trim().to_string();
    let repo_root_path = match input.repo_root_path.trim() {
        "" => worktree_path.clone(),
        value => value.to_string(),
    };
    validate_non_empty("projectName", &project_name)?;
    validate_non_empty("repoRootPath", &repo_root_path)?;
    validate_non_empty("worktreePath", &worktree_path)?;

    let repo_name = input
        .repo_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| path_label(&repo_root_path).unwrap_or_else(|| project_name.clone()));
    let branch_name = input
        .branch_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let timestamp = now_iso();

    let project_id = upsert_project(conn, &project_name, &timestamp)?;
    let repo_id = upsert_repo(
        conn,
        &project_id,
        &repo_name,
        &repo_root_path,
        branch_name.as_deref(),
        &timestamp,
    )?;
    let branch_id = match branch_name {
        Some(branch_name) => Some(upsert_branch(conn, &repo_id, &branch_name, &timestamp)?),
        None => None,
    };
    upsert_worktree(
        conn,
        &repo_id,
        branch_id.as_deref(),
        &worktree_path,
        input.is_main.unwrap_or(false),
        &timestamp,
    )?;

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct GitWorktreeFacts {
    path: String,
    branch_name: Option<String>,
    is_main: bool,
}

fn register_task_repo_anchor(
    conn: &Connection,
    input: RegisterTaskRepoCommandInput,
) -> Result<(), String> {
    let repo_root_path = input.repo_root_path.trim().to_string();
    validate_non_empty("repoRootPath", &repo_root_path)?;

    let git_root_path = git_stdout(&repo_root_path, &["rev-parse", "--show-toplevel"])?
        .trim()
        .to_string();
    validate_non_empty("gitRootPath", &git_root_path)?;

    let repo_label = path_label(&git_root_path).unwrap_or_else(|| "Repository".to_string());
    let project_name = input
        .project_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| repo_label.clone());
    let repo_name = input
        .repo_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| repo_label.clone());
    let default_branch = git_default_branch(&git_root_path)?;
    let worktrees = git_worktree_facts(&git_root_path)?;
    let branch_names = git_branch_names(&git_root_path, &worktrees)?;
    let timestamp = now_iso();

    let project_id = upsert_project(conn, &project_name, &timestamp)?;
    let repo_id = upsert_repo(
        conn,
        &project_id,
        &repo_name,
        &git_root_path,
        default_branch.as_deref(),
        &timestamp,
    )?;
    let branch_ids = upsert_branches(conn, &repo_id, &branch_names, &timestamp)?;

    for worktree in worktrees {
        let branch_id = worktree
            .branch_name
            .as_deref()
            .and_then(|branch_name| branch_ids.get(branch_name))
            .map(String::as_str);
        upsert_worktree(
            conn,
            &repo_id,
            branch_id,
            &worktree.path,
            worktree.is_main,
            &timestamp,
        )?;
    }

    Ok(())
}

fn upsert_branches(
    conn: &Connection,
    repo_id: &str,
    branch_names: &[String],
    timestamp: &str,
) -> Result<BTreeMap<String, String>, String> {
    let mut branch_ids = BTreeMap::new();

    for branch_name in branch_names {
        branch_ids.insert(
            branch_name.clone(),
            upsert_branch(conn, repo_id, branch_name, timestamp)?,
        );
    }

    Ok(branch_ids)
}

fn upsert_project(conn: &Connection, name: &str, timestamp: &str) -> Result<String, String> {
    if let Some(project_id) = conn
        .query_row(
            "SELECT id FROM projects WHERE name = ?1 ORDER BY created_at, id LIMIT 1",
            params![name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error("read project by name"))?
    {
        conn.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            params![timestamp, project_id],
        )
        .map_err(sql_error("update project"))?;
        return Ok(project_id);
    }

    let project_id = Uuid::new_v4().to_string();
    conn.execute(
        "
INSERT INTO projects (id, name, description, created_at, updated_at)
VALUES (?1, ?2, NULL, ?3, ?3)
",
        params![project_id, name, timestamp],
    )
    .map_err(sql_error("create project"))?;
    Ok(project_id)
}

fn upsert_repo(
    conn: &Connection,
    project_id: &str,
    name: &str,
    root_path: &str,
    default_branch: Option<&str>,
    timestamp: &str,
) -> Result<String, String> {
    if let Some(repo_id) = conn
        .query_row(
            "SELECT id FROM repos WHERE project_id = ?1 AND root_path = ?2",
            params![project_id, root_path],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error("read repo by path"))?
    {
        conn.execute(
            "
UPDATE repos SET name = ?1, default_branch = ?2, updated_at = ?3 WHERE id = ?4
",
            params![name, default_branch, timestamp, repo_id],
        )
        .map_err(sql_error("update repo"))?;
        return Ok(repo_id);
    }

    let repo_id = Uuid::new_v4().to_string();
    conn.execute(
        "
INSERT INTO repos (id, project_id, name, root_path, default_branch, remote_url, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?6)
",
        params![repo_id, project_id, name, root_path, default_branch, timestamp],
    )
    .map_err(sql_error("create repo"))?;
    Ok(repo_id)
}

fn upsert_branch(
    conn: &Connection,
    repo_id: &str,
    name: &str,
    timestamp: &str,
) -> Result<String, String> {
    if let Some(branch_id) = conn
        .query_row(
            "SELECT id FROM branches WHERE repo_id = ?1 AND name = ?2",
            params![repo_id, name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error("read branch by name"))?
    {
        conn.execute(
            "UPDATE branches SET updated_at = ?1 WHERE id = ?2",
            params![timestamp, branch_id],
        )
        .map_err(sql_error("update branch"))?;
        return Ok(branch_id);
    }

    let branch_id = Uuid::new_v4().to_string();
    conn.execute(
        "
INSERT INTO branches (id, repo_id, name, base_branch, head_sha, intent, created_at, updated_at)
VALUES (?1, ?2, ?3, NULL, NULL, NULL, ?4, ?4)
",
        params![branch_id, repo_id, name, timestamp],
    )
    .map_err(sql_error("create branch"))?;
    Ok(branch_id)
}

fn upsert_worktree(
    conn: &Connection,
    repo_id: &str,
    branch_id: Option<&str>,
    path: &str,
    is_main: bool,
    timestamp: &str,
) -> Result<String, String> {
    if let Some(worktree_id) = conn
        .query_row(
            "SELECT id FROM worktrees WHERE repo_id = ?1 AND path = ?2",
            params![repo_id, path],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error("read worktree by path"))?
    {
        conn.execute(
            "
UPDATE worktrees SET branch_id = ?1, is_main = ?2, last_scanned_at = ?3, updated_at = ?3
WHERE id = ?4
",
            params![branch_id, bool_to_sqlite(is_main), timestamp, worktree_id],
        )
        .map_err(sql_error("update worktree"))?;
        return Ok(worktree_id);
    }

    let worktree_id = Uuid::new_v4().to_string();
    conn.execute(
        "
INSERT INTO worktrees (
  id, repo_id, branch_id, path, is_main, is_dirty, lock_reason, last_scanned_at, created_at, updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, 0, NULL, ?6, ?6, ?6)
",
        params![
            worktree_id,
            repo_id,
            branch_id,
            path,
            bool_to_sqlite(is_main),
            timestamp
        ],
    )
    .map_err(sql_error("create worktree"))?;
    Ok(worktree_id)
}

fn resolve_create_task_anchor(
    conn: &Connection,
    project_id: &str,
    repo_id: Option<String>,
    branch_id: Option<String>,
    worktree_id: Option<String>,
) -> Result<(Option<String>, Option<String>, Option<String>), String> {
    if let Some(worktree_id) = worktree_id {
        validate_non_empty("worktreeId", &worktree_id)?;
        let (anchor_project_id, anchor_repo_id, anchor_branch_id) =
            select_worktree_task_anchor(conn, &worktree_id)?
                .ok_or_else(|| format!("Worktree not found: {worktree_id}"))?;
        ensure_same_anchor("projectId", project_id, &anchor_project_id)?;

        if let Some(repo_id) = repo_id.as_deref() {
            ensure_same_anchor("repoId", repo_id, &anchor_repo_id)?;
        }

        if let Some(branch_id) = branch_id.as_deref() {
            match anchor_branch_id.as_deref() {
                Some(anchor_branch_id) => {
                    ensure_same_anchor("branchId", branch_id, anchor_branch_id)?
                }
                None => return Err(format!("Branch does not belong to worktree: {branch_id}")),
            }
        }

        return Ok((Some(anchor_repo_id), anchor_branch_id, Some(worktree_id)));
    }

    if let Some(branch_id) = branch_id {
        validate_non_empty("branchId", &branch_id)?;
        let (anchor_project_id, anchor_repo_id) = select_branch_task_anchor(conn, &branch_id)?
            .ok_or_else(|| format!("Branch not found: {branch_id}"))?;
        ensure_same_anchor("projectId", project_id, &anchor_project_id)?;

        if let Some(repo_id) = repo_id.as_deref() {
            ensure_same_anchor("repoId", repo_id, &anchor_repo_id)?;
        }

        return Ok((Some(anchor_repo_id), Some(branch_id), None));
    }

    if let Some(repo_id) = repo_id {
        validate_non_empty("repoId", &repo_id)?;
        let anchor_project_id = select_repo_project_id(conn, &repo_id)?
            .ok_or_else(|| format!("Repo not found: {repo_id}"))?;
        ensure_same_anchor("projectId", project_id, &anchor_project_id)?;
        return Ok((Some(repo_id), None, None));
    }

    Ok((None, None, None))
}

fn ensure_same_anchor(label: &str, expected: &str, actual: &str) -> Result<(), String> {
    if expected == actual {
        return Ok(());
    }

    Err(format!(
        "Mismatched {label}: {expected} does not match {actual}"
    ))
}

fn path_label(path: &str) -> Option<String> {
    PathBuf::from(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn bool_to_sqlite(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn git_stdout(cwd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|error| format!("Failed to launch git: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("Git command failed: git -C {cwd} {}", args.join(" "))
        } else {
            stderr
        });
    }

    String::from_utf8(output.stdout).map_err(|error| format!("Git output was not UTF-8: {error}"))
}

fn git_default_branch(repo_root_path: &str) -> Result<Option<String>, String> {
    if let Ok(default_branch) = git_stdout(
        repo_root_path,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    ) {
        let normalized = default_branch
            .trim()
            .strip_prefix("origin/")
            .unwrap_or(default_branch.trim())
            .to_string();

        if !normalized.is_empty() {
            return Ok(Some(normalized));
        }
    }

    let current_branch = git_stdout(repo_root_path, &["branch", "--show-current"])?;
    let current_branch = current_branch.trim();

    if current_branch.is_empty() {
        Ok(None)
    } else {
        Ok(Some(current_branch.to_string()))
    }
}

fn git_branch_names(
    repo_root_path: &str,
    worktrees: &[GitWorktreeFacts],
) -> Result<Vec<String>, String> {
    let mut names = HashSet::new();
    let branch_output = git_stdout(repo_root_path, &["branch", "--format=%(refname:short)"])?;

    for branch_name in branch_output.lines().map(str::trim) {
        if !branch_name.is_empty() {
            names.insert(branch_name.to_string());
        }
    }

    for worktree in worktrees {
        if let Some(branch_name) = worktree.branch_name.as_deref() {
            names.insert(branch_name.to_string());
        }
    }

    let mut names = names.into_iter().collect::<Vec<_>>();
    names.sort();
    Ok(names)
}

fn git_worktree_facts(repo_root_path: &str) -> Result<Vec<GitWorktreeFacts>, String> {
    let output = git_stdout(repo_root_path, &["worktree", "list", "--porcelain"])?;
    let worktrees = parse_git_worktree_list(&output, repo_root_path);

    if worktrees.is_empty() {
        Ok(vec![GitWorktreeFacts {
            path: repo_root_path.to_string(),
            branch_name: git_default_branch(repo_root_path)?,
            is_main: true,
        }])
    } else {
        Ok(worktrees)
    }
}

fn parse_git_worktree_list(output: &str, repo_root_path: &str) -> Vec<GitWorktreeFacts> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            push_git_worktree_fact(
                &mut worktrees,
                current_path.take(),
                current_branch.take(),
                is_bare,
                repo_root_path,
            );
            current_path = Some(path.to_string());
            current_branch = None;
            is_bare = false;
            continue;
        }

        if let Some(branch_ref) = line.strip_prefix("branch ") {
            current_branch = normalize_git_branch_ref(branch_ref);
        } else if line == "bare" {
            is_bare = true;
        }
    }

    push_git_worktree_fact(
        &mut worktrees,
        current_path,
        current_branch,
        is_bare,
        repo_root_path,
    );

    worktrees
}

fn push_git_worktree_fact(
    worktrees: &mut Vec<GitWorktreeFacts>,
    path: Option<String>,
    branch_name: Option<String>,
    is_bare: bool,
    repo_root_path: &str,
) {
    if is_bare {
        return;
    }

    if let Some(path) = path.filter(|value| !value.trim().is_empty()) {
        worktrees.push(GitWorktreeFacts {
            is_main: same_filesystem_path(&path, repo_root_path),
            path,
            branch_name,
        });
    }
}

fn normalize_git_branch_ref(branch_ref: &str) -> Option<String> {
    let branch_name = branch_ref
        .trim()
        .strip_prefix("refs/heads/")
        .unwrap_or(branch_ref.trim());

    if branch_name.is_empty() {
        None
    } else {
        Some(branch_name.to_string())
    }
}

fn same_filesystem_path(left: &str, right: &str) -> bool {
    normalize_path_for_compare(left) == normalize_path_for_compare(right)
}

fn normalize_path_for_compare(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

fn discover_git_repos(
    input: DiscoverTaskReposCommandInput,
) -> Result<Vec<DiscoveredTaskRepo>, String> {
    let root_path = input.root_path.trim();
    validate_non_empty("rootPath", root_path)?;

    let root = PathBuf::from(root_path);

    if !root.is_dir() {
        return Err(format!("Search root is not a directory: {root_path}"));
    }

    let max_depth = input.max_depth.unwrap_or(4).min(8);
    let mut repos = Vec::new();
    let mut seen_paths = HashSet::new();
    collect_git_repos(&root, 0, max_depth, &mut repos, &mut seen_paths);
    repos.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(repos)
}

fn collect_git_repos(
    directory: &Path,
    depth: usize,
    max_depth: usize,
    repos: &mut Vec<DiscoveredTaskRepo>,
    seen_paths: &mut HashSet<String>,
) {
    if is_git_repo_path(directory) {
        add_discovered_repo(directory, repos, seen_paths);
        return;
    }

    if depth >= max_depth {
        return;
    }

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() && should_scan_child_dir(&path) {
            collect_git_repos(&path, depth + 1, max_depth, repos, seen_paths);
        }
    }
}

fn is_git_repo_path(directory: &Path) -> bool {
    let git_marker = directory.join(".git");
    git_marker.is_dir() || git_marker.is_file()
}

fn should_scan_child_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    !matches!(
        name,
        ".git" | ".dev" | ".vite" | "coverage" | "dist" | "dist-ssr" | "node_modules" | "target"
    )
}

fn add_discovered_repo(
    directory: &Path,
    repos: &mut Vec<DiscoveredTaskRepo>,
    seen_paths: &mut HashSet<String>,
) {
    let display_path = directory.to_string_lossy().to_string();
    let key = normalize_path_for_compare(&display_path);

    if !seen_paths.insert(key) {
        return;
    }

    repos.push(DiscoveredTaskRepo {
        name: path_label(&display_path).unwrap_or_else(|| display_path.clone()),
        path: display_path,
    });
}

fn create_task(conn: &Connection, input: CreateOpenTaskCommandInput) -> Result<(), String> {
    validate_non_empty("projectId", &input.project_id)?;
    validate_non_empty("title", &input.title)?;
    validate_non_empty("summary", &input.summary)?;

    let execution_state = validate_value(
        "executionState",
        input.execution_state.as_deref().unwrap_or("draft"),
        &EXECUTION_STATES,
    )?;
    let attention_state = validate_value(
        "attentionState",
        input
            .attention_state
            .as_deref()
            .unwrap_or("needs_action_now"),
        &ATTENTION_STATES,
    )?;
    let priority = validate_value(
        "priority",
        input.priority.as_deref().unwrap_or("normal"),
        &PRIORITIES,
    )?;
    let timestamp = now_iso();
    let task_id = Uuid::new_v4().to_string();
    let (repo_id, branch_id, worktree_id) = resolve_create_task_anchor(
        conn,
        &input.project_id,
        input.repo_id,
        input.branch_id,
        input.worktree_id,
    )?;

    conn.execute(
        "
INSERT INTO tasks (
  id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, due_at, snoozed_until, created_at, updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, NULL, ?11, ?11)
",
        params![
            task_id,
            input.project_id,
            repo_id,
            branch_id,
            worktree_id,
            input.title,
            input.summary,
            execution_state,
            attention_state,
            priority,
            timestamp
        ],
    )
    .map_err(sql_error("create open task"))?;

    Ok(())
}

fn update_task(
    conn: &Connection,
    task_id: &str,
    input: UpdateOpenTaskCommandInput,
) -> Result<(), String> {
    validate_non_empty("taskId", task_id)?;
    let existing = select_task(conn, task_id)?.ok_or_else(|| open_task_not_found(task_id))?;

    let title = validate_optional_non_empty("title", input.title)?.unwrap_or(existing.title);
    let summary =
        validate_optional_non_empty("summary", input.summary)?.unwrap_or(existing.summary);
    let execution_state = match input.execution_state {
        Some(value) => validate_value("executionState", &value, &EXECUTION_STATES)?.to_string(),
        None => existing.execution_state,
    };
    let attention_state = match input.attention_state {
        Some(value) => validate_value("attentionState", &value, &ATTENTION_STATES)?.to_string(),
        None => existing.attention_state,
    };
    let priority = match input.priority {
        Some(value) => validate_value("priority", &value, &PRIORITIES)?.to_string(),
        None => existing.priority,
    };

    conn.execute(
        "
UPDATE tasks SET
  title = ?1,
  summary = ?2,
  execution_state = ?3,
  attention_state = ?4,
  priority = ?5,
  updated_at = ?6
WHERE id = ?7
",
        params![
            title,
            summary,
            execution_state,
            attention_state,
            priority,
            now_iso(),
            task_id
        ],
    )
    .map_err(sql_error("update open task"))?;

    Ok(())
}

fn archive_task(conn: &Connection, task_id: &str) -> Result<(), String> {
    validate_non_empty("taskId", task_id)?;
    let changed = conn
        .execute(
            "UPDATE tasks SET execution_state = 'archived', updated_at = ?1 WHERE id = ?2",
            params![now_iso(), task_id],
        )
        .map_err(sql_error("archive open task"))?;

    if changed == 0 {
        return Err(open_task_not_found(task_id));
    }

    Ok(())
}

fn load_dashboard_snapshot(conn: &Connection) -> Result<TaskDashboardSnapshot, String> {
    let tasks = select_tasks(conn)?;
    let projects = select_projects(conn)?;
    let repos = select_repos(conn)?;
    let branches = select_branches(conn)?;
    let worktrees = select_worktrees(conn)?;
    let worktree_anchors = select_dashboard_worktree_anchors(conn)?;

    let mut groups = DASHBOARD_GROUPS
        .iter()
        .map(|(id, title)| DashboardGroup {
            id: *id,
            title: *title,
            tasks: Vec::new(),
        })
        .collect::<Vec<_>>();

    for task in tasks {
        if is_closed_task(&task) {
            continue;
        }

        let group_id = dashboard_group_id(&task);
        let group = groups
            .iter_mut()
            .find(|group| group.id == group_id)
            .ok_or_else(|| format!("Unknown dashboard group: {group_id}"))?;
        let project = projects
            .iter()
            .find(|project| project.id == task.project_id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Unassigned project".to_string());
        let repo = task
            .repo_id
            .as_ref()
            .and_then(|id| repos.iter().find(|repo| repo.id == *id))
            .map(|repo| repo.name.clone());
        let branch = task
            .branch_id
            .as_ref()
            .and_then(|id| branches.iter().find(|branch| branch.id == *id))
            .map(|branch| branch.name.clone());
        let worktree_path = task
            .worktree_id
            .as_ref()
            .and_then(|id| worktrees.iter().find(|worktree| worktree.id == *id))
            .map(|worktree| worktree.path.clone());

        group.tasks.push(DashboardTask {
            id: task.id,
            title: task.title,
            summary: task.summary,
            project,
            execution_state: task.execution_state,
            attention_state: task.attention_state,
            priority: task.priority,
            repo,
            branch,
            worktree_path,
            updated_at: task.updated_at,
        });
    }

    for group in &mut groups {
        group
            .tasks
            .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    }

    let mut dashboard_projects = projects
        .iter()
        .map(|project| TaskDashboardProject {
            id: project.id.clone(),
            name: project.name.clone(),
        })
        .collect::<Vec<_>>();
    dashboard_projects.sort_by(|left, right| left.name.cmp(&right.name));
    let mut dashboard_repos = repos
        .into_iter()
        .map(|repo| {
            let project = projects
                .iter()
                .find(|project| project.id == repo.project_id)
                .map(|project| project.name.clone())
                .unwrap_or_else(|| "Unassigned project".to_string());

            TaskDashboardRepo {
                id: repo.id,
                project_id: repo.project_id,
                project,
                name: repo.name,
                root_path: repo.root_path,
            }
        })
        .collect::<Vec<_>>();
    dashboard_repos.sort_by(|left, right| {
        left.project
            .cmp(&right.project)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    let total_open_tasks = groups.iter().map(|group| group.tasks.len()).sum();

    Ok(TaskDashboardSnapshot {
        groups,
        projects: dashboard_projects,
        repos: dashboard_repos,
        worktree_anchors,
        total_open_tasks,
    })
}

fn load_task_run_detail_snapshot(
    conn: &Connection,
    task_id: &str,
) -> Result<TaskRunDetailSnapshot, String> {
    validate_non_empty("taskId", task_id)?;
    let task = select_detail_task(conn, task_id)?.ok_or_else(|| task_detail_not_found(task_id))?;
    let task_runs = select_detail_task_runs(conn, task_id)?;
    let artifacts = select_detail_artifacts(conn, task_id)?;
    let events = select_detail_events(conn, task_id)?;
    let validation_runs = select_detail_validation_runs(conn, task_id)?;

    let run_ids = task_runs
        .iter()
        .map(|run| run.id.clone())
        .collect::<HashSet<_>>();
    let validation_output_artifact_ids = validation_runs
        .iter()
        .filter_map(|run| run.output_artifact_id.clone())
        .collect::<HashSet<_>>();

    let mut runs_for_review = task_runs.clone();
    runs_for_review.sort_by(compare_runs_for_review);

    let runs = runs_for_review
        .into_iter()
        .map(|run| {
            let mut run_artifacts = artifacts
                .iter()
                .filter(|artifact| artifact.task_run_id.as_deref() == Some(run.id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            let run_validation_runs = validation_runs
                .iter()
                .filter(|validation_run| {
                    validation_run_belongs_to_run(validation_run, &run.id, &artifacts)
                })
                .cloned()
                .collect::<Vec<_>>();
            let run_validation_artifact_ids = run_validation_runs
                .iter()
                .filter_map(|validation_run| validation_run.output_artifact_id.clone())
                .collect::<HashSet<_>>();
            let mut run_artifact_ids = run_artifacts
                .iter()
                .map(|artifact| artifact.id.clone())
                .collect::<HashSet<_>>();

            let additional_validation_artifacts = artifacts
                .iter()
                .filter(|artifact| {
                    artifact.task_run_id.is_none()
                        && run_validation_artifact_ids.contains(&artifact.id)
                        && !run_artifact_ids.contains(&artifact.id)
                })
                .cloned()
                .collect::<Vec<_>>();

            for artifact in additional_validation_artifacts {
                run_artifact_ids.insert(artifact.id.clone());
                run_artifacts.push(artifact);
            }

            let mut run_events = events
                .iter()
                .filter(|event| event.task_run_id.as_deref() == Some(run.id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            run_events.sort_by(compare_events_chronologically);

            TaskRunDetailRun {
                run,
                artifacts: group_artifacts(run_artifacts),
                validation_runs: run_validation_runs
                    .iter()
                    .map(|validation_run| detail_validation_run(validation_run, &artifacts))
                    .collect(),
                events: run_events,
            }
        })
        .collect::<Vec<_>>();

    let unlinked_artifacts = group_artifacts(
        artifacts
            .iter()
            .filter(|artifact| {
                artifact.task_run_id.is_none()
                    && !validation_output_artifact_ids.contains(&artifact.id)
            })
            .cloned()
            .collect(),
    );
    let mut unlinked_validation_runs = validation_runs
        .iter()
        .filter(|validation_run| {
            !validation_run_belongs_to_any_run(validation_run, &run_ids, &artifacts)
        })
        .cloned()
        .collect::<Vec<_>>();
    unlinked_validation_runs.sort_by(compare_validation_runs_for_review);

    let mut event_timeline = events;
    event_timeline.sort_by(compare_events_chronologically);

    Ok(TaskRunDetailSnapshot {
        task: TaskRunDetailTaskAnchor {
            project: select_detail_project(conn, &task.project_id)?,
            repo: match &task.repo_id {
                Some(repo_id) => select_detail_repo(conn, repo_id)?,
                None => None,
            },
            branch: match &task.branch_id {
                Some(branch_id) => select_detail_branch(conn, branch_id)?,
                None => None,
            },
            worktree: match &task.worktree_id {
                Some(worktree_id) => select_detail_worktree(conn, worktree_id)?,
                None => None,
            },
            record: task,
        },
        runs,
        unlinked_artifacts,
        unlinked_validation_runs: unlinked_validation_runs
            .iter()
            .map(|validation_run| detail_validation_run(validation_run, &artifacts))
            .collect(),
        event_timeline,
    })
}

fn validation_run_belongs_to_run(
    validation_run: &DetailValidationRun,
    task_run_id: &str,
    artifacts: &[DetailArtifact],
) -> bool {
    if validation_run.task_run_id.as_deref() == Some(task_run_id) {
        return true;
    }

    let Some(output_artifact_id) = &validation_run.output_artifact_id else {
        return false;
    };

    artifacts.iter().any(|artifact| {
        artifact.id == *output_artifact_id && artifact.task_run_id.as_deref() == Some(task_run_id)
    })
}

fn validation_run_belongs_to_any_run(
    validation_run: &DetailValidationRun,
    run_ids: &HashSet<String>,
    artifacts: &[DetailArtifact],
) -> bool {
    if validation_run
        .task_run_id
        .as_ref()
        .is_some_and(|task_run_id| run_ids.contains(task_run_id))
    {
        return true;
    }

    let Some(output_artifact_id) = &validation_run.output_artifact_id else {
        return false;
    };

    artifacts.iter().any(|artifact| {
        artifact.id == *output_artifact_id
            && artifact
                .task_run_id
                .as_ref()
                .is_some_and(|task_run_id| run_ids.contains(task_run_id))
    })
}

fn detail_validation_run(
    validation_run: &DetailValidationRun,
    artifacts: &[DetailArtifact],
) -> TaskRunDetailValidationRun {
    TaskRunDetailValidationRun {
        run: validation_run.clone(),
        output_artifact: validation_run
            .output_artifact_id
            .as_ref()
            .and_then(|artifact_id| {
                artifacts
                    .iter()
                    .find(|artifact| artifact.id == *artifact_id)
                    .cloned()
            }),
    }
}

fn group_artifacts(mut artifacts: Vec<DetailArtifact>) -> TaskRunDetailArtifactGroups {
    artifacts.sort_by(compare_artifacts_chronologically);
    let mut groups = TaskRunDetailArtifactGroups::default();

    for artifact in artifacts {
        match artifact.kind.as_str() {
            "final_response" => groups.final_responses.push(artifact),
            "raw_event_stream" => groups.raw_event_streams.push(artifact),
            "diff" => groups.diffs.push(artifact),
            "validation_log" => groups.validation_logs.push(artifact),
            "note" => groups.notes.push(artifact),
            "screenshot" => groups.screenshots.push(artifact),
            "handoff" => groups.handoffs.push(artifact),
            "summary" => groups.summaries.push(artifact),
            _ => groups.other.push(artifact),
        }
    }

    groups
}

fn compare_runs_for_review(left: &DetailTaskRun, right: &DetailTaskRun) -> std::cmp::Ordering {
    review_time(right)
        .cmp(review_time(left))
        .then_with(|| right.id.cmp(&left.id))
}

fn review_time(run: &DetailTaskRun) -> &str {
    run.completed_at
        .as_deref()
        .or(run.started_at.as_deref())
        .unwrap_or(&run.created_at)
}

fn compare_artifacts_chronologically(
    left: &DetailArtifact,
    right: &DetailArtifact,
) -> std::cmp::Ordering {
    left.created_at
        .cmp(&right.created_at)
        .then_with(|| left.id.cmp(&right.id))
}

fn compare_events_chronologically(left: &DetailEvent, right: &DetailEvent) -> std::cmp::Ordering {
    left.occurred_at
        .cmp(&right.occurred_at)
        .then_with(|| left.id.cmp(&right.id))
}

fn compare_validation_runs_for_review(
    left: &DetailValidationRun,
    right: &DetailValidationRun,
) -> std::cmp::Ordering {
    validation_review_time(right)
        .cmp(validation_review_time(left))
        .then_with(|| right.id.cmp(&left.id))
}

fn validation_review_time(run: &DetailValidationRun) -> &str {
    run.completed_at
        .as_deref()
        .or(run.started_at.as_deref())
        .unwrap_or(&run.created_at)
}

fn select_task(conn: &Connection, task_id: &str) -> Result<Option<TaskRow>, String> {
    conn.query_row(
        "
SELECT id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, updated_at
FROM tasks
WHERE id = ?1
",
        params![task_id],
        map_task_row,
    )
    .optional()
    .map_err(sql_error("load open task"))
}

fn select_tasks(conn: &Connection) -> Result<Vec<TaskRow>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, updated_at
FROM tasks
ORDER BY updated_at DESC, id
",
        )
        .map_err(sql_error("prepare task dashboard query"))?;

    let rows = stmt
        .query_map([], map_task_row)
        .map_err(sql_error("query task dashboard rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task dashboard rows"))?;

    Ok(rows)
}

fn select_projects(conn: &Connection) -> Result<Vec<ProjectRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name FROM projects ORDER BY id")
        .map_err(sql_error("prepare projects query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectRow {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(sql_error("query project rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read project rows"))?;

    Ok(rows)
}

fn select_repos(conn: &Connection) -> Result<Vec<RepoRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, project_id, name, root_path FROM repos ORDER BY id")
        .map_err(sql_error("prepare repos query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(RepoRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                root_path: row.get(3)?,
            })
        })
        .map_err(sql_error("query repo rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read repo rows"))?;

    Ok(rows)
}

fn select_branches(conn: &Connection) -> Result<Vec<BranchRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name FROM branches ORDER BY id")
        .map_err(sql_error("prepare branches query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(BranchRow {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(sql_error("query branch rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read branch rows"))?;

    Ok(rows)
}

fn select_worktrees(conn: &Connection) -> Result<Vec<WorktreeRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, path FROM worktrees ORDER BY id")
        .map_err(sql_error("prepare worktrees query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(WorktreeRow {
                id: row.get(0)?,
                path: row.get(1)?,
            })
        })
        .map_err(sql_error("query worktree rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read worktree rows"))?;

    Ok(rows)
}

fn select_dashboard_worktree_anchors(
    conn: &Connection,
) -> Result<Vec<TaskDashboardWorktreeAnchor>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT
  worktrees.id,
  projects.id,
  projects.name,
  repos.id,
  repos.name,
  branches.id,
  branches.name,
  worktrees.path
FROM worktrees
JOIN repos ON repos.id = worktrees.repo_id
JOIN projects ON projects.id = repos.project_id
LEFT JOIN branches ON branches.id = worktrees.branch_id
ORDER BY projects.name, repos.name, worktrees.path, worktrees.id
",
        )
        .map_err(sql_error("prepare worktree anchor query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(TaskDashboardWorktreeAnchor {
                id: row.get(0)?,
                project_id: row.get(1)?,
                project: row.get(2)?,
                repo_id: row.get(3)?,
                repo: row.get(4)?,
                branch_id: row.get(5)?,
                branch: row.get(6)?,
                path: row.get(7)?,
            })
        })
        .map_err(sql_error("query worktree anchor rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read worktree anchor rows"))?;

    Ok(rows)
}

fn select_worktree_task_anchor(
    conn: &Connection,
    worktree_id: &str,
) -> Result<Option<(String, String, Option<String>)>, String> {
    conn.query_row(
        "
SELECT repos.project_id, worktrees.repo_id, worktrees.branch_id
FROM worktrees
JOIN repos ON repos.id = worktrees.repo_id
WHERE worktrees.id = ?1
",
        params![worktree_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
    .map_err(sql_error("read worktree task anchor"))
}

fn select_branch_task_anchor(
    conn: &Connection,
    branch_id: &str,
) -> Result<Option<(String, String)>, String> {
    conn.query_row(
        "
SELECT repos.project_id, branches.repo_id
FROM branches
JOIN repos ON repos.id = branches.repo_id
WHERE branches.id = ?1
",
        params![branch_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(sql_error("read branch task anchor"))
}

fn select_repo_project_id(conn: &Connection, repo_id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT project_id FROM repos WHERE id = ?1",
        params![repo_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_error("read repo project id"))
}

fn select_detail_task(conn: &Connection, task_id: &str) -> Result<Option<DetailTask>, String> {
    let task = conn
        .query_row(
            "
SELECT id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, due_at, snoozed_until, created_at, updated_at
FROM tasks
WHERE id = ?1
",
            params![task_id],
            |row| {
                Ok(DetailTask {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    repo_id: row.get(2)?,
                    branch_id: row.get(3)?,
                    worktree_id: row.get(4)?,
                    conversation_ids: Vec::new(),
                    title: row.get(5)?,
                    summary: row.get(6)?,
                    execution_state: row.get(7)?,
                    attention_state: row.get(8)?,
                    priority: row.get(9)?,
                    due_at: row.get(10)?,
                    snoozed_until: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            },
        )
        .optional()
        .map_err(sql_error("load task detail task"))?;

    match task {
        Some(mut task) => {
            task.conversation_ids = select_task_conversation_ids(conn, task_id)?;
            Ok(Some(task))
        }
        None => Ok(None),
    }
}

fn select_task_conversation_ids(conn: &Connection, task_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT conversation_id
FROM task_conversation_links
WHERE task_id = ?1
ORDER BY position, conversation_id
",
        )
        .map_err(sql_error("prepare task conversation links query"))?;

    let rows = stmt
        .query_map(params![task_id], |row| row.get(0))
        .map_err(sql_error("query task conversation link rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task conversation link rows"))?;

    Ok(rows)
}

fn select_detail_project(
    conn: &Connection,
    project_id: &str,
) -> Result<Option<DetailProject>, String> {
    conn.query_row(
        "
SELECT id, name, description, created_at, updated_at
FROM projects
WHERE id = ?1
",
        params![project_id],
        |row| {
            Ok(DetailProject {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(sql_error("load task detail project"))
}

fn select_detail_repo(conn: &Connection, repo_id: &str) -> Result<Option<DetailRepo>, String> {
    conn.query_row(
        "
SELECT id, project_id, name, root_path, default_branch, remote_url, created_at, updated_at
FROM repos
WHERE id = ?1
",
        params![repo_id],
        |row| {
            Ok(DetailRepo {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                root_path: row.get(3)?,
                default_branch: row.get(4)?,
                remote_url: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(sql_error("load task detail repo"))
}

fn select_detail_branch(
    conn: &Connection,
    branch_id: &str,
) -> Result<Option<DetailBranch>, String> {
    conn.query_row(
        "
SELECT id, repo_id, name, base_branch, head_sha, intent, created_at, updated_at
FROM branches
WHERE id = ?1
",
        params![branch_id],
        |row| {
            Ok(DetailBranch {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                name: row.get(2)?,
                base_branch: row.get(3)?,
                head_sha: row.get(4)?,
                intent: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(sql_error("load task detail branch"))
}

fn select_detail_worktree(
    conn: &Connection,
    worktree_id: &str,
) -> Result<Option<DetailWorktree>, String> {
    conn.query_row(
        "
SELECT id, repo_id, branch_id, path, is_main, is_dirty, lock_reason, last_scanned_at,
  created_at, updated_at
FROM worktrees
WHERE id = ?1
",
        params![worktree_id],
        |row| {
            let is_main: i64 = row.get(4)?;
            let is_dirty: i64 = row.get(5)?;

            Ok(DetailWorktree {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                branch_id: row.get(2)?,
                path: row.get(3)?,
                is_main: is_main == 1,
                is_dirty: is_dirty == 1,
                lock_reason: row.get(6)?,
                last_scanned_at: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(sql_error("load task detail worktree"))
}

fn select_detail_task_runs(conn: &Connection, task_id: &str) -> Result<Vec<DetailTaskRun>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT id, task_id, conversation_id, worktree_id, execution_state, started_at, completed_at,
  exit_code, created_at, updated_at
FROM task_runs
WHERE task_id = ?1
ORDER BY created_at, id
",
        )
        .map_err(sql_error("prepare task detail runs query"))?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            Ok(DetailTaskRun {
                id: row.get(0)?,
                task_id: row.get(1)?,
                conversation_id: row.get(2)?,
                worktree_id: row.get(3)?,
                execution_state: row.get(4)?,
                started_at: row.get(5)?,
                completed_at: row.get(6)?,
                exit_code: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(sql_error("query task detail run rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task detail run rows"))?;

    Ok(rows)
}

fn select_detail_artifacts(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<DetailArtifact>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT id, task_id, task_run_id, conversation_id, kind, title, uri, content, created_at
FROM artifacts
WHERE task_id = ?1
ORDER BY created_at, id
",
        )
        .map_err(sql_error("prepare task detail artifacts query"))?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            Ok(DetailArtifact {
                id: row.get(0)?,
                task_id: row.get(1)?,
                task_run_id: row.get(2)?,
                conversation_id: row.get(3)?,
                kind: row.get(4)?,
                title: row.get(5)?,
                uri: row.get(6)?,
                content: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(sql_error("query task detail artifact rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task detail artifact rows"))?;

    Ok(rows)
}

fn select_detail_validation_runs(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<DetailValidationRun>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT id, task_id, task_run_id, command, status, started_at, completed_at, exit_code,
  output_artifact_id, created_at, updated_at
FROM validation_runs
WHERE task_id = ?1
ORDER BY created_at, id
",
        )
        .map_err(sql_error("prepare task detail validation runs query"))?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            Ok(DetailValidationRun {
                id: row.get(0)?,
                task_id: row.get(1)?,
                task_run_id: row.get(2)?,
                command: row.get(3)?,
                status: row.get(4)?,
                started_at: row.get(5)?,
                completed_at: row.get(6)?,
                exit_code: row.get(7)?,
                output_artifact_id: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(sql_error("query task detail validation run rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task detail validation run rows"))?;

    Ok(rows)
}

fn select_detail_events(conn: &Connection, task_id: &str) -> Result<Vec<DetailEvent>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT id, kind, occurred_at, project_id, task_id, task_run_id, conversation_id, artifact_id,
  validation_run_id, payload_json
FROM events
WHERE task_id = ?1
ORDER BY occurred_at, id
",
        )
        .map_err(sql_error("prepare task detail events query"))?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            let id: String = row.get(0)?;
            let payload_json: String = row.get(9)?;
            let payload = parse_event_payload(&id, &payload_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    9,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                )
            })?;

            Ok(DetailEvent {
                id,
                kind: row.get(1)?,
                occurred_at: row.get(2)?,
                project_id: row.get(3)?,
                task_id: row.get(4)?,
                task_run_id: row.get(5)?,
                conversation_id: row.get(6)?,
                artifact_id: row.get(7)?,
                validation_run_id: row.get(8)?,
                payload,
            })
        })
        .map_err(sql_error("query task detail event rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task detail event rows"))?;

    Ok(rows)
}

fn map_task_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRow> {
    Ok(TaskRow {
        id: row.get(0)?,
        project_id: row.get(1)?,
        repo_id: row.get(2)?,
        branch_id: row.get(3)?,
        worktree_id: row.get(4)?,
        title: row.get(5)?,
        summary: row.get(6)?,
        execution_state: row.get(7)?,
        attention_state: row.get(8)?,
        priority: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn dashboard_group_id(task: &TaskRow) -> &'static str {
    if task.attention_state == "needs_action_now" {
        return "needs_action_now";
    }

    if task.attention_state == "needs_review" {
        return "review_decide";
    }

    if task.execution_state == "running" || task.execution_state == "queued" {
        return "working";
    }

    if task.attention_state == "waiting_on_agent" || task.attention_state == "waiting_on_external" {
        return "waiting";
    }

    "later"
}

fn is_closed_task(task: &TaskRow) -> bool {
    task.execution_state == "archived" || task.execution_state == "abandoned"
}

fn next_task_conversation_position(conn: &Connection, task_id: &str) -> Result<i64, String> {
    conn.query_row(
        "
SELECT COALESCE(MAX(position) + 1, 0)
FROM task_conversation_links
WHERE task_id = ?1
",
        params![task_id],
        |row| row.get(0),
    )
    .map_err(sql_error("read next task conversation position"))
}

fn create_artifact(
    conn: &Connection,
    task_id: Option<&str>,
    task_run_id: Option<&str>,
    conversation_id: Option<&str>,
    kind: &str,
    title: &str,
    content: Option<&str>,
) -> Result<String, String> {
    let artifact_id = Uuid::new_v4().to_string();
    let created_at = now_iso();

    conn.execute(
        "
INSERT INTO artifacts (
  id, task_id, task_run_id, conversation_id, kind, title, uri, content, created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8)
",
        params![
            artifact_id,
            task_id,
            task_run_id,
            conversation_id,
            kind,
            title,
            content,
            created_at
        ],
    )
    .map_err(sql_error("create artifact"))?;

    Ok(artifact_id)
}

#[allow(clippy::too_many_arguments)]
fn create_event(
    conn: &Connection,
    kind: &str,
    occurred_at: &str,
    project_id: Option<&str>,
    task_id: Option<&str>,
    task_run_id: Option<&str>,
    conversation_id: Option<&str>,
    artifact_id: Option<&str>,
    validation_run_id: Option<&str>,
    payload: Map<String, Value>,
) -> Result<String, String> {
    let event_id = Uuid::new_v4().to_string();
    let payload_json = Value::Object(payload).to_string();

    conn.execute(
        "
INSERT INTO events (
  id, kind, occurred_at, project_id, task_id, task_run_id, conversation_id, artifact_id,
  validation_run_id, payload_json
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
",
        params![
            event_id,
            kind,
            occurred_at,
            project_id,
            task_id,
            task_run_id,
            conversation_id,
            artifact_id,
            validation_run_id,
            payload_json
        ],
    )
    .map_err(sql_error("create event"))?;

    Ok(event_id)
}

fn insert_string(payload: &mut Map<String, Value>, key: &str, value: &str) {
    payload.insert(key.to_string(), Value::String(value.to_string()));
}

fn insert_i64(payload: &mut Map<String, Value>, key: &str, value: i64) {
    payload.insert(key.to_string(), Value::Number(value.into()));
}

fn insert_nullable_i64(payload: &mut Map<String, Value>, key: &str, value: Option<i64>) {
    match value {
        Some(value) => insert_i64(payload, key, value),
        None => {
            payload.insert(key.to_string(), Value::Null);
        }
    }
}

fn insert_nullable_string(payload: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    match value {
        Some(value) => insert_string(payload, key, value),
        None => {
            payload.insert(key.to_string(), Value::Null);
        }
    }
}

fn insert_bool(payload: &mut Map<String, Value>, key: &str, value: bool) {
    payload.insert(key.to_string(), Value::Bool(value));
}

fn insert_string_array(payload: &mut Map<String, Value>, key: &str, values: &[String]) {
    payload.insert(
        key.to_string(),
        Value::Array(
            values
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        ),
    );
}

fn validate_non_empty(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} is required."));
    }

    Ok(())
}

fn validate_optional_non_empty(
    label: &str,
    value: Option<String>,
) -> Result<Option<String>, String> {
    match value {
        Some(value) => {
            validate_non_empty(label, &value)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

fn validate_value<'a>(
    label: &str,
    value: &'a str,
    allowed_values: &[&str],
) -> Result<&'a str, String> {
    if allowed_values.contains(&value) {
        return Ok(value);
    }

    Err(format!("Invalid {label}: {value}"))
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn parse_event_payload(event_id: &str, payload_json: &str) -> Result<Map<String, Value>, String> {
    match serde_json::from_str::<Value>(payload_json)
        .map_err(|error| format!("Invalid JSON payload for event {event_id}: {error}"))?
    {
        Value::Object(payload) => Ok(payload),
        _ => Err(format!(
            "Invalid JSON payload for event {event_id}: expected a JSON object"
        )),
    }
}

fn open_task_not_found(task_id: &str) -> String {
    format!("Open task not found: {task_id}")
}

fn task_detail_not_found(task_id: &str) -> String {
    format!("Task not found: {task_id}")
}

fn truncate(value: &str, max_length: usize) -> String {
    if value.len() <= max_length {
        return value.to_string();
    }

    let truncated = value
        .chars()
        .take(max_length.saturating_sub(3))
        .collect::<String>();
    format!("{truncated}...")
}

fn sql_error(context: &str) -> impl FnOnce(rusqlite::Error) -> String + '_ {
    move |error| format!("Unable to {context}: {error}")
}

struct Migration {
    id: &'static str,
    position: i64,
    sql: &'static str,
    prepare: Option<fn(&Connection) -> Result<(), String>>,
}

const ARCHIVED_PROTOTYPE_MIGRATIONS: [(&str, i64); 3] = [
    ("006_orchestration_drafts_schema", 5),
    ("007_orchestration_stage_runs_schema", 6),
    ("008_agent_sessions_schema", 7),
];

fn validate_migration_registration(migrations: &[Migration]) -> Result<(), String> {
    let mut ids = HashSet::new();
    let mut positions = HashSet::new();

    for migration in migrations {
        if !ids.insert(migration.id) {
            return Err(format!("Duplicate SQLite migration id: {}", migration.id));
        }

        if migration.position < 0 {
            return Err(format!(
                "Invalid SQLite migration position for {}: {}",
                migration.id, migration.position
            ));
        }

        if !positions.insert(migration.position) {
            return Err(format!(
                "Duplicate SQLite migration position: {}",
                migration.position
            ));
        }

        if let Some((reserved_id, reserved_position)) =
            ARCHIVED_PROTOTYPE_MIGRATIONS.iter().find(|(id, position)| {
                migration.id.starts_with(&id[..4]) || *position == migration.position
            })
        {
            return Err(format!(
                "SQLite migration {} at position {} reuses archived prototype migration {} at position {}",
                migration.id, migration.position, reserved_id, reserved_position
            ));
        }
    }

    Ok(())
}

fn app_migrations() -> [Migration; 6] {
    [
        Migration {
            id: "001_repo_sync_schema",
            position: 0,
            sql: REPO_SYNC_SCHEMA,
            prepare: None,
        },
        Migration {
            id: "002_open_tasks_schema",
            position: 1,
            sql: TASK_SCHEMA,
            prepare: None,
        },
        Migration {
            id: "003_task_runs_conversations_schema",
            position: 2,
            sql: RUN_CONVERSATION_SCHEMA,
            prepare: None,
        },
        Migration {
            id: "004_artifacts_validation_runs_schema",
            position: 3,
            sql: ARTIFACT_VALIDATION_SCHEMA,
            prepare: None,
        },
        Migration {
            id: "005_events_schema",
            position: 4,
            sql: EVENT_SCHEMA,
            prepare: None,
        },
        Migration {
            id: "009_durable_agent_sessions_schema",
            position: 8,
            sql: agent_sessions::repository::AGENT_SESSION_SCHEMA,
            prepare: Some(agent_sessions::repository::quarantine_archived_prototype_tables),
        },
    ]
}

const REPO_SYNC_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS repos (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  name TEXT NOT NULL,
  root_path TEXT NOT NULL,
  default_branch TEXT,
  remote_url TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
  UNIQUE (project_id, root_path)
);

CREATE TABLE IF NOT EXISTS branches (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL,
  name TEXT NOT NULL,
  base_branch TEXT,
  head_sha TEXT,
  intent TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE,
  UNIQUE (repo_id, name)
);

CREATE TABLE IF NOT EXISTS worktrees (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL,
  branch_id TEXT,
  path TEXT NOT NULL,
  is_main INTEGER NOT NULL CHECK (is_main IN (0, 1)),
  is_dirty INTEGER NOT NULL CHECK (is_dirty IN (0, 1)),
  lock_reason TEXT,
  last_scanned_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE,
  FOREIGN KEY (branch_id) REFERENCES branches(id) ON DELETE SET NULL,
  UNIQUE (repo_id, path)
);
";

const TASK_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  repo_id TEXT,
  branch_id TEXT,
  worktree_id TEXT,
  title TEXT NOT NULL,
  summary TEXT NOT NULL,
  execution_state TEXT NOT NULL CHECK (execution_state IN ('draft', 'queued', 'running', 'blocked', 'completed', 'failed', 'abandoned', 'archived')),
  attention_state TEXT NOT NULL CHECK (attention_state IN ('needs_action_now', 'needs_review', 'waiting_on_agent', 'waiting_on_external', 'consider_later', 'snoozed', 'reference_only')),
  priority TEXT NOT NULL CHECK (priority IN ('low', 'normal', 'high')),
  due_at TEXT,
  snoozed_until TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
  FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE SET NULL,
  FOREIGN KEY (branch_id) REFERENCES branches(id) ON DELETE SET NULL,
  FOREIGN KEY (worktree_id) REFERENCES worktrees(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS task_conversation_links (
  task_id TEXT NOT NULL,
  conversation_id TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  created_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
  PRIMARY KEY (task_id, conversation_id),
  UNIQUE (task_id, position)
);
";

const RUN_CONVERSATION_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS task_runs (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  conversation_id TEXT,
  worktree_id TEXT,
  execution_state TEXT NOT NULL CHECK (execution_state IN ('draft', 'queued', 'running', 'blocked', 'completed', 'failed', 'abandoned', 'archived')),
  started_at TEXT,
  completed_at TEXT,
  exit_code INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
  FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL,
  FOREIGN KEY (worktree_id) REFERENCES worktrees(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS conversations (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  task_run_id TEXT,
  provider TEXT NOT NULL CHECK (provider IN ('codex', 'chatgpt_export', 'manual')),
  external_thread_id TEXT,
  title TEXT NOT NULL,
  summary TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
  FOREIGN KEY (task_run_id) REFERENCES task_runs(id) ON DELETE SET NULL
);
";

const ARTIFACT_VALIDATION_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  task_run_id TEXT,
  conversation_id TEXT,
  kind TEXT NOT NULL CHECK (kind IN ('final_response', 'diff', 'validation_log', 'note', 'screenshot', 'handoff', 'summary', 'raw_event_stream')),
  title TEXT NOT NULL,
  uri TEXT,
  content TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
  FOREIGN KEY (task_run_id) REFERENCES task_runs(id) ON DELETE SET NULL,
  FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS validation_runs (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  task_run_id TEXT,
  command TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'passed', 'failed', 'canceled')),
  started_at TEXT,
  completed_at TEXT,
  exit_code INTEGER,
  output_artifact_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
  FOREIGN KEY (task_run_id) REFERENCES task_runs(id) ON DELETE SET NULL,
  FOREIGN KEY (output_artifact_id) REFERENCES artifacts(id) ON DELETE SET NULL
);
";

const EVENT_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS events (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('task_created', 'task_updated', 'attention_changed', 'execution_changed', 'run_started', 'run_event', 'run_completed', 'artifact_created', 'validation_started', 'validation_completed')),
  occurred_at TEXT NOT NULL,
  project_id TEXT,
  task_id TEXT,
  task_run_id TEXT,
  conversation_id TEXT,
  artifact_id TEXT,
  validation_run_id TEXT,
  payload_json TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
  FOREIGN KEY (task_run_id) REFERENCES task_runs(id) ON DELETE SET NULL,
  FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL,
  FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE SET NULL,
  FOREIGN KEY (validation_run_id) REFERENCES validation_runs(id) ON DELETE SET NULL
);
";

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::named_params;
    use serde_json::json;
    use std::cell::RefCell;

    const CREATED_AT: &str = "2026-07-02T10:00:00.000Z";

    #[test]
    fn legacy_task_commands_are_fail_closed_in_the_reset_baseline() {
        assert_eq!(
            ensure_legacy_tasks_available().expect_err("legacy tasks stay quarantined"),
            "Legacy Tasks are quarantined in the Agent Session reset baseline"
        );
    }

    struct FakeCodexRunner {
        result: Result<CodexCommandRunResult, String>,
        calls: RefCell<Vec<CodexCommandRunInput>>,
    }

    impl FakeCodexRunner {
        fn new(result: Result<CodexCommandRunResult, String>) -> Self {
            Self {
                result,
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl CodexCommandRunner for FakeCodexRunner {
        fn run(&self, input: CodexCommandRunInput) -> Result<CodexCommandRunResult, String> {
            self.calls.borrow_mut().push(input);
            self.result.clone()
        }
    }

    struct FakeGitDiffRunner {
        result: Result<GitDiffRunResult, String>,
        calls: RefCell<Vec<GitDiffRunInput>>,
    }

    impl FakeGitDiffRunner {
        fn new(result: Result<GitDiffRunResult, String>) -> Self {
            Self {
                result,
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl GitDiffRunner for FakeGitDiffRunner {
        fn collect_tracked_diff(&self, input: GitDiffRunInput) -> Result<GitDiffRunResult, String> {
            self.calls.borrow_mut().push(input);
            self.result.clone()
        }
    }

    struct FakeValidationCommandRunner {
        result: Result<ValidationCommandRunResult, String>,
        calls: RefCell<Vec<ValidationCommandRunInput>>,
    }

    impl FakeValidationCommandRunner {
        fn new(result: Result<ValidationCommandRunResult, String>) -> Self {
            Self {
                result,
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl ValidationCommandRunner for FakeValidationCommandRunner {
        fn run(
            &self,
            input: ValidationCommandRunInput,
        ) -> Result<ValidationCommandRunResult, String> {
            self.calls.borrow_mut().push(input);
            self.result.clone()
        }
    }

    #[test]
    fn initializes_database_with_archived_prototype_migration_records() {
        let conn = Connection::open_in_memory().expect("memory database");
        conn.execute_batch(
            "
CREATE TABLE schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE (position)
);

INSERT INTO schema_migrations (id, applied_at, position) VALUES
  ('006_orchestration_drafts_schema', 'prototype-006', 5),
  ('007_orchestration_stage_runs_schema', 'prototype-007', 6),
  ('008_agent_sessions_schema', 'prototype-008', 7);

CREATE TABLE agent_sessions (
  id TEXT PRIMARY KEY,
  codex_session_id TEXT,
  status TEXT NOT NULL,
  command TEXT NOT NULL,
  args_json TEXT NOT NULL,
  cwd TEXT,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  exit_code INTEGER,
  error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE agent_session_cli_logs (
  id TEXT PRIMARY KEY,
  agent_session_id TEXT NOT NULL,
  stream_id TEXT NOT NULL,
  stdout TEXT NOT NULL,
  stderr TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (agent_session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
);
INSERT INTO agent_sessions VALUES (
  'prototype-session', 'codex-thread', 'completed', 'codex', '[\"exec\"]', 'C:/work',
  '2026-07-01T00:00:00Z', '2026-07-01T00:01:00Z', 0, NULL,
  '2026-07-01T00:00:00Z', '2026-07-01T00:01:00Z'
);
INSERT INTO agent_session_cli_logs VALUES (
  'prototype-log', 'prototype-session', 'stream', 'stdout', 'stderr',
  '2026-07-01T00:01:00Z'
);
",
        )
        .expect("seed prototype migration ledger");

        initialize_database(&conn).expect("initialize around prototype ledger");
        let mut stmt = conn
            .prepare("SELECT id, position FROM schema_migrations ORDER BY position")
            .expect("prepare migration ledger query");
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .expect("query migration ledger")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect migration ledger");

        assert_eq!(
            rows,
            vec![
                ("001_repo_sync_schema".to_string(), 0),
                ("002_open_tasks_schema".to_string(), 1),
                ("003_task_runs_conversations_schema".to_string(), 2),
                ("004_artifacts_validation_runs_schema".to_string(), 3),
                ("005_events_schema".to_string(), 4),
                ("006_orchestration_drafts_schema".to_string(), 5),
                ("007_orchestration_stage_runs_schema".to_string(), 6),
                ("008_agent_sessions_schema".to_string(), 7),
                ("009_durable_agent_sessions_schema".to_string(), 8),
            ]
        );

        assert_eq!(
            conn.query_row(
                "SELECT codex_session_id FROM archived_prototype_agent_sessions_008 WHERE id = 'prototype-session'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("quarantined prototype session"),
            "codex-thread"
        );
        assert_eq!(
            conn.query_row(
                "SELECT stdout FROM archived_prototype_agent_session_cli_logs_008 WHERE id = 'prototype-log'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("quarantined prototype log"),
            "stdout"
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM agent_sessions", [], |row| row
                .get::<_, i64>(0))
                .expect("new durable Agent Session table"),
            0
        );
    }

    #[test]
    fn initializes_clean_database_with_durable_agent_session_schema_at_position_eight() {
        let conn = Connection::open_in_memory().expect("memory database");

        initialize_database(&conn).expect("initialize clean database");

        assert_eq!(
            conn.query_row(
                "SELECT position FROM schema_migrations WHERE id = '009_durable_agent_sessions_schema'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("durable Agent Session migration ledger row"),
            8
        );
        let tables = [
            "agent_sessions",
            "agent_session_invocations",
            "agent_session_runtime_events",
            "agent_session_invocation_diagnostics",
        ];
        for table in tables {
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get::<_, i64>(0),
                )
                .expect("query durable table"),
                1,
                "missing {table}"
            );
        }
    }

    #[test]
    fn initializes_database_with_archived_ledger_but_no_prototype_tables() {
        let conn = Connection::open_in_memory().expect("memory database");
        conn.execute_batch(
            "
CREATE TABLE schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE (position)
);
INSERT INTO schema_migrations (id, applied_at, position) VALUES
  ('006_orchestration_drafts_schema', 'prototype-006', 5),
  ('007_orchestration_stage_runs_schema', 'prototype-007', 6),
  ('008_agent_sessions_schema', 'prototype-008', 7);
",
        )
        .expect("seed archived ledger only");

        initialize_database(&conn).expect("initialize archived ledger without tables");

        assert_eq!(
            conn.query_row(
                "SELECT position FROM schema_migrations WHERE id = '009_durable_agent_sessions_schema'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("durable migration"),
            8
        );
    }

    #[test]
    fn rejects_unrecognized_agent_session_table_without_altering_it_or_recording_009() {
        let conn = Connection::open_in_memory().expect("memory database");
        conn.execute_batch(
            "
CREATE TABLE schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE (position)
);
CREATE TABLE agent_sessions (id TEXT PRIMARY KEY, unexpected TEXT NOT NULL);
INSERT INTO agent_sessions VALUES ('keep-me', 'untouched');
",
        )
        .expect("seed unrecognized collision");

        let error = initialize_database(&conn).expect_err("reject unknown Agent Session table");

        assert!(error.contains("not the recognized archived 008 prototype shape"));
        assert_eq!(
            conn.query_row(
                "SELECT unexpected FROM agent_sessions WHERE id = 'keep-me'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("original table remains"),
            "untouched"
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE id = '009_durable_agent_sessions_schema'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("query migration ledger"),
            0
        );
    }

    #[test]
    fn rejects_an_applied_migration_at_a_changed_position() {
        let conn = Connection::open_in_memory().expect("memory database");
        conn.execute_batch(
            "
CREATE TABLE schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE (position)
);
INSERT INTO schema_migrations (id, applied_at, position)
VALUES ('001_repo_sync_schema', 'prototype', 1);
",
        )
        .expect("seed changed migration position");

        let error = initialize_database(&conn).expect_err("reject changed migration position");

        assert_eq!(
            error,
            "SQLite migration 001_repo_sync_schema is recorded at position 1; expected 0"
        );
    }

    #[test]
    fn load_dashboard_returns_empty_snapshot_for_new_database() {
        let conn = open_memory_database();

        let snapshot = load_dashboard_snapshot(&conn).expect("snapshot");

        assert_eq!(snapshot.total_open_tasks, 0);
        assert_eq!(snapshot.projects, Vec::<TaskDashboardProject>::new());
        assert_eq!(
            snapshot.worktree_anchors,
            Vec::<TaskDashboardWorktreeAnchor>::new()
        );
        assert_eq!(
            snapshot
                .groups
                .iter()
                .map(|group| group.id)
                .collect::<Vec<_>>(),
            vec![
                "needs_action_now",
                "review_decide",
                "working",
                "waiting",
                "later"
            ]
        );
    }

    #[test]
    fn create_update_and_archive_open_task_are_durable() {
        let conn = open_memory_database();
        seed_project(&conn);

        create_task(
            &conn,
            CreateOpenTaskCommandInput {
                project_id: "project-1".to_string(),
                repo_id: None,
                branch_id: None,
                worktree_id: None,
                title: "Persist Tauri tasks".to_string(),
                summary: "Create through the Rust SQLite backend.".to_string(),
                execution_state: None,
                attention_state: None,
                priority: None,
            },
        )
        .expect("create task");

        let created = load_dashboard_snapshot(&conn).expect("created snapshot");
        let created_task = created
            .groups
            .iter()
            .flat_map(|group| &group.tasks)
            .next()
            .expect("created task");
        assert_eq!(created.total_open_tasks, 1);
        assert_eq!(created_task.execution_state, "draft");
        assert_eq!(created_task.attention_state, "needs_action_now");

        let task_id = created_task.id.clone();
        update_task(
            &conn,
            &task_id,
            UpdateOpenTaskCommandInput {
                title: Some("Updated Tauri task".to_string()),
                summary: Some("Update through the Rust SQLite backend.".to_string()),
                execution_state: Some("completed".to_string()),
                attention_state: Some("needs_review".to_string()),
                priority: Some("high".to_string()),
            },
        )
        .expect("update task");

        let updated = load_dashboard_snapshot(&conn).expect("updated snapshot");
        assert_eq!(updated.groups[1].id, "review_decide");
        assert_eq!(updated.groups[1].tasks[0].id, task_id);
        assert_eq!(updated.groups[1].tasks[0].priority, "high");

        archive_task(&conn, &task_id).expect("archive task");
        let archived = load_dashboard_snapshot(&conn).expect("archived snapshot");
        assert_eq!(archived.total_open_tasks, 0);
    }

    #[test]
    fn register_worktree_anchor_allows_creating_runnable_task() {
        let conn = open_memory_database();

        register_task_worktree_anchor(
            &conn,
            RegisterTaskWorktreeCommandInput {
                project_name: "Codex Orchestrator".to_string(),
                repo_name: None,
                repo_root_path: "C:/Repos/Codex Orchestrator".to_string(),
                branch_name: Some("main".to_string()),
                worktree_path: "C:/Repos/Codex Orchestrator".to_string(),
                is_main: Some(true),
            },
        )
        .expect("register worktree");

        let registered = load_dashboard_snapshot(&conn).expect("registered snapshot");
        let anchor = registered
            .worktree_anchors
            .first()
            .expect("registered worktree anchor");
        assert_eq!(anchor.project, "Codex Orchestrator");
        assert_eq!(anchor.repo, "Codex Orchestrator");
        assert_eq!(anchor.branch.as_deref(), Some("main"));

        create_task(
            &conn,
            CreateOpenTaskCommandInput {
                project_id: anchor.project_id.clone(),
                repo_id: Some(anchor.repo_id.clone()),
                branch_id: anchor.branch_id.clone(),
                worktree_id: Some(anchor.id.clone()),
                title: "Run through registered worktree".to_string(),
                summary: "Task created with a runnable technical anchor.".to_string(),
                execution_state: None,
                attention_state: None,
                priority: None,
            },
        )
        .expect("create anchored task");

        let created = load_dashboard_snapshot(&conn).expect("created snapshot");
        let task = created
            .groups
            .iter()
            .flat_map(|group| &group.tasks)
            .next()
            .expect("created anchored task");
        assert_eq!(task.repo.as_deref(), Some("Codex Orchestrator"));
        assert_eq!(task.branch.as_deref(), Some("main"));
        assert_eq!(
            task.worktree_path.as_deref(),
            Some("C:/Repos/Codex Orchestrator")
        );
    }

    #[test]
    fn parse_git_worktree_list_returns_branch_anchors() {
        let output = "\
worktree C:/Repos/Codex Orchestrator
HEAD abc123
branch refs/heads/main

worktree C:/Repos/Codex Orchestrator Worktrees/feature
HEAD def456
branch refs/heads/worker/feature
";

        let worktrees = parse_git_worktree_list(output, "C:/Repos/Codex Orchestrator");

        assert_eq!(
            worktrees,
            vec![
                GitWorktreeFacts {
                    path: "C:/Repos/Codex Orchestrator".to_string(),
                    branch_name: Some("main".to_string()),
                    is_main: true,
                },
                GitWorktreeFacts {
                    path: "C:/Repos/Codex Orchestrator Worktrees/feature".to_string(),
                    branch_name: Some("worker/feature".to_string()),
                    is_main: false,
                },
            ]
        );
    }

    #[test]
    fn discover_git_repos_finds_repos_under_designated_root() {
        let root = std::env::temp_dir().join(format!("codex-orchestrator-scan-{}", Uuid::new_v4()));
        let repo = root.join("CodexOrchestrator");
        let nested_repo = root.join("Nested").join("Tooling");
        fs::create_dir_all(repo.join(".git")).expect("create repo marker");
        fs::create_dir_all(nested_repo.join(".git")).expect("create nested repo marker");
        fs::create_dir_all(root.join("node_modules").join("ignored").join(".git"))
            .expect("create ignored repo marker");

        let repos = discover_git_repos(DiscoverTaskReposCommandInput {
            root_path: root.to_string_lossy().to_string(),
            max_depth: Some(3),
        })
        .expect("discover repos");

        assert_eq!(
            repos
                .iter()
                .map(|repo| repo.name.as_str())
                .collect::<Vec<_>>(),
            vec!["CodexOrchestrator", "Tooling"]
        );

        fs::remove_dir_all(root).expect("remove temp scan root");
    }

    #[test]
    fn register_repo_anchor_scans_git_worktrees_for_runnable_task_anchor() {
        let git_available = Command::new("git").arg("--version").output().is_ok();

        if !git_available {
            return;
        }

        let root = std::env::temp_dir().join(format!("codex-orchestrator-repo-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp repo");
        let init_output = Command::new("git")
            .arg("init")
            .arg("-b")
            .arg("main")
            .current_dir(&root)
            .output()
            .expect("run git init");

        if !init_output.status.success() {
            fs::remove_dir_all(root).expect("remove temp repo");
            return;
        }

        let conn = open_memory_database();
        register_task_repo_anchor(
            &conn,
            RegisterTaskRepoCommandInput {
                repo_root_path: root.to_string_lossy().to_string(),
                project_name: None,
                repo_name: None,
            },
        )
        .expect("register repo");

        let snapshot = load_dashboard_snapshot(&conn).expect("load dashboard");
        let anchor = snapshot
            .worktree_anchors
            .first()
            .expect("repo worktree anchor");

        assert_eq!(snapshot.projects.len(), 1);
        assert_eq!(anchor.branch.as_deref(), Some("main"));
        assert!(same_filesystem_path(&anchor.path, &root.to_string_lossy()));

        fs::remove_dir_all(root).expect("remove temp repo");
    }

    #[test]
    fn load_dashboard_resolves_technical_anchors_and_omits_closed_tasks() {
        let conn = open_memory_database();
        seed_project_repo_branch_worktree(&conn);
        insert_task(
            &conn,
            "task-open",
            "running",
            "waiting_on_agent",
            Some("repo-1"),
            Some("branch-1"),
            Some("worktree-1"),
        );
        insert_task(
            &conn,
            "task-archived",
            "archived",
            "needs_action_now",
            None,
            None,
            None,
        );

        let snapshot = load_dashboard_snapshot(&conn).expect("snapshot");
        let working_group = snapshot
            .groups
            .iter()
            .find(|group| group.id == "working")
            .expect("working group");
        let task = &working_group.tasks[0];

        assert_eq!(snapshot.total_open_tasks, 1);
        assert_eq!(task.repo.as_deref(), Some("Codex Orchestrator"));
        assert_eq!(task.branch.as_deref(), Some("worker/test"));
        assert_eq!(
            task.worktree_path.as_deref(),
            Some("C:/Repos/Codex Orchestrator")
        );
    }

    #[test]
    fn missing_task_writes_return_not_found() {
        let conn = open_memory_database();

        let error = archive_task(&conn, "task-missing").expect_err("missing task");

        assert_eq!(error, "Open task not found: task-missing");
    }

    #[test]
    fn load_task_run_detail_returns_clear_error_for_missing_task() {
        let conn = open_memory_database();

        let error = load_task_run_detail_snapshot(&conn, "task-missing").expect_err("missing task");

        assert_eq!(error, "Task not found: task-missing");
    }

    #[test]
    fn load_task_run_detail_resolves_task_anchors_and_conversation_links() {
        let conn = open_memory_database();
        seed_project_repo_branch_worktree(&conn);
        insert_task(
            &conn,
            "task-detail",
            "completed",
            "needs_review",
            Some("repo-1"),
            Some("branch-1"),
            Some("worktree-1"),
        );
        insert_conversation_link(&conn, "task-detail", "conversation-2", 1);
        insert_conversation_link(&conn, "task-detail", "conversation-1", 0);

        let snapshot = load_task_run_detail_snapshot(&conn, "task-detail").expect("snapshot");

        assert_eq!(snapshot.task.record.id, "task-detail");
        assert_eq!(
            snapshot.task.record.conversation_ids,
            vec!["conversation-1", "conversation-2"]
        );
        assert_eq!(
            snapshot.task.project.expect("project").name,
            "Codex Orchestrator"
        );
        assert_eq!(
            snapshot.task.repo.expect("repo").root_path,
            "C:/Repos/Codex Orchestrator"
        );
        assert_eq!(snapshot.task.branch.expect("branch").name, "worker/test");
        assert_eq!(
            snapshot.task.worktree.expect("worktree").path,
            "C:/Repos/Codex Orchestrator"
        );
    }

    #[test]
    fn load_task_run_detail_groups_runs_artifacts_validations_and_events() {
        let conn = open_memory_database();
        seed_project_repo_branch_worktree(&conn);
        insert_task(
            &conn,
            "task-detail",
            "completed",
            "needs_review",
            Some("repo-1"),
            Some("branch-1"),
            Some("worktree-1"),
        );
        insert_task_run(
            &conn,
            "run-old",
            "task-detail",
            "completed",
            Some("2026-07-02T10:15:00.000Z"),
            Some("2026-07-02T10:20:00.000Z"),
            "2026-07-02T10:10:00.000Z",
        );
        insert_task_run(
            &conn,
            "run-new",
            "task-detail",
            "completed",
            Some("2026-07-02T11:15:00.000Z"),
            Some("2026-07-02T11:20:00.000Z"),
            "2026-07-02T11:10:00.000Z",
        );
        insert_artifact(
            &conn,
            "artifact-final",
            "task-detail",
            Some("run-new"),
            "final_response",
            "Final response",
            Some("Done"),
            "2026-07-02T11:21:00.000Z",
        );
        insert_artifact(
            &conn,
            "artifact-raw",
            "task-detail",
            Some("run-new"),
            "raw_event_stream",
            "Raw JSONL",
            Some("{}"),
            "2026-07-02T11:22:00.000Z",
        );
        insert_artifact(
            &conn,
            "artifact-diff",
            "task-detail",
            Some("run-new"),
            "diff",
            "Git diff",
            Some("diff --git"),
            "2026-07-02T11:23:00.000Z",
        );
        insert_artifact(
            &conn,
            "artifact-validation-direct",
            "task-detail",
            Some("run-new"),
            "validation_log",
            "Direct validation",
            Some("passed"),
            "2026-07-02T11:24:00.000Z",
        );
        insert_artifact(
            &conn,
            "artifact-validation-linked",
            "task-detail",
            Some("run-new"),
            "validation_log",
            "Linked validation",
            Some("passed by artifact"),
            "2026-07-02T11:25:00.000Z",
        );
        insert_artifact(
            &conn,
            "artifact-old-summary",
            "task-detail",
            Some("run-old"),
            "summary",
            "Old summary",
            Some("Older run"),
            "2026-07-02T10:21:00.000Z",
        );
        insert_artifact(
            &conn,
            "artifact-task-note",
            "task-detail",
            None,
            "note",
            "Task note",
            Some("Review this"),
            "2026-07-02T11:30:00.000Z",
        );
        insert_artifact(
            &conn,
            "artifact-unlinked-validation",
            "task-detail",
            None,
            "validation_log",
            "Unlinked validation",
            Some("failed"),
            "2026-07-02T11:35:00.000Z",
        );
        insert_validation_run(
            &conn,
            "validation-direct",
            "task-detail",
            Some("run-new"),
            "npm test",
            "passed",
            Some("artifact-validation-direct"),
            Some(0),
            "2026-07-02T11:24:30.000Z",
            Some("2026-07-02T11:24:45.000Z"),
        );
        insert_validation_run(
            &conn,
            "validation-linked",
            "task-detail",
            None,
            "npm run lint",
            "passed",
            Some("artifact-validation-linked"),
            Some(0),
            "2026-07-02T11:25:30.000Z",
            Some("2026-07-02T11:25:45.000Z"),
        );
        insert_validation_run(
            &conn,
            "validation-unlinked",
            "task-detail",
            None,
            "cargo test",
            "failed",
            Some("artifact-unlinked-validation"),
            Some(1),
            "2026-07-02T11:35:30.000Z",
            Some("2026-07-02T11:35:45.000Z"),
        );
        insert_event(
            &conn,
            "event-late",
            "run_completed",
            "task-detail",
            Some("run-new"),
            "2026-07-02T11:26:00.000Z",
            json!({ "status": "completed" }),
        );
        insert_event(
            &conn,
            "event-early",
            "run_started",
            "task-detail",
            Some("run-old"),
            "2026-07-02T10:15:00.000Z",
            json!({ "status": "running" }),
        );
        insert_event(
            &conn,
            "event-middle",
            "artifact_created",
            "task-detail",
            None,
            "2026-07-02T11:23:00.000Z",
            json!({ "artifactId": "artifact-diff" }),
        );

        let snapshot = load_task_run_detail_snapshot(&conn, "task-detail").expect("snapshot");

        assert_eq!(
            snapshot
                .runs
                .iter()
                .map(|run| run.run.id.as_str())
                .collect::<Vec<_>>(),
            vec!["run-new", "run-old"]
        );

        let newest_run = &snapshot.runs[0];
        assert_eq!(newest_run.artifacts.final_responses[0].id, "artifact-final");
        assert_eq!(newest_run.artifacts.raw_event_streams[0].id, "artifact-raw");
        assert_eq!(newest_run.artifacts.diffs[0].id, "artifact-diff");
        assert_eq!(
            newest_run
                .artifacts
                .validation_logs
                .iter()
                .map(|artifact| artifact.id.as_str())
                .collect::<Vec<_>>(),
            vec!["artifact-validation-direct", "artifact-validation-linked"]
        );
        assert_eq!(
            newest_run
                .validation_runs
                .iter()
                .map(|validation_run| validation_run.run.id.as_str())
                .collect::<Vec<_>>(),
            vec!["validation-direct", "validation-linked"]
        );
        assert_eq!(
            newest_run.validation_runs[1]
                .output_artifact
                .as_ref()
                .expect("linked output artifact")
                .id,
            "artifact-validation-linked"
        );
        assert_eq!(
            snapshot.runs[1].artifacts.summaries[0].id,
            "artifact-old-summary"
        );
        assert_eq!(
            snapshot.unlinked_artifacts.notes[0].id,
            "artifact-task-note"
        );
        assert!(snapshot.unlinked_artifacts.validation_logs.is_empty());
        assert_eq!(
            snapshot.unlinked_validation_runs[0].run.id,
            "validation-unlinked"
        );
        assert_eq!(
            snapshot.unlinked_validation_runs[0]
                .output_artifact
                .as_ref()
                .expect("unlinked output artifact")
                .id,
            "artifact-unlinked-validation"
        );
        assert_eq!(
            snapshot
                .event_timeline
                .iter()
                .map(|event| event.id.as_str())
                .collect::<Vec<_>>(),
            vec!["event-early", "event-middle", "event-late"]
        );
        assert_eq!(
            newest_run
                .events
                .iter()
                .map(|event| event.id.as_str())
                .collect::<Vec<_>>(),
            vec!["event-late"]
        );
    }

    #[test]
    fn start_codex_task_run_executes_codex_and_persists_completed_lifecycle() {
        let conn = open_memory_database();
        seed_project_repo_branch_worktree(&conn);
        insert_task(
            &conn,
            "task-run",
            "draft",
            "needs_action_now",
            Some("repo-1"),
            Some("branch-1"),
            Some("worktree-1"),
        );
        let stdout = completed_codex_stdout("thread-123", "Done from Codex");
        let runner = FakeCodexRunner::new(Ok(CodexCommandRunResult {
            stdout: stdout.clone(),
            stderr: String::new(),
            exit_code: Some(0),
            signal: None,
        }));
        let mut env = BTreeMap::new();
        env.insert("CODEX_PROFILE".to_string(), Some("worker".to_string()));
        env.insert("REMOVE_ME".to_string(), None);

        let result = start_codex_task_run_with_runner(
            &conn,
            StartCodexTaskRunCommandInput {
                task_id: "task-run".to_string(),
                prompt: "Finish task".to_string(),
                cwd: Some("C:/Repos/Codex Orchestrator".to_string()),
                worktree_id: Some("worktree-1".to_string()),
                conversation_title: Some("Worker run".to_string()),
                conversation_summary: Some("Initial summary".to_string()),
                additional_args: Some(vec!["--sandbox".to_string(), "read-only".to_string()]),
                env: Some(env.clone()),
                post_run_capture: None,
            },
            &runner,
        )
        .expect("start run");

        assert_eq!(result.status, "completed");
        assert_eq!(result.exit_code, Some(0));
        assert!(result.raw_event_stream_artifact_id.is_some());
        assert!(result.final_response_artifact_id.is_some());
        assert_eq!(result.task.execution_state, "completed");
        assert_eq!(result.task.attention_state, "needs_review");
        assert_eq!(
            result.task.conversation_ids,
            vec![result.conversation_id.clone().unwrap()]
        );
        assert_eq!(result.task_run.execution_state, "completed");
        assert_eq!(result.task_run.worktree_id.as_deref(), Some("worktree-1"));

        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].command, "codex");
        assert_eq!(
            calls[0].args,
            vec!["exec", "--json", "--sandbox", "read-only", "Finish task"]
        );
        assert_eq!(calls[0].cwd.as_deref(), Some("C:/Repos/Codex Orchestrator"));
        assert_eq!(calls[0].env.as_ref(), Some(&env));

        let detail = load_task_run_detail_snapshot(&conn, "task-run").expect("detail");
        assert_eq!(detail.runs.len(), 1);
        assert_eq!(
            detail.runs[0].artifacts.raw_event_streams[0]
                .content
                .as_deref(),
            Some(stdout.as_str())
        );
        assert_eq!(
            detail.runs[0].artifacts.final_responses[0]
                .content
                .as_deref(),
            Some("Done from Codex")
        );
        assert_eq!(
            conversation_metadata(
                &conn,
                result.conversation_id.as_deref().expect("conversation id")
            ),
            (
                Some("thread-123".to_string()),
                Some("Codex completed: Done from Codex".to_string())
            )
        );
        assert_eq!(
            event_kinds(&conn, "task-run"),
            vec!["run_started", "artifact_created", "run_completed"]
        );
    }

    #[test]
    fn start_codex_task_run_persists_failed_codex_run_with_raw_stream() {
        let conn = open_memory_database();
        seed_project_repo_branch_worktree(&conn);
        insert_task(
            &conn,
            "task-failed-run",
            "draft",
            "needs_action_now",
            Some("repo-1"),
            Some("branch-1"),
            Some("worktree-1"),
        );
        let stdout = "{\"type\":\"turn.failed\"}\n".to_string();
        let runner = FakeCodexRunner::new(Ok(CodexCommandRunResult {
            stdout: stdout.clone(),
            stderr: "permission denied".to_string(),
            exit_code: Some(1),
            signal: None,
        }));

        let result = start_codex_task_run_with_runner(
            &conn,
            start_command_input("task-failed-run"),
            &runner,
        )
        .expect("failed run result");

        assert_eq!(result.status, "failed");
        assert_eq!(result.exit_code, Some(1));
        assert_eq!(
            result.status_reason.as_deref(),
            Some("Codex emitted a turn.failed event")
        );
        assert_eq!(
            result.error.as_deref(),
            Some("Codex emitted a turn.failed event: permission denied")
        );
        assert!(result.raw_event_stream_artifact_id.is_some());
        assert!(result.final_response_artifact_id.is_none());
        assert_eq!(result.task.execution_state, "failed");
        assert_eq!(result.task.attention_state, "needs_action_now");
        assert_eq!(result.task_run.execution_state, "failed");

        let detail = load_task_run_detail_snapshot(&conn, "task-failed-run").expect("detail");
        assert_eq!(
            detail.runs[0].artifacts.raw_event_streams[0]
                .content
                .as_deref(),
            Some(stdout.as_str())
        );
        assert!(detail.runs[0].artifacts.final_responses.is_empty());
        assert_eq!(
            event_kinds(&conn, "task-failed-run"),
            vec!["run_started", "artifact_created", "run_completed"]
        );
    }

    #[test]
    fn start_codex_task_run_marks_failed_when_process_launch_fails() {
        let conn = open_memory_database();
        seed_project_repo_branch_worktree(&conn);
        insert_task(
            &conn,
            "task-launch-error",
            "draft",
            "needs_action_now",
            Some("repo-1"),
            Some("branch-1"),
            Some("worktree-1"),
        );
        let runner = FakeCodexRunner::new(Err("Unable to launch Codex: not found".to_string()));

        let result = start_codex_task_run_with_runner(
            &conn,
            start_command_input("task-launch-error"),
            &runner,
        )
        .expect("launch failure result");

        assert_eq!(result.status, "failed");
        assert_eq!(
            result.error.as_deref(),
            Some("Unable to launch Codex: not found")
        );
        assert!(result.raw_event_stream_artifact_id.is_none());
        assert_eq!(artifact_count(&conn, "task-launch-error"), 0);
        assert_eq!(result.task.execution_state, "failed");
        assert_eq!(result.task_run.execution_state, "failed");
        assert_eq!(
            event_kinds(&conn, "task-launch-error"),
            vec!["run_started", "run_completed"]
        );
    }

    #[test]
    fn start_codex_task_run_preserves_raw_stream_before_jsonl_parse_failure() {
        let conn = open_memory_database();
        seed_project_repo_branch_worktree(&conn);
        insert_task(
            &conn,
            "task-parse-error",
            "draft",
            "needs_action_now",
            Some("repo-1"),
            Some("branch-1"),
            Some("worktree-1"),
        );
        let runner = FakeCodexRunner::new(Ok(CodexCommandRunResult {
            stdout: "{not json}\n".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            signal: None,
        }));

        let result = start_codex_task_run_with_runner(
            &conn,
            start_command_input("task-parse-error"),
            &runner,
        )
        .expect("parse failure result");

        assert_eq!(result.status, "failed");
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(
            result.status_reason.as_deref(),
            Some("Codex JSONL parse failed")
        );
        assert!(result
            .error
            .as_deref()
            .expect("parse error")
            .starts_with("Line 1: Invalid JSON"));
        assert!(result.raw_event_stream_artifact_id.is_some());

        let detail = load_task_run_detail_snapshot(&conn, "task-parse-error").expect("detail");
        assert_eq!(
            detail.runs[0].artifacts.raw_event_streams[0]
                .content
                .as_deref(),
            Some("{not json}\n")
        );
        assert!(detail.runs[0].artifacts.final_responses.is_empty());
        assert_eq!(
            event_kinds(&conn, "task-parse-error"),
            vec!["run_started", "artifact_created", "run_completed"]
        );
    }

    #[test]
    fn start_codex_task_run_collects_post_run_diff_when_requested() {
        let conn = open_memory_database();
        seed_project_repo_branch_worktree(&conn);
        insert_task(
            &conn,
            "task-diff-capture",
            "draft",
            "needs_action_now",
            Some("repo-1"),
            Some("branch-1"),
            Some("worktree-1"),
        );
        let codex_runner = FakeCodexRunner::new(Ok(CodexCommandRunResult {
            stdout: completed_codex_stdout("thread-diff", "Diff is ready"),
            stderr: String::new(),
            exit_code: Some(0),
            signal: None,
        }));
        let diff = "diff --git a/file.txt b/file.txt\n+changed\n".to_string();
        let git_diff_runner = FakeGitDiffRunner::new(Ok(GitDiffRunResult { diff: diff.clone() }));
        let validation_runner =
            FakeValidationCommandRunner::new(Err("validation should not run".to_string()));
        let mut input = start_command_input("task-diff-capture");
        input.post_run_capture = Some(StartCodexTaskRunPostRunCaptureInput {
            collect_diff: Some(true),
            validation_command: None,
        });

        let result = start_codex_task_run_with_runners(
            &conn,
            input,
            &codex_runner,
            &git_diff_runner,
            &validation_runner,
        )
        .expect("captured diff run");

        assert_eq!(result.status, "completed");
        let diff_capture = result
            .post_run_capture
            .as_ref()
            .and_then(|capture| capture.diff.as_ref())
            .expect("diff capture");
        assert_eq!(
            diff_capture,
            &StartCodexTaskRunDiffCaptureResult::Captured {
                artifact_id: match diff_capture {
                    StartCodexTaskRunDiffCaptureResult::Captured { artifact_id, .. } =>
                        artifact_id.clone(),
                    StartCodexTaskRunDiffCaptureResult::Failed { .. } => unreachable!(),
                },
                event_id: match diff_capture {
                    StartCodexTaskRunDiffCaptureResult::Captured { event_id, .. } =>
                        event_id.clone(),
                    StartCodexTaskRunDiffCaptureResult::Failed { .. } => unreachable!(),
                },
                diff_length: diff.len() as i64,
                is_empty_diff: false,
                worktree_path: "C:/Repos/Codex Orchestrator".to_string(),
            }
        );
        assert_eq!(
            git_diff_runner.calls.borrow().as_slice(),
            &[GitDiffRunInput {
                worktree_path: "C:/Repos/Codex Orchestrator".to_string()
            }]
        );
        assert!(validation_runner.calls.borrow().is_empty());

        let detail = load_task_run_detail_snapshot(&conn, "task-diff-capture").expect("detail");
        assert_eq!(
            detail.runs[0].artifacts.diffs[0].content.as_deref(),
            Some(diff.as_str())
        );
        assert_eq!(
            event_kinds(&conn, "task-diff-capture"),
            vec![
                "run_started",
                "artifact_created",
                "run_completed",
                "artifact_created"
            ]
        );
    }

    #[test]
    fn start_codex_task_run_runs_post_run_validation_when_requested() {
        let conn = open_memory_database();
        seed_project_repo_branch_worktree(&conn);
        insert_task(
            &conn,
            "task-validation-capture",
            "draft",
            "needs_action_now",
            Some("repo-1"),
            Some("branch-1"),
            Some("worktree-1"),
        );
        let codex_runner = FakeCodexRunner::new(Ok(CodexCommandRunResult {
            stdout: completed_codex_stdout("thread-validation", "Validation is ready"),
            stderr: String::new(),
            exit_code: Some(0),
            signal: None,
        }));
        let git_diff_runner = FakeGitDiffRunner::new(Err("git diff should not run".to_string()));
        let validation_runner = FakeValidationCommandRunner::new(Ok(ValidationCommandRunResult {
            stdout: "tests passed\n".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            signal: None,
        }));
        let mut env = BTreeMap::new();
        env.insert("CI".to_string(), Some("1".to_string()));
        let mut input = start_command_input("task-validation-capture");
        input.post_run_capture = Some(StartCodexTaskRunPostRunCaptureInput {
            collect_diff: None,
            validation_command: Some(StartCodexTaskRunValidationCommandInput {
                command: "npm".to_string(),
                args: Some(vec!["run".to_string(), "test".to_string()]),
                cwd: Some("C:/Repos/Codex Orchestrator/app".to_string()),
                env: Some(env.clone()),
            }),
        });

        let result = start_codex_task_run_with_runners(
            &conn,
            input,
            &codex_runner,
            &git_diff_runner,
            &validation_runner,
        )
        .expect("validation run");

        assert_eq!(result.status, "completed");
        let validation_capture = result
            .post_run_capture
            .as_ref()
            .and_then(|capture| capture.validation.as_ref())
            .expect("validation capture");
        assert_eq!(validation_capture.status, "passed");
        assert!(validation_capture.validation_run_id.is_some());
        assert!(validation_capture.output_artifact_id.is_some());
        assert_eq!(validation_capture.exit_code, Some(0));
        assert_eq!(
            validation_runner.calls.borrow().as_slice(),
            &[ValidationCommandRunInput {
                command: "npm".to_string(),
                args: vec!["run".to_string(), "test".to_string()],
                cwd: "C:/Repos/Codex Orchestrator/app".to_string(),
                env: Some(env)
            }]
        );
        assert!(git_diff_runner.calls.borrow().is_empty());

        let detail =
            load_task_run_detail_snapshot(&conn, "task-validation-capture").expect("detail");
        assert_eq!(detail.runs[0].validation_runs[0].run.status, "passed");
        assert_eq!(
            detail.runs[0].artifacts.validation_logs[0]
                .content
                .as_deref()
                .expect("validation log")
                .contains("\"stdout\": \"tests passed\\n\""),
            true
        );
        assert_eq!(
            detail.runs[0].validation_runs[0]
                .run
                .output_artifact_id
                .as_deref(),
            Some(detail.runs[0].artifacts.validation_logs[0].id.as_str())
        );
        assert_eq!(
            event_kinds(&conn, "task-validation-capture"),
            vec![
                "run_started",
                "artifact_created",
                "run_completed",
                "validation_started",
                "artifact_created",
                "validation_completed"
            ]
        );
    }

    #[test]
    fn start_codex_task_run_reports_post_run_capture_failures_after_completed_run() {
        let conn = open_memory_database();
        seed_project_repo_branch_worktree(&conn);
        insert_task(
            &conn,
            "task-capture-failures",
            "draft",
            "needs_action_now",
            Some("repo-1"),
            Some("branch-1"),
            Some("worktree-1"),
        );
        let codex_runner = FakeCodexRunner::new(Ok(CodexCommandRunResult {
            stdout: completed_codex_stdout("thread-capture-failures", "Capture can fail"),
            stderr: String::new(),
            exit_code: Some(0),
            signal: None,
        }));
        let git_diff_runner = FakeGitDiffRunner::new(Err("git diff failed".to_string()));
        let validation_runner =
            FakeValidationCommandRunner::new(Err("validation launch failed".to_string()));
        let mut input = start_command_input("task-capture-failures");
        input.post_run_capture = Some(StartCodexTaskRunPostRunCaptureInput {
            collect_diff: Some(true),
            validation_command: Some(StartCodexTaskRunValidationCommandInput {
                command: "npm".to_string(),
                args: Some(vec!["run".to_string(), "lint".to_string()]),
                cwd: None,
                env: None,
            }),
        });

        let result = start_codex_task_run_with_runners(
            &conn,
            input,
            &codex_runner,
            &git_diff_runner,
            &validation_runner,
        )
        .expect("completed run with capture failures");

        assert_eq!(result.status, "completed");
        assert_eq!(result.task.execution_state, "completed");
        assert_eq!(result.task_run.execution_state, "completed");
        let capture = result.post_run_capture.as_ref().expect("capture result");
        assert_eq!(
            capture.diff,
            Some(StartCodexTaskRunDiffCaptureResult::Failed {
                error: "git diff failed".to_string()
            })
        );
        let validation = capture.validation.as_ref().expect("validation capture");
        assert_eq!(validation.status, "failed");
        assert_eq!(
            validation.error.as_deref(),
            Some("validation launch failed")
        );
        assert!(validation.validation_run_id.is_some());
        assert!(validation.output_artifact_id.is_some());
        assert_eq!(git_diff_runner.calls.borrow().len(), 1);
        assert_eq!(validation_runner.calls.borrow().len(), 1);

        let detail = load_task_run_detail_snapshot(&conn, "task-capture-failures").expect("detail");
        assert!(detail.runs[0].artifacts.diffs.is_empty());
        assert_eq!(detail.runs[0].validation_runs[0].run.status, "failed");
        assert_eq!(
            event_kinds(&conn, "task-capture-failures"),
            vec![
                "run_started",
                "artifact_created",
                "run_completed",
                "validation_started",
                "artifact_created",
                "validation_completed"
            ]
        );
    }

    fn open_memory_database() -> Connection {
        let conn = Connection::open_in_memory().expect("memory database");
        initialize_database(&conn).expect("initialize database");
        conn
    }

    fn completed_codex_stdout(thread_id: &str, final_message: &str) -> String {
        [
            format!(r#"{{"type":"thread.started","thread_id":"{thread_id}"}}"#),
            r#"{"type":"turn.started"}"#.to_string(),
            format!(
                r#"{{"type":"item.completed","item":{{"type":"agent_message","text":"{final_message}"}}}}"#
            ),
            r#"{"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":2}}"#
                .to_string(),
        ]
        .join("\n")
    }

    fn start_command_input(task_id: &str) -> StartCodexTaskRunCommandInput {
        StartCodexTaskRunCommandInput {
            task_id: task_id.to_string(),
            prompt: "Run Codex".to_string(),
            cwd: Some("C:/Repos/Codex Orchestrator".to_string()),
            worktree_id: Some("worktree-1".to_string()),
            conversation_title: None,
            conversation_summary: None,
            additional_args: None,
            env: None,
            post_run_capture: None,
        }
    }

    fn conversation_metadata(
        conn: &Connection,
        conversation_id: &str,
    ) -> (Option<String>, Option<String>) {
        conn.query_row(
            "SELECT external_thread_id, summary FROM conversations WHERE id = ?1",
            params![conversation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("conversation metadata")
    }

    fn event_kinds(conn: &Connection, task_id: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT kind FROM events WHERE task_id = ?1 ORDER BY rowid")
            .expect("prepare event kinds");
        stmt.query_map(params![task_id], |row| row.get(0))
            .expect("query event kinds")
            .collect::<Result<Vec<_>, _>>()
            .expect("event kinds")
    }

    fn artifact_count(conn: &Connection, task_id: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM artifacts WHERE task_id = ?1",
            params![task_id],
            |row| row.get(0),
        )
        .expect("artifact count")
    }

    fn seed_project(conn: &Connection) {
        conn.execute(
            "
INSERT INTO projects (id, name, description, created_at, updated_at)
VALUES ('project-1', 'Codex Orchestrator', NULL, ?1, ?1)
",
            params![CREATED_AT],
        )
        .expect("seed project");
    }

    fn seed_project_repo_branch_worktree(conn: &Connection) {
        seed_project(conn);
        conn.execute(
            "
INSERT INTO repos (id, project_id, name, root_path, default_branch, remote_url, created_at, updated_at)
VALUES ('repo-1', 'project-1', 'Codex Orchestrator', 'C:/Repos/Codex Orchestrator', 'main', NULL, ?1, ?1)
",
            params![CREATED_AT],
        )
        .expect("seed repo");
        conn.execute(
            "
INSERT INTO branches (id, repo_id, name, base_branch, head_sha, intent, created_at, updated_at)
VALUES ('branch-1', 'repo-1', 'worker/test', 'main', NULL, NULL, ?1, ?1)
",
            params![CREATED_AT],
        )
        .expect("seed branch");
        conn.execute(
            "
INSERT INTO worktrees (id, repo_id, branch_id, path, is_main, is_dirty, lock_reason, last_scanned_at, created_at, updated_at)
VALUES ('worktree-1', 'repo-1', 'branch-1', 'C:/Repos/Codex Orchestrator', 0, 0, NULL, NULL, ?1, ?1)
",
            params![CREATED_AT],
        )
        .expect("seed worktree");
    }

    fn insert_task(
        conn: &Connection,
        id: &str,
        execution_state: &str,
        attention_state: &str,
        repo_id: Option<&str>,
        branch_id: Option<&str>,
        worktree_id: Option<&str>,
    ) {
        conn.execute(
            "
INSERT INTO tasks (
  id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, due_at, snoozed_until, created_at, updated_at
) VALUES (
  @id, 'project-1', @repo_id, @branch_id, @worktree_id, 'Task', 'Task summary',
  @execution_state, @attention_state, 'normal', NULL, NULL, @created_at, @created_at
)
",
            named_params! {
                "@id": id,
                "@repo_id": repo_id,
                "@branch_id": branch_id,
                "@worktree_id": worktree_id,
                "@execution_state": execution_state,
                "@attention_state": attention_state,
                "@created_at": CREATED_AT,
            },
        )
        .expect("insert task");
    }

    fn insert_conversation_link(
        conn: &Connection,
        task_id: &str,
        conversation_id: &str,
        position: i64,
    ) {
        conn.execute(
            "
INSERT INTO task_conversation_links (task_id, conversation_id, position, created_at)
VALUES (?1, ?2, ?3, ?4)
",
            params![task_id, conversation_id, position, CREATED_AT],
        )
        .expect("insert conversation link");
    }

    fn insert_task_run(
        conn: &Connection,
        id: &str,
        task_id: &str,
        execution_state: &str,
        started_at: Option<&str>,
        completed_at: Option<&str>,
        created_at: &str,
    ) {
        conn.execute(
            "
INSERT INTO task_runs (
  id, task_id, conversation_id, worktree_id, execution_state, started_at, completed_at,
  exit_code, created_at, updated_at
) VALUES (?1, ?2, NULL, 'worktree-1', ?3, ?4, ?5, 0, ?6, ?6)
",
            params![
                id,
                task_id,
                execution_state,
                started_at,
                completed_at,
                created_at
            ],
        )
        .expect("insert task run");
    }

    fn insert_artifact(
        conn: &Connection,
        id: &str,
        task_id: &str,
        task_run_id: Option<&str>,
        kind: &str,
        title: &str,
        content: Option<&str>,
        created_at: &str,
    ) {
        conn.execute(
            "
INSERT INTO artifacts (
  id, task_id, task_run_id, conversation_id, kind, title, uri, content, created_at
) VALUES (?1, ?2, ?3, NULL, ?4, ?5, NULL, ?6, ?7)
",
            params![id, task_id, task_run_id, kind, title, content, created_at],
        )
        .expect("insert artifact");
    }

    fn insert_validation_run(
        conn: &Connection,
        id: &str,
        task_id: &str,
        task_run_id: Option<&str>,
        command: &str,
        status: &str,
        output_artifact_id: Option<&str>,
        exit_code: Option<i64>,
        started_at: &str,
        completed_at: Option<&str>,
    ) {
        conn.execute(
            "
INSERT INTO validation_runs (
  id, task_id, task_run_id, command, status, started_at, completed_at, exit_code,
  output_artifact_id, created_at, updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?8, ?9, ?7, ?6, ?8, ?8)
",
            params![
                id,
                task_id,
                task_run_id,
                command,
                status,
                output_artifact_id,
                exit_code,
                started_at,
                completed_at
            ],
        )
        .expect("insert validation run");
    }

    fn insert_event(
        conn: &Connection,
        id: &str,
        kind: &str,
        task_id: &str,
        task_run_id: Option<&str>,
        occurred_at: &str,
        payload: Value,
    ) {
        conn.execute(
            "
INSERT INTO events (
  id, kind, occurred_at, project_id, task_id, task_run_id, conversation_id, artifact_id,
  validation_run_id, payload_json
) VALUES (?1, ?2, ?3, 'project-1', ?4, ?5, NULL, NULL, NULL, ?6)
",
            params![
                id,
                kind,
                occurred_at,
                task_id,
                task_run_id,
                payload.to_string()
            ],
        )
        .expect("insert event");
    }
}
