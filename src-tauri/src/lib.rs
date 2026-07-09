use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

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
struct StartAgentSessionCommandInput {
    stream_id: Option<String>,
    session_id: Option<String>,
    prompt: String,
    cwd: Option<String>,
    additional_args: Option<Vec<String>>,
    env: Option<BTreeMap<String, Option<String>>>,
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

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct UploadedOrchestrationDraftFileInput {
    id: String,
    name: String,
    size: i64,
    last_modified: Option<i64>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct CreateOrchestrationDraftCommandInput {
    title: String,
    folder_path: String,
    prompt: String,
    files: Vec<UploadedOrchestrationDraftFileInput>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AddOrchestrationDraftNoteCommandInput {
    build_package_id: String,
    body: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AttachOrchestrationDraftFilesCommandInput {
    build_package_id: String,
    files: Vec<UploadedOrchestrationDraftFileInput>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RequestOrchestrationBuildStageCommandInput {
    build_package_id: String,
    stage_id: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartOrchestrationPlanBuilderRunCommandInput {
    build_package_id: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartOrchestrationCommandInput {
    build_package_id: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct OrchestrationStageRunEvidenceRecord {
    id: String,
    build_package_id: String,
    stage_id: String,
    status: String,
    provenance: String,
    status_reason: Option<String>,
    prompt_artifact_id: Option<String>,
    output_artifact_id: Option<String>,
    raw_event_artifact_id: Option<String>,
    task_id: Option<String>,
    task_run_id: Option<String>,
    conversation_id: Option<String>,
    event_ids: Vec<String>,
    evidence: Value,
    started_at: Option<String>,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
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

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartAgentSessionCommandResult {
    session_id: String,
    status: String,
    command: String,
    args: Vec<String>,
    stdout: String,
    stderr: String,
    output_was_streamed: bool,
    started_at: String,
    completed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StartAgentSessionStartedCommandResult {
    session_id: String,
    stream_id: String,
    status: String,
    command: String,
    args: Vec<String>,
    started_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AgentSessionCliOutputEvent {
    stream_id: String,
    stream: String,
    content: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AgentSessionCliCompletedEvent {
    stream_id: String,
    result: StartAgentSessionCommandResult,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct CodexRuntimeInfoCommandResult {
    doctor_stdout: String,
    model_catalog_stdout: String,
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
    with_app_database(&app, |conn| load_dashboard_snapshot(conn))
}

#[tauri::command]
fn register_task_worktree(
    app: AppHandle,
    input: RegisterTaskWorktreeCommandInput,
) -> Result<TaskDashboardSnapshot, String> {
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
    with_app_database(&app, |conn| {
        register_task_repo_anchor(conn, input)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
fn discover_task_repos(
    input: DiscoverTaskReposCommandInput,
) -> Result<Vec<DiscoveredTaskRepo>, String> {
    discover_git_repos(input)
}

#[tauri::command]
fn create_open_task(
    app: AppHandle,
    input: CreateOpenTaskCommandInput,
) -> Result<TaskDashboardSnapshot, String> {
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
    with_app_database(&app, |conn| {
        update_task(conn, &task_id, input)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
fn archive_open_task(app: AppHandle, task_id: String) -> Result<TaskDashboardSnapshot, String> {
    with_app_database(&app, |conn| {
        archive_task(conn, &task_id)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
fn load_task_run_detail(app: AppHandle, task_id: String) -> Result<TaskRunDetailSnapshot, String> {
    with_app_database(&app, |conn| load_task_run_detail_snapshot(conn, &task_id))
}

#[tauri::command]
fn start_codex_task_run(
    app: AppHandle,
    input: StartCodexTaskRunCommandInput,
) -> Result<StartCodexTaskRunCommandResult, String> {
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

#[tauri::command]
fn start_agent_session(
    app: AppHandle,
    input: StartAgentSessionCommandInput,
) -> Result<StartAgentSessionStartedCommandResult, String> {
    start_agent_session_streaming(&app, input)
}

#[tauri::command]
fn load_agent_session(
    app: AppHandle,
    session_id: String,
) -> Result<Option<StartAgentSessionCommandResult>, String> {
    validate_non_empty("sessionId", &session_id)?;
    with_app_database(&app, |conn| load_agent_session_record(conn, &session_id))
}

#[tauri::command]
fn load_codex_runtime_info() -> Result<CodexRuntimeInfoCommandResult, String> {
    Ok(CodexRuntimeInfoCommandResult {
        doctor_stdout: run_codex_runtime_info_command(&["doctor", "--json"])?,
        model_catalog_stdout: run_codex_runtime_info_command(&["debug", "models", "--bundled"])?,
    })
}

#[tauri::command]
fn select_orchestration_directory(default_path: String) -> Result<Option<String>, String> {
    let default_path = PathBuf::from(default_path);
    let mut dialog = rfd::FileDialog::new().set_title("Choose orchestration folder");

    if default_path.exists() {
        dialog = dialog.set_directory(default_path);
    }

    Ok(dialog
        .pick_folder()
        .map(|path| path.to_string_lossy().into_owned()))
}

#[tauri::command]
fn load_orchestration_registry(app: AppHandle) -> Result<Value, String> {
    with_app_database(&app, |conn| load_orchestration_registry_snapshot(conn))
}

#[tauri::command]
fn create_orchestration_draft(
    app: AppHandle,
    input: CreateOrchestrationDraftCommandInput,
) -> Result<Value, String> {
    with_app_database(&app, |conn| create_orchestration_draft_record(conn, input))
}

#[tauri::command]
fn add_orchestration_draft_note(
    app: AppHandle,
    input: AddOrchestrationDraftNoteCommandInput,
) -> Result<Value, String> {
    with_app_database(&app, |conn| {
        add_orchestration_draft_note_record(conn, input)
    })
}

#[tauri::command]
fn attach_orchestration_draft_files(
    app: AppHandle,
    input: AttachOrchestrationDraftFilesCommandInput,
) -> Result<Value, String> {
    with_app_database(&app, |conn| {
        attach_orchestration_draft_files_record(conn, input)
    })
}

#[tauri::command]
fn request_orchestration_build_stage(
    app: AppHandle,
    input: RequestOrchestrationBuildStageCommandInput,
) -> Result<Value, String> {
    with_app_database(&app, |conn| {
        request_orchestration_build_stage_record(conn, input)
    })
}

#[tauri::command]
fn start_orchestration_plan_builder_run(
    app: AppHandle,
    input: StartOrchestrationPlanBuilderRunCommandInput,
) -> Result<Value, String> {
    with_app_database(&app, |conn| {
        start_orchestration_plan_builder_run_with_runner(conn, input, &SystemCodexCommandRunner)
    })
}

#[tauri::command]
fn start_orchestration(
    app: AppHandle,
    input: StartOrchestrationCommandInput,
) -> Result<Value, String> {
    with_app_database(&app, |conn| start_orchestration_record(conn, input))
}

#[tauri::command]
fn load_orchestration(app: AppHandle, id: String) -> Result<Option<Value>, String> {
    with_app_database(&app, |conn| load_orchestration_snapshot(conn, &id))
}

#[tauri::command]
fn cancel_orchestration_draft(app: AppHandle, build_package_id: String) -> Result<Value, String> {
    with_app_database(&app, |conn| {
        cancel_orchestration_draft_record(conn, &build_package_id)
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_metadata,
            load_open_task_dashboard,
            register_task_worktree,
            register_task_repo,
            discover_task_repos,
            create_open_task,
            update_open_task,
            archive_open_task,
            load_task_run_detail,
            start_codex_task_run,
            start_agent_session,
            load_agent_session,
            load_codex_runtime_info,
            select_orchestration_directory,
            load_orchestration_registry,
            create_orchestration_draft,
            add_orchestration_draft_note,
            attach_orchestration_draft_files,
            request_orchestration_build_stage,
            start_orchestration_plan_builder_run,
            start_orchestration,
            load_orchestration,
            cancel_orchestration_draft
        ])
        .run(tauri::generate_context!())
        .expect("error while running Codex Orchestrator");
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

    for (position, migration) in app_migrations().iter().enumerate() {
        let already_applied = conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE id = ?1",
                params![migration.id],
                |_| Ok(()),
            )
            .optional()
            .map_err(sql_error("read schema migration state"))?
            .is_some();

        if already_applied {
            continue;
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(sql_error("begin schema migration"))?;
        tx.execute_batch(migration.sql)
            .map_err(|error| format!("Unable to apply migration {}: {error}", migration.id))?;
        tx.execute(
            "INSERT INTO schema_migrations (id, applied_at, position) VALUES (?1, ?2, ?3)",
            params![migration.id, now_iso(), position as i64],
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

fn start_agent_session_with_runner(
    input: StartAgentSessionCommandInput,
    runner: &impl CodexCommandRunner,
) -> Result<StartAgentSessionCommandResult, String> {
    validate_start_agent_session_input(&input)?;

    let session_id = input
        .session_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let started_at = now_iso();
    let command_input = CodexCommandRunInput {
        command: "codex".to_string(),
        args: build_agent_session_args(&input),
        cwd: input.cwd.clone(),
        env: input.env.clone(),
    };
    let command = command_input.command.clone();
    let args = command_input.args.clone();

    match runner.run(command_input) {
        Ok(process_result) => {
            let exit_code = process_result.exit_code;
            let signal = process_result.signal.clone();
            let failed = exit_code != Some(0) || signal.is_some();
            let error = if failed {
                Some(format!(
                    "Codex session failed {}",
                    process_failure_reason(exit_code, signal.as_deref())
                ))
            } else {
                None
            };

            Ok(StartAgentSessionCommandResult {
                session_id,
                status: if failed { "failed" } else { "completed" }.to_string(),
                command,
                args,
                stdout: process_result.stdout,
                stderr: process_result.stderr,
                output_was_streamed: false,
                started_at,
                completed_at: now_iso(),
                exit_code,
                signal,
                error,
            })
        }
        Err(error) => Ok(StartAgentSessionCommandResult {
            session_id,
            status: "failed".to_string(),
            command,
            args,
            stdout: String::new(),
            stderr: String::new(),
            output_was_streamed: false,
            started_at,
            completed_at: now_iso(),
            exit_code: None,
            signal: None,
            error: Some(error),
        }),
    }
}

fn start_agent_session_streaming(
    app: &AppHandle,
    input: StartAgentSessionCommandInput,
) -> Result<StartAgentSessionStartedCommandResult, String> {
    validate_start_agent_session_input(&input)?;

    let session_id = input
        .session_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let stream_id = input
        .stream_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let started_at = now_iso();
    let command_name = "codex".to_string();
    let args = build_agent_session_args(&input);
    let app = app.clone();
    let background_input = input.clone();
    let background_session_id = session_id.clone();
    let background_stream_id = stream_id.clone();
    let background_started_at = started_at.clone();
    let background_command_name = command_name.clone();
    let background_args = args.clone();

    thread::spawn(move || {
        let mut result = run_agent_session_process(
            &app,
            background_input,
            background_session_id.clone(),
            background_stream_id.clone(),
            background_started_at,
            background_command_name,
            background_args,
        );

        if let Err(error) = with_app_database(&app, |conn| {
            persist_agent_session_run(conn, &background_session_id, &background_stream_id, &result)
        }) {
            result.status = "failed".to_string();
            result.error = Some(error);
        }

        let _ = app.emit(
            "agent-session-cli-completed",
            AgentSessionCliCompletedEvent {
                stream_id: background_stream_id,
                result,
            },
        );
    });

    Ok(StartAgentSessionStartedCommandResult {
        session_id,
        stream_id,
        status: "running".to_string(),
        command: command_name,
        args,
        started_at,
    })
}

fn run_agent_session_process(
    app: &AppHandle,
    input: StartAgentSessionCommandInput,
    session_id: String,
    stream_id: String,
    started_at: String,
    command_name: String,
    args: Vec<String>,
) -> StartAgentSessionCommandResult {
    let mut command = Command::new(&command_name);
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

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

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return failed_agent_session_result(
                session_id,
                command_name,
                args,
                started_at,
                format!("Unable to launch Codex: {error}"),
            );
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            return failed_agent_session_result(
                session_id,
                command_name,
                args,
                started_at,
                "Unable to capture Codex stdout.".to_string(),
            );
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            return failed_agent_session_result(
                session_id,
                command_name,
                args,
                started_at,
                "Unable to capture Codex stderr.".to_string(),
            );
        }
    };
    let stdout_accumulator = Arc::new(Mutex::new(String::new()));
    let stderr_accumulator = Arc::new(Mutex::new(String::new()));
    let stdout_handle = spawn_agent_session_stream_reader(
        app.clone(),
        stream_id.clone(),
        "stdout",
        stdout,
        stdout_accumulator.clone(),
    );
    let stderr_handle = spawn_agent_session_stream_reader(
        app.clone(),
        stream_id.clone(),
        "stderr",
        stderr,
        stderr_accumulator.clone(),
    );
    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            return failed_agent_session_result(
                session_id,
                command_name,
                args,
                started_at,
                format!("Unable to wait for Codex: {error}"),
            );
        }
    };

    if stdout_handle.join().is_err() {
        return failed_agent_session_result(
            session_id,
            command_name,
            args,
            started_at,
            "Codex stdout reader panicked.".to_string(),
        );
    }

    if stderr_handle.join().is_err() {
        return failed_agent_session_result(
            session_id,
            command_name,
            args,
            started_at,
            "Codex stderr reader panicked.".to_string(),
        );
    }

    let completed_at = now_iso();
    let stdout = match take_accumulated_output(&stdout_accumulator) {
        Ok(stdout) => stdout,
        Err(error) => {
            return failed_agent_session_result(session_id, command_name, args, started_at, error);
        }
    };
    let stderr = match take_accumulated_output(&stderr_accumulator) {
        Ok(stderr) => stderr,
        Err(error) => {
            return failed_agent_session_result(session_id, command_name, args, started_at, error);
        }
    };
    let exit_code = status.code().map(i64::from);
    let signal = process_exit_signal(&status);
    let failed = exit_code != Some(0) || signal.is_some();
    let error = if failed {
        Some(format!(
            "Codex session failed {}",
            process_failure_reason(exit_code, signal.as_deref())
        ))
    } else {
        None
    };

    StartAgentSessionCommandResult {
        session_id,
        status: if failed { "failed" } else { "completed" }.to_string(),
        command: command_name,
        args,
        stdout,
        stderr,
        output_was_streamed: true,
        started_at,
        completed_at,
        exit_code,
        signal,
        error,
    }
}

fn failed_agent_session_result(
    session_id: String,
    command: String,
    args: Vec<String>,
    started_at: String,
    error: String,
) -> StartAgentSessionCommandResult {
    StartAgentSessionCommandResult {
        session_id,
        status: "failed".to_string(),
        command,
        args,
        stdout: String::new(),
        stderr: String::new(),
        output_was_streamed: true,
        started_at,
        completed_at: now_iso(),
        exit_code: None,
        signal: None,
        error: Some(error),
    }
}

fn run_codex_runtime_info_command(args: &[&str]) -> Result<String, String> {
    let output = Command::new("codex")
        .args(args)
        .output()
        .map_err(|error| format!("Unable to launch codex {}: {error}", args.join(" ")))?;

    if !output.status.success() {
        return Err(format!(
            "codex {} failed {}: {}",
            args.join(" "),
            process_failure_reason(
                output.status.code().map(i64::from),
                process_exit_signal(&output.status).as_deref()
            ),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn spawn_agent_session_stream_reader<R>(
    app: AppHandle,
    stream_id: String,
    stream: &'static str,
    reader: R,
    accumulator: Arc<Mutex<String>>,
) -> thread::JoinHandle<()>
where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let reader = BufReader::new(reader);

        for line in reader.lines().map_while(Result::ok) {
            if let Ok(mut output) = accumulator.lock() {
                output.push_str(&line);
                output.push('\n');
            }

            let _ = app.emit(
                "agent-session-cli-output",
                AgentSessionCliOutputEvent {
                    stream_id: stream_id.clone(),
                    stream: stream.to_string(),
                    content: line,
                },
            );
        }
    })
}

fn take_accumulated_output(output: &Arc<Mutex<String>>) -> Result<String, String> {
    output
        .lock()
        .map(|output| output.clone())
        .map_err(|_| "Unable to read captured Codex output.".to_string())
}

fn persist_agent_session_run(
    conn: &Connection,
    session_id: &str,
    stream_id: &str,
    result: &StartAgentSessionCommandResult,
) -> Result<(), String> {
    let timestamp = now_iso();
    let args_json = serde_json::to_string(&result.args)
        .map_err(|error| format!("Unable to serialize agent session args: {error}"))?;
    let codex_session_id = extract_codex_thread_id(&result.stdout);

    conn.execute(
        "
INSERT INTO agent_sessions (
  id, codex_session_id, status, command, args_json, cwd, started_at, completed_at, exit_code,
  error, created_at, updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9, ?10, ?10)
ON CONFLICT(id) DO UPDATE SET
  codex_session_id = excluded.codex_session_id,
  status = excluded.status,
  command = excluded.command,
  args_json = excluded.args_json,
  started_at = excluded.started_at,
  completed_at = excluded.completed_at,
  exit_code = excluded.exit_code,
  error = excluded.error,
  updated_at = excluded.updated_at
",
        params![
            session_id,
            codex_session_id,
            result.status,
            result.command,
            args_json,
            result.started_at,
            result.completed_at,
            result.exit_code,
            result.error,
            timestamp
        ],
    )
    .map_err(sql_error("persist agent session"))?;

    conn.execute(
        "
INSERT INTO agent_session_cli_logs (
  id, agent_session_id, stream_id, stdout, stderr, created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
",
        params![
            Uuid::new_v4().to_string(),
            session_id,
            stream_id,
            result.stdout,
            result.stderr,
            timestamp
        ],
    )
    .map_err(sql_error("persist agent session CLI log"))?;

    Ok(())
}

fn load_agent_session_record(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<StartAgentSessionCommandResult>, String> {
    conn.query_row(
        "
SELECT
  s.id,
  s.status,
  s.command,
  s.args_json,
  s.started_at,
  s.completed_at,
  s.exit_code,
  s.error,
  COALESCE(l.stdout, ''),
  COALESCE(l.stderr, '')
FROM agent_sessions s
LEFT JOIN agent_session_cli_logs l
  ON l.id = (
    SELECT id
    FROM agent_session_cli_logs
    WHERE agent_session_id = s.id
    ORDER BY created_at DESC
    LIMIT 1
  )
WHERE s.id = ?1
",
        params![session_id],
        |row| {
            let args_json: String = row.get(3)?;
            let args = serde_json::from_str::<Vec<String>>(&args_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            let completed_at = row
                .get::<_, Option<String>>(5)?
                .unwrap_or_else(|| row.get::<_, String>(4).unwrap_or_else(|_| now_iso()));

            Ok(StartAgentSessionCommandResult {
                session_id: row.get(0)?,
                status: row.get(1)?,
                command: row.get(2)?,
                args,
                stdout: row.get(8)?,
                stderr: row.get(9)?,
                output_was_streamed: false,
                started_at: row.get(4)?,
                completed_at,
                exit_code: row.get(6)?,
                signal: None,
                error: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(sql_error("load agent session"))
}

fn extract_codex_thread_id(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let value = serde_json::from_str::<Value>(line).ok()?;
        if value.get("type")?.as_str()? == "thread.started" {
            value
                .get("thread_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        } else {
            None
        }
    })
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

fn validate_start_agent_session_input(input: &StartAgentSessionCommandInput) -> Result<(), String> {
    validate_non_empty("prompt", &input.prompt)?;

    if let Some(session_id) = &input.session_id {
        validate_non_empty("sessionId", session_id)?;
    }

    if let Some(cwd) = &input.cwd {
        validate_non_empty("cwd", cwd)?;
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

fn build_agent_session_args(input: &StartAgentSessionCommandInput) -> Vec<String> {
    let mut args = vec!["exec".to_string(), "--json".to_string()];

    if let Some(additional_args) = &input.additional_args {
        args.extend(additional_args.iter().cloned());
    }

    if let Some(session_id) = &input.session_id {
        args.push("resume".to_string());
        args.push(session_id.clone());
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

    let mut dashboard_repos = repos
        .iter()
        .filter_map(|repo| {
            let project = projects
                .iter()
                .find(|project| project.id == repo.project_id)?;

            Some(TaskDashboardRepo {
                id: repo.id.clone(),
                project_id: project.id.clone(),
                project: project.name.clone(),
                name: repo.name.clone(),
                root_path: repo.root_path.clone(),
            })
        })
        .collect::<Vec<_>>();
    dashboard_repos.sort_by(|left, right| {
        format!("{}\0{}\0{}", left.project, left.name, left.root_path).cmp(&format!(
            "{}\0{}\0{}",
            right.project, right.name, right.root_path
        ))
    });

    let mut dashboard_projects = projects
        .into_iter()
        .map(|project| TaskDashboardProject {
            id: project.id,
            name: project.name,
        })
        .collect::<Vec<_>>();
    dashboard_projects.sort_by(|left, right| left.name.cmp(&right.name));
    let total_open_tasks = groups.iter().map(|group| group.tasks.len()).sum();

    Ok(TaskDashboardSnapshot {
        groups,
        projects: dashboard_projects,
        repos: dashboard_repos,
        worktree_anchors,
        total_open_tasks,
    })
}

fn load_orchestration_registry_snapshot(conn: &Connection) -> Result<Value, String> {
    let build_packages = select_active_orchestration_draft_snapshots(conn)?;

    Ok(json!({
        "orchestrations": [],
        "buildPackages": build_packages,
        "clientState": orchestration_registry_client_state(),
    }))
}

fn load_orchestration_snapshot(
    _conn: &Connection,
    orchestration_id: &str,
) -> Result<Option<Value>, String> {
    validate_non_empty("orchestrationId", orchestration_id)?;

    Err(
        "Live orchestration snapshot loading is integration-pending: persisted drafts are available through load_orchestration_registry, but this backend does not persist or fabricate live orchestration snapshots yet."
            .to_string(),
    )
}

fn create_orchestration_draft_record(
    conn: &Connection,
    input: CreateOrchestrationDraftCommandInput,
) -> Result<Value, String> {
    validate_non_empty("title", &input.title)?;
    validate_non_empty("folderPath", &input.folder_path)?;
    validate_non_empty("prompt", &input.prompt)?;

    for file in &input.files {
        validate_non_empty("file.id", &file.id)?;
        validate_non_empty("file.name", &file.name)?;

        if file.size < 0 {
            return Err(format!(
                "Invalid file size for {}: {}",
                file.name, file.size
            ));
        }
    }

    let build_package_id = Uuid::new_v4().to_string();
    let created_at = now_iso();
    let snapshot = build_persisted_orchestration_draft_snapshot(
        &build_package_id,
        &created_at,
        &input.title,
        &input.folder_path,
        &input.prompt,
        &input.files,
    );

    insert_orchestration_draft_snapshot(
        conn,
        &build_package_id,
        &input.title,
        &input.folder_path,
        &input.prompt,
        &snapshot,
        &created_at,
    )?;

    Ok(snapshot)
}

fn add_orchestration_draft_note_record(
    conn: &Connection,
    input: AddOrchestrationDraftNoteCommandInput,
) -> Result<Value, String> {
    validate_non_empty("buildPackageId", &input.build_package_id)?;
    validate_non_empty("body", &input.body)?;

    let mut snapshot = select_orchestration_draft_snapshot(conn, &input.build_package_id)?;
    let updated_at = now_iso();
    let continuation_notice = unsupported_orchestration_continuation_notice();
    let messages = snapshot_array_mut(&mut snapshot, "messages")?;
    messages.push(json!({
        "id": format!("message-{}", Uuid::new_v4()),
        "role": "user",
        "body": input.body,
        "createdAt": updated_at,
        "state": "completed",
        "truth": persisted_draft_truth_state(),
    }));
    messages.push(json!({
        "id": format!("message-{}", Uuid::new_v4()),
        "role": "system",
        "body": continuation_notice["message"].as_str().unwrap_or("Runtime continuation is unsupported."),
        "createdAt": updated_at,
        "state": "completed",
        "truth": unsupported_pending_truth_state(),
    }));
    set_orchestration_snapshot_updated_state(
        &mut snapshot,
        &input.build_package_id,
        &updated_at,
        vec![continuation_notice],
    )?;
    update_orchestration_draft_snapshot(conn, &input.build_package_id, &snapshot, &updated_at)?;

    Ok(snapshot)
}

fn attach_orchestration_draft_files_record(
    conn: &Connection,
    input: AttachOrchestrationDraftFilesCommandInput,
) -> Result<Value, String> {
    validate_non_empty("buildPackageId", &input.build_package_id)?;

    for file in &input.files {
        validate_non_empty("file.id", &file.id)?;
        validate_non_empty("file.name", &file.name)?;

        if file.size < 0 {
            return Err(format!(
                "Invalid file size for {}: {}",
                file.name, file.size
            ));
        }
    }

    let mut snapshot = select_orchestration_draft_snapshot(conn, &input.build_package_id)?;
    let updated_at = now_iso();
    let files = snapshot_array_mut(&mut snapshot, "files")?;
    let mut existing_keys = files
        .iter()
        .filter_map(uploaded_orchestration_file_key)
        .collect::<HashSet<_>>();

    for file in input.files {
        let key = uploaded_orchestration_file_input_key(&file);

        if existing_keys.insert(key) {
            files.push(uploaded_orchestration_file_value(&file));
        }
    }

    set_orchestration_snapshot_updated_state(
        &mut snapshot,
        &input.build_package_id,
        &updated_at,
        vec![orchestration_registry_notice()],
    )?;
    update_orchestration_draft_snapshot(conn, &input.build_package_id, &snapshot, &updated_at)?;

    Ok(snapshot)
}

fn request_orchestration_build_stage_record(
    conn: &Connection,
    input: RequestOrchestrationBuildStageCommandInput,
) -> Result<Value, String> {
    validate_non_empty("buildPackageId", &input.build_package_id)?;
    let stage_title = orchestration_stage_title(&input.stage_id)?;
    let mut snapshot = select_orchestration_draft_snapshot(conn, &input.build_package_id)?;
    let updated_at = now_iso();
    let notice = missing_orchestration_runtime_notice(&input.stage_id, stage_title);

    if let Some(stages) = snapshot.get_mut("stages").and_then(Value::as_array_mut) {
        for stage in stages {
            let Some(stage_object) = stage.as_object_mut() else {
                continue;
            };
            let is_target = stage_object
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id == input.stage_id);

            if is_target {
                let current_detail = stage_object
                    .get("detail")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                stage_object.insert(
                    "state".to_string(),
                    notice
                        .get("truth")
                        .cloned()
                        .unwrap_or_else(unsupported_pending_truth_state),
                );
                stage_object.insert(
                    "detail".to_string(),
                    Value::String(format!(
                        "{current_detail} {}",
                        notice["message"].as_str().unwrap()
                    )),
                );
                if input.stage_id == "instantiator" {
                    stage_object.insert(
                        "summary".to_string(),
                        Value::String(
                            "Build plan approval was accepted; instantiator runtime is unsupported."
                                .to_string(),
                        ),
                    );
                }
            } else if input.stage_id == "instantiator"
                && stage_object
                    .get("id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == "plan-review")
            {
                stage_object.insert(
                    "state".to_string(),
                    json!({ "status": "completed", "provenance": "backend_response" }),
                );
                stage_object.insert(
                    "summary".to_string(),
                    Value::String("The user confirmed the Plan Builder proposal.".to_string()),
                );
                stage_object.insert(
                    "detail".to_string(),
                    Value::String(
                        "Approval was accepted before attempting instantiation. No instantiator runtime route has started."
                            .to_string(),
                    ),
                );
            }
        }
    }

    snapshot_array_mut(&mut snapshot, "messages")?.push(json!({
        "id": format!("message-{}", Uuid::new_v4()),
        "role": "system",
        "body": notice["message"].as_str().unwrap_or("Runtime integration is pending."),
        "createdAt": updated_at,
        "state": "completed",
        "truth": unsupported_pending_truth_state(),
    }));
    set_orchestration_snapshot_updated_state(
        &mut snapshot,
        &input.build_package_id,
        &updated_at,
        vec![notice],
    )?;
    update_orchestration_draft_snapshot(conn, &input.build_package_id, &snapshot, &updated_at)?;

    Ok(snapshot)
}

fn start_orchestration_plan_builder_run_with_runner(
    conn: &Connection,
    input: StartOrchestrationPlanBuilderRunCommandInput,
    codex_runner: &impl CodexCommandRunner,
) -> Result<Value, String> {
    validate_non_empty("buildPackageId", &input.build_package_id)?;

    let snapshot = select_orchestration_draft_snapshot(conn, &input.build_package_id)?;
    let started_at = now_iso();
    let stage_run_id = Uuid::new_v4().to_string();
    let conversation_id =
        create_orchestration_plan_builder_conversation(conn, &snapshot, &started_at)?;
    let prompt = build_orchestration_plan_builder_prompt(&snapshot)?;
    let prompt_artifact_id = create_artifact(
        conn,
        None,
        None,
        Some(&conversation_id),
        "handoff",
        "Submitted Plan Builder prompt",
        Some(&prompt),
    )?;
    let run_started_event_id = append_orchestration_plan_builder_started_event(
        conn,
        &input.build_package_id,
        &stage_run_id,
        &conversation_id,
        &started_at,
    )?;
    let prompt_event_id = append_orchestration_artifact_created_event(
        conn,
        &input.build_package_id,
        &stage_run_id,
        &conversation_id,
        &prompt_artifact_id,
        "handoff",
        "submitted_prompt",
        None,
    )?;
    let mut event_ids = vec![run_started_event_id, prompt_event_id];

    insert_orchestration_stage_run_evidence_record(
        conn,
        &OrchestrationStageRunEvidenceRecord {
            id: stage_run_id.clone(),
            build_package_id: input.build_package_id.clone(),
            stage_id: "plan-builder".to_string(),
            status: "waiting_for_event".to_string(),
            provenance: "backend_response".to_string(),
            status_reason: Some(
                "Backend accepted the Plan Builder run request; waiting for final Codex output."
                    .to_string(),
            ),
            prompt_artifact_id: Some(prompt_artifact_id.clone()),
            output_artifact_id: None,
            raw_event_artifact_id: None,
            task_id: None,
            task_run_id: None,
            conversation_id: Some(conversation_id.clone()),
            event_ids: event_ids.clone(),
            evidence: orchestration_plan_builder_evidence_payload(
                &snapshot,
                None,
                None,
                "codex exec --json",
            )?,
            started_at: Some(started_at.clone()),
            completed_at: None,
            created_at: started_at.clone(),
            updated_at: started_at.clone(),
        },
    )?;
    let mut ack_snapshot = select_orchestration_draft_snapshot(conn, &input.build_package_id)?;
    apply_orchestration_plan_builder_runtime_state(
        &mut ack_snapshot,
        &input.build_package_id,
        "waiting_for_event",
        "backend_response",
        "Backend accepted the Plan Builder runtime request; waiting for the final response.",
        "Backend accepted the Plan Builder request.",
        "The prompt was submitted through the orchestration Plan Builder runtime route. Live streaming is not available in this increment.",
        vec![orchestration_plan_builder_waiting_notice()],
    )?;
    update_orchestration_draft_snapshot(conn, &input.build_package_id, &ack_snapshot, &started_at)?;

    let process_result = match codex_runner.run(CodexCommandRunInput {
        command: "codex".to_string(),
        args: vec!["exec".to_string(), "--json".to_string(), prompt],
        cwd: None,
        env: None,
    }) {
        Ok(process_result) => process_result,
        Err(error) => {
            let completed_at = now_iso();
            let completed_event_id = append_orchestration_plan_builder_completed_event(
                conn,
                &input.build_package_id,
                &stage_run_id,
                &conversation_id,
                "failed",
                Some(&error),
                None,
            )?;
            event_ids.push(completed_event_id);
            update_orchestration_stage_run_evidence_record(
                conn,
                &OrchestrationStageRunEvidenceRecord {
                    id: stage_run_id,
                    build_package_id: input.build_package_id.clone(),
                    stage_id: "plan-builder".to_string(),
                    status: "failed".to_string(),
                    provenance: "backend_response".to_string(),
                    status_reason: Some(error.clone()),
                    prompt_artifact_id: Some(prompt_artifact_id),
                    output_artifact_id: None,
                    raw_event_artifact_id: None,
                    task_id: None,
                    task_run_id: None,
                    conversation_id: Some(conversation_id),
                    event_ids,
                    evidence: orchestration_plan_builder_evidence_payload(
                        &snapshot,
                        None,
                        Some(&error),
                        "codex exec --json",
                    )?,
                    started_at: Some(started_at),
                    completed_at: Some(completed_at.clone()),
                    created_at: completed_at.clone(),
                    updated_at: completed_at.clone(),
                },
            )?;
            let mut failed_snapshot =
                select_orchestration_draft_snapshot(conn, &input.build_package_id)?;
            apply_orchestration_plan_builder_terminal_message(
                &mut failed_snapshot,
                &completed_at,
                "system",
                &format!("Plan Builder runtime failed to launch. {error}"),
                "failed",
                "backend_response",
            )?;
            apply_orchestration_plan_builder_runtime_state(
                &mut failed_snapshot,
                &input.build_package_id,
                "failed",
                "backend_response",
                &format!("Plan Builder runtime failed to launch. {error}"),
                "Plan-builder runtime start failed.",
                "The draft is preserved. The backend route was called, but Codex did not launch successfully.",
                vec![orchestration_plan_builder_failed_notice(&error)],
            )?;
            update_orchestration_draft_snapshot(
                conn,
                &input.build_package_id,
                &failed_snapshot,
                &completed_at,
            )?;
            return Ok(failed_snapshot);
        }
    };

    finish_orchestration_plan_builder_run_from_process_result(
        conn,
        &input.build_package_id,
        &stage_run_id,
        &conversation_id,
        &prompt_artifact_id,
        &started_at,
        event_ids,
        snapshot,
        process_result,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_orchestration_plan_builder_run_from_process_result(
    conn: &Connection,
    build_package_id: &str,
    stage_run_id: &str,
    conversation_id: &str,
    prompt_artifact_id: &str,
    started_at: &str,
    mut event_ids: Vec<String>,
    original_snapshot: Value,
    process_result: CodexCommandRunResult,
) -> Result<Value, String> {
    let raw_event_artifact_id = create_artifact(
        conn,
        None,
        None,
        Some(conversation_id),
        "raw_event_stream",
        "Raw Codex JSONL",
        Some(&process_result.stdout),
    )?;
    let raw_event_id = append_orchestration_artifact_created_event(
        conn,
        build_package_id,
        stage_run_id,
        conversation_id,
        &raw_event_artifact_id,
        "raw_event_stream",
        "raw_event_stream",
        Some(process_result.stdout.len() as i64),
    )?;
    event_ids.push(raw_event_id);

    let runtime_result = match codex_runtime_result_from_process_result(process_result) {
        Ok(runtime_result) => runtime_result,
        Err(error) => {
            return finish_failed_orchestration_plan_builder_run(
                conn,
                build_package_id,
                stage_run_id,
                conversation_id,
                prompt_artifact_id,
                Some(&raw_event_artifact_id),
                started_at,
                event_ids,
                original_snapshot,
                "Codex JSONL parse failed",
                &error,
            );
        }
    };

    update_orchestration_conversation_from_runtime_result(conn, conversation_id, &runtime_result)?;

    if runtime_result.status == CodexRuntimeStatus::Completed {
        let completed_at = now_iso();
        let output_artifact_id = match &runtime_result.summary.final_agent_message_text {
            Some(final_response) => Some(create_artifact(
                conn,
                None,
                None,
                Some(conversation_id),
                "final_response",
                "Final Plan Builder response",
                Some(final_response),
            )?),
            None => None,
        };

        if let Some(output_artifact_id) = &output_artifact_id {
            event_ids.push(append_orchestration_artifact_created_event(
                conn,
                build_package_id,
                stage_run_id,
                conversation_id,
                output_artifact_id,
                "final_response",
                "final_response",
                runtime_result
                    .summary
                    .final_agent_message_text
                    .as_ref()
                    .map(|value| value.len() as i64),
            )?);
        }

        event_ids.push(append_orchestration_plan_builder_completed_event(
            conn,
            build_package_id,
            stage_run_id,
            conversation_id,
            "completed",
            None,
            runtime_result.exit_code,
        )?);
        update_orchestration_stage_run_evidence_record(
            conn,
            &OrchestrationStageRunEvidenceRecord {
                id: stage_run_id.to_string(),
                build_package_id: build_package_id.to_string(),
                stage_id: "plan-builder".to_string(),
                status: "completed".to_string(),
                provenance: "backend_response".to_string(),
                status_reason: Some(runtime_result.status_reason.clone()),
                prompt_artifact_id: Some(prompt_artifact_id.to_string()),
                output_artifact_id: output_artifact_id.clone(),
                raw_event_artifact_id: Some(raw_event_artifact_id),
                task_id: None,
                task_run_id: None,
                conversation_id: Some(conversation_id.to_string()),
                event_ids,
                evidence: orchestration_plan_builder_evidence_payload(
                    &original_snapshot,
                    runtime_result.summary.thread_id.as_deref(),
                    None,
                    "codex exec --json",
                )?,
                started_at: Some(started_at.to_string()),
                completed_at: Some(completed_at.clone()),
                created_at: started_at.to_string(),
                updated_at: completed_at.clone(),
            },
        )?;
        let mut completed_snapshot = select_orchestration_draft_snapshot(conn, build_package_id)?;
        if let Some(final_response) = runtime_result.summary.final_agent_message_text {
            apply_orchestration_plan_builder_terminal_message(
                &mut completed_snapshot,
                &completed_at,
                "assistant",
                &final_response,
                "completed",
                "backend_response",
            )?;
        } else {
            apply_orchestration_plan_builder_terminal_message(
                &mut completed_snapshot,
                &completed_at,
                "system",
                "Plan Builder completed, but Codex did not emit a final agent message.",
                "completed",
                "backend_response",
            )?;
        }
        apply_orchestration_plan_builder_runtime_state(
            &mut completed_snapshot,
            build_package_id,
            "completed",
            "backend_response",
            "Plan Builder output is ready for review. Confirm the build plan to request instantiation, or preserve feedback locally; runtime continuation is unsupported.",
            "Plan-builder output is available.",
            "The backend persisted the submitted prompt, raw Codex JSONL, final response artifact, and stage-run evidence. This is a proposal awaiting explicit user approval.",
            vec![orchestration_plan_builder_completed_notice()],
        )?;
        update_orchestration_draft_snapshot(
            conn,
            build_package_id,
            &completed_snapshot,
            &completed_at,
        )?;

        return Ok(completed_snapshot);
    }

    let error = codex_failure_reason(&runtime_result);
    finish_failed_orchestration_plan_builder_run(
        conn,
        build_package_id,
        stage_run_id,
        conversation_id,
        prompt_artifact_id,
        Some(&raw_event_artifact_id),
        started_at,
        event_ids,
        original_snapshot,
        &runtime_result.status_reason,
        &error,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_failed_orchestration_plan_builder_run(
    conn: &Connection,
    build_package_id: &str,
    stage_run_id: &str,
    conversation_id: &str,
    prompt_artifact_id: &str,
    raw_event_artifact_id: Option<&str>,
    started_at: &str,
    mut event_ids: Vec<String>,
    original_snapshot: Value,
    status_reason: &str,
    error: &str,
) -> Result<Value, String> {
    let completed_at = now_iso();
    event_ids.push(append_orchestration_plan_builder_completed_event(
        conn,
        build_package_id,
        stage_run_id,
        conversation_id,
        "failed",
        Some(error),
        None,
    )?);
    update_orchestration_stage_run_evidence_record(
        conn,
        &OrchestrationStageRunEvidenceRecord {
            id: stage_run_id.to_string(),
            build_package_id: build_package_id.to_string(),
            stage_id: "plan-builder".to_string(),
            status: "failed".to_string(),
            provenance: "backend_response".to_string(),
            status_reason: Some(status_reason.to_string()),
            prompt_artifact_id: Some(prompt_artifact_id.to_string()),
            output_artifact_id: None,
            raw_event_artifact_id: raw_event_artifact_id.map(str::to_string),
            task_id: None,
            task_run_id: None,
            conversation_id: Some(conversation_id.to_string()),
            event_ids,
            evidence: orchestration_plan_builder_evidence_payload(
                &original_snapshot,
                None,
                Some(error),
                "codex exec --json",
            )?,
            started_at: Some(started_at.to_string()),
            completed_at: Some(completed_at.clone()),
            created_at: started_at.to_string(),
            updated_at: completed_at.clone(),
        },
    )?;
    let mut failed_snapshot = select_orchestration_draft_snapshot(conn, build_package_id)?;
    apply_orchestration_plan_builder_terminal_message(
        &mut failed_snapshot,
        &completed_at,
        "system",
        &format!("Plan Builder runtime failed. {error}"),
        "failed",
        "backend_response",
    )?;
    apply_orchestration_plan_builder_runtime_state(
        &mut failed_snapshot,
        build_package_id,
        "failed",
        "backend_response",
        &format!("Plan Builder runtime failed. {error}"),
        "Plan-builder runtime failed.",
        "The draft is preserved. Raw Codex JSONL is linked when the process produced output.",
        vec![orchestration_plan_builder_failed_notice(error)],
    )?;
    update_orchestration_draft_snapshot(conn, build_package_id, &failed_snapshot, &completed_at)?;

    Ok(failed_snapshot)
}

fn start_orchestration_record(
    conn: &Connection,
    input: StartOrchestrationCommandInput,
) -> Result<Value, String> {
    validate_non_empty("buildPackageId", &input.build_package_id)?;

    let mut snapshot = select_orchestration_draft_snapshot(conn, &input.build_package_id)?;
    let updated_at = now_iso();
    let notice = live_orchestration_runtime_notice();
    snapshot_array_mut(&mut snapshot, "messages")?.push(json!({
        "id": format!("message-{}", Uuid::new_v4()),
        "role": "system",
        "body": notice["message"].as_str().unwrap_or("Live orchestration runtime is unavailable."),
        "createdAt": updated_at,
        "state": "completed",
        "truth": unsupported_pending_truth_state(),
    }));
    set_orchestration_snapshot_updated_state(
        &mut snapshot,
        &input.build_package_id,
        &updated_at,
        vec![notice],
    )?;
    update_orchestration_draft_snapshot(conn, &input.build_package_id, &snapshot, &updated_at)?;
    let client_state = snapshot.get("clientState").cloned().ok_or_else(|| {
        "Persisted orchestration draft snapshot is missing clientState.".to_string()
    })?;

    Ok(json!({
        "buildPackage": snapshot,
        "clientState": client_state,
    }))
}

fn cancel_orchestration_draft_record(
    conn: &Connection,
    build_package_id: &str,
) -> Result<Value, String> {
    validate_non_empty("buildPackageId", build_package_id)?;
    let changed = conn
        .execute(
            "UPDATE orchestration_drafts SET canceled_at = ?1, updated_at = ?1 WHERE id = ?2 AND canceled_at IS NULL",
            params![now_iso(), build_package_id],
        )
        .map_err(sql_error("cancel orchestration draft"))?;

    if changed == 0 {
        return Err(orchestration_draft_not_found(build_package_id));
    }

    load_orchestration_registry_snapshot(conn)
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

fn select_active_orchestration_draft_snapshots(conn: &Connection) -> Result<Vec<Value>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT snapshot_json
FROM orchestration_drafts
WHERE canceled_at IS NULL
ORDER BY updated_at DESC, created_at DESC, id
",
        )
        .map_err(sql_error("prepare active orchestration draft query"))?;

    let mut snapshots = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(sql_error("query active orchestration draft rows"))?
        .map(|row| {
            let snapshot_json = row.map_err(sql_error("read active orchestration draft row"))?;
            serde_json::from_str::<Value>(&snapshot_json)
                .map_err(|error| format!("Invalid persisted orchestration draft snapshot: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    for snapshot in &mut snapshots {
        attach_orchestration_stage_runs(conn, snapshot)?;
    }

    Ok(snapshots)
}

fn select_orchestration_draft_snapshot(
    conn: &Connection,
    build_package_id: &str,
) -> Result<Value, String> {
    let mut snapshot = select_orchestration_draft_snapshot_base(conn, build_package_id)?;
    attach_orchestration_stage_runs(conn, &mut snapshot)?;
    Ok(snapshot)
}

fn select_orchestration_draft_snapshot_base(
    conn: &Connection,
    build_package_id: &str,
) -> Result<Value, String> {
    let snapshot_json = conn
        .query_row(
            "
SELECT snapshot_json
FROM orchestration_drafts
WHERE id = ?1 AND canceled_at IS NULL
",
            params![build_package_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error("read orchestration draft snapshot"))?
        .ok_or_else(|| orchestration_draft_not_found(build_package_id))?;

    serde_json::from_str::<Value>(&snapshot_json)
        .map_err(|error| format!("Invalid persisted orchestration draft snapshot: {error}"))
}

fn attach_orchestration_stage_runs(conn: &Connection, snapshot: &mut Value) -> Result<(), String> {
    let build_package_id = snapshot_string_field(snapshot, "id")?.to_string();
    let stage_runs = select_orchestration_stage_run_evidence(conn, &build_package_id)?;
    let object = snapshot.as_object_mut().ok_or_else(|| {
        "Persisted orchestration draft snapshot must be a JSON object.".to_string()
    })?;
    object.insert("stageRuns".to_string(), Value::Array(stage_runs));
    Ok(())
}

fn select_orchestration_stage_run_evidence(
    conn: &Connection,
    build_package_id: &str,
) -> Result<Vec<Value>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT
  id, build_package_id, stage_id, status, provenance, status_reason,
  prompt_artifact_id, output_artifact_id, raw_event_artifact_id,
  task_id, task_run_id, conversation_id, event_ids_json, evidence_json,
  started_at, completed_at, created_at, updated_at
FROM orchestration_stage_runs
WHERE build_package_id = ?1
ORDER BY created_at ASC, id ASC
",
        )
        .map_err(sql_error("prepare orchestration stage run evidence query"))?;
    let mut rows = stmt
        .query(params![build_package_id])
        .map_err(sql_error("query orchestration stage run evidence"))?;
    let mut evidence = Vec::new();

    while let Some(row) = rows
        .next()
        .map_err(sql_error("read orchestration stage run evidence row"))?
    {
        let event_ids_json: String = row
            .get(12)
            .map_err(sql_error("read orchestration stage run event ids"))?;
        let event_ids_value = serde_json::from_str::<Value>(&event_ids_json)
            .map_err(|error| format!("Invalid orchestration stage run event IDs: {error}"))?;
        let event_ids = event_ids_value
            .as_array()
            .ok_or_else(|| {
                "Invalid orchestration stage run event IDs: expected array.".to_string()
            })?
            .iter()
            .map(|event_id| {
                event_id
                    .as_str()
                    .map(|value| Value::String(value.to_string()))
                    .ok_or_else(|| {
                        "Invalid orchestration stage run event ID: expected string.".to_string()
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let evidence_json: String = row
            .get(13)
            .map_err(sql_error("read orchestration stage run evidence payload"))?;
        let evidence_value = serde_json::from_str::<Value>(&evidence_json).map_err(|error| {
            format!("Invalid orchestration stage run evidence payload: {error}")
        })?;

        let mut payload = Map::new();
        insert_string(
            &mut payload,
            "id",
            &row.get::<_, String>(0)
                .map_err(sql_error("read orchestration stage run id"))?,
        );
        insert_string(
            &mut payload,
            "buildPackageId",
            &row.get::<_, String>(1)
                .map_err(sql_error("read orchestration stage run build package id"))?,
        );
        insert_string(
            &mut payload,
            "stageId",
            &row.get::<_, String>(2)
                .map_err(sql_error("read orchestration stage run stage id"))?,
        );
        let status = row
            .get::<_, String>(3)
            .map_err(sql_error("read orchestration stage run status"))?;
        let provenance = row
            .get::<_, String>(4)
            .map_err(sql_error("read orchestration stage run provenance"))?;
        payload.insert(
            "state".to_string(),
            json!({
                "status": status,
                "provenance": provenance,
            }),
        );
        insert_optional_string_value(
            &mut payload,
            "statusReason",
            row.get::<_, Option<String>>(5)
                .map_err(sql_error("read orchestration stage run status reason"))?,
        );
        insert_optional_string_value(
            &mut payload,
            "promptArtifactId",
            row.get::<_, Option<String>>(6)
                .map_err(sql_error("read orchestration stage run prompt artifact id"))?,
        );
        insert_optional_string_value(
            &mut payload,
            "outputArtifactId",
            row.get::<_, Option<String>>(7)
                .map_err(sql_error("read orchestration stage run output artifact id"))?,
        );
        insert_optional_string_value(
            &mut payload,
            "rawEventArtifactId",
            row.get::<_, Option<String>>(8).map_err(sql_error(
                "read orchestration stage run raw event artifact id",
            ))?,
        );
        insert_optional_string_value(
            &mut payload,
            "taskId",
            row.get::<_, Option<String>>(9)
                .map_err(sql_error("read orchestration stage run task id"))?,
        );
        insert_optional_string_value(
            &mut payload,
            "taskRunId",
            row.get::<_, Option<String>>(10)
                .map_err(sql_error("read orchestration stage run task run id"))?,
        );
        insert_optional_string_value(
            &mut payload,
            "conversationId",
            row.get::<_, Option<String>>(11)
                .map_err(sql_error("read orchestration stage run conversation id"))?,
        );
        payload.insert("eventIds".to_string(), Value::Array(event_ids));
        payload.insert("evidence".to_string(), evidence_value);
        insert_optional_string_value(
            &mut payload,
            "startedAt",
            row.get::<_, Option<String>>(14)
                .map_err(sql_error("read orchestration stage run started at"))?,
        );
        insert_optional_string_value(
            &mut payload,
            "completedAt",
            row.get::<_, Option<String>>(15)
                .map_err(sql_error("read orchestration stage run completed at"))?,
        );
        insert_string(
            &mut payload,
            "createdAt",
            &row.get::<_, String>(16)
                .map_err(sql_error("read orchestration stage run created at"))?,
        );
        insert_string(
            &mut payload,
            "updatedAt",
            &row.get::<_, String>(17)
                .map_err(sql_error("read orchestration stage run updated at"))?,
        );
        evidence.push(Value::Object(payload));
    }

    Ok(evidence)
}

#[allow(dead_code)]
fn insert_orchestration_stage_run_evidence(
    conn: &Connection,
    record: OrchestrationStageRunEvidenceRecord,
) -> Result<Value, String> {
    let build_package_id = record.build_package_id.clone();
    insert_orchestration_stage_run_evidence_record(conn, &record)?;
    select_orchestration_draft_snapshot(conn, &build_package_id)
}

fn insert_orchestration_stage_run_evidence_record(
    conn: &Connection,
    record: &OrchestrationStageRunEvidenceRecord,
) -> Result<(), String> {
    validate_non_empty("stageRunId", &record.id)?;
    validate_non_empty("buildPackageId", &record.build_package_id)?;
    let _stage_title = orchestration_stage_title(&record.stage_id)?;
    validate_orchestration_stage_run_truth(&record.status, &record.provenance)?;

    if !record.evidence.is_object() {
        return Err("Orchestration stage run evidence must be a JSON object.".to_string());
    }

    select_orchestration_draft_snapshot_base(conn, &record.build_package_id)?;

    let event_ids_json = serde_json::to_string(&record.event_ids).map_err(|error| {
        format!("Unable to serialize orchestration stage run event IDs: {error}")
    })?;
    let evidence_json = serde_json::to_string(&record.evidence).map_err(|error| {
        format!("Unable to serialize orchestration stage run evidence payload: {error}")
    })?;

    conn.execute(
        "
INSERT INTO orchestration_stage_runs (
  id, build_package_id, stage_id, status, provenance, status_reason,
  prompt_artifact_id, output_artifact_id, raw_event_artifact_id,
  task_id, task_run_id, conversation_id, event_ids_json, evidence_json,
  started_at, completed_at, created_at, updated_at
) VALUES (
  ?1, ?2, ?3, ?4, ?5, ?6,
  ?7, ?8, ?9,
  ?10, ?11, ?12, ?13, ?14,
  ?15, ?16, ?17, ?18
)
",
        params![
            record.id,
            record.build_package_id,
            record.stage_id,
            record.status,
            record.provenance,
            record.status_reason,
            record.prompt_artifact_id,
            record.output_artifact_id,
            record.raw_event_artifact_id,
            record.task_id,
            record.task_run_id,
            record.conversation_id,
            event_ids_json,
            evidence_json,
            record.started_at,
            record.completed_at,
            record.created_at,
            record.updated_at,
        ],
    )
    .map_err(sql_error("create orchestration stage run evidence"))?;

    Ok(())
}

fn update_orchestration_stage_run_evidence_record(
    conn: &Connection,
    record: &OrchestrationStageRunEvidenceRecord,
) -> Result<(), String> {
    validate_non_empty("stageRunId", &record.id)?;
    validate_non_empty("buildPackageId", &record.build_package_id)?;
    let _stage_title = orchestration_stage_title(&record.stage_id)?;
    validate_orchestration_stage_run_truth(&record.status, &record.provenance)?;

    if !record.evidence.is_object() {
        return Err("Orchestration stage run evidence must be a JSON object.".to_string());
    }

    let event_ids_json = serde_json::to_string(&record.event_ids).map_err(|error| {
        format!("Unable to serialize orchestration stage run event IDs: {error}")
    })?;
    let evidence_json = serde_json::to_string(&record.evidence).map_err(|error| {
        format!("Unable to serialize orchestration stage run evidence payload: {error}")
    })?;

    let changed = conn
        .execute(
            "
UPDATE orchestration_stage_runs
SET status = ?1,
    provenance = ?2,
    status_reason = ?3,
    prompt_artifact_id = ?4,
    output_artifact_id = ?5,
    raw_event_artifact_id = ?6,
    task_id = ?7,
    task_run_id = ?8,
    conversation_id = ?9,
    event_ids_json = ?10,
    evidence_json = ?11,
    started_at = ?12,
    completed_at = ?13,
    updated_at = ?14
WHERE id = ?15 AND build_package_id = ?16
",
            params![
                record.status,
                record.provenance,
                record.status_reason,
                record.prompt_artifact_id,
                record.output_artifact_id,
                record.raw_event_artifact_id,
                record.task_id,
                record.task_run_id,
                record.conversation_id,
                event_ids_json,
                evidence_json,
                record.started_at,
                record.completed_at,
                record.updated_at,
                record.id,
                record.build_package_id,
            ],
        )
        .map_err(sql_error("update orchestration stage run evidence"))?;

    if changed == 0 {
        return Err(format!("Orchestration stage run not found: {}", record.id));
    }

    Ok(())
}

fn create_orchestration_plan_builder_conversation(
    conn: &Connection,
    snapshot: &Value,
    created_at: &str,
) -> Result<String, String> {
    let conversation_id = Uuid::new_v4().to_string();
    let title = format!(
        "Plan Builder: {}",
        snapshot_string_field(snapshot, "title").unwrap_or("Orchestration draft")
    );

    conn.execute(
        "
INSERT INTO conversations (
  id, task_id, task_run_id, provider, external_thread_id, title, summary, created_at, updated_at
) VALUES (?1, NULL, NULL, 'codex', NULL, ?2, ?3, ?4, ?4)
",
        params![
            conversation_id,
            title,
            "Orchestration Plan Builder runtime conversation",
            created_at
        ],
    )
    .map_err(sql_error("create orchestration Plan Builder conversation"))?;

    Ok(conversation_id)
}

fn build_orchestration_plan_builder_prompt(snapshot: &Value) -> Result<String, String> {
    let source_prompt = snapshot_string_field(snapshot, "sourcePrompt")?;
    let folder_path = snapshot_string_field(snapshot, "folderPath")?;
    let files = snapshot
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "Persisted orchestration draft snapshot is missing files.".to_string())?;
    let mut file_lines = Vec::new();

    for file in files {
        let name = file
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unnamed attachment");
        let size = file.get("size").and_then(Value::as_i64).unwrap_or(0);
        let last_modified = file
            .get("lastModified")
            .and_then(Value::as_i64)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        file_lines.push(format!(
            "- {name} ({size} bytes, lastModified {last_modified})"
        ));
    }

    let attachment_section = if file_lines.is_empty() {
        "No attached files were submitted.".to_string()
    } else {
        format!(
            "Attached files are metadata-only in this increment. Their paths and contents are unavailable and were not sent to Codex as file content:\n{}",
            file_lines.join("\n")
        )
    };

    Ok(format!(
        r#"Use the Orchestration Plan Builder skill intent.

Your job is to turn the raw strategic input below into an orchestration-ready plan draft for user approval.

Hard boundaries:
- Plan only. Do not instantiate an orchestration package.
- Do not create durable files.
- Do not launch root orchestration threads, record threads, or workers.
- Do not claim generated files, conversations, thread ids, or runtime state that this prompt does not provide.
- Produce plan output suitable for the next approval gate, including an orchestrationPlanDraft JSON object if enough source material exists.
- If source material is insufficient, return explicit questions or productBlockers instead of inventing facts.

Known product facts:
- Selected orchestration home candidate: {folder_path}
- The selected path is product metadata for this draft; it is not being used as the Codex process cwd in this increment.
- Live streaming and continuation are unsupported for this run.

{attachment_section}

Raw source material:
{source_prompt}
"#
    ))
}

fn orchestration_plan_builder_evidence_payload(
    snapshot: &Value,
    external_thread_id: Option<&str>,
    error: Option<&str>,
    runtime_route: &str,
) -> Result<Value, String> {
    let files = snapshot
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "Persisted orchestration draft snapshot is missing files.".to_string())?;
    let mut payload = Map::new();
    insert_string(
        &mut payload,
        "schema",
        "orchestration-stage-run-evidence/v1",
    );
    insert_string(&mut payload, "runtimeRoute", runtime_route);
    payload.insert("attachmentContentsSent".to_string(), Value::Bool(false));
    payload.insert(
        "attachmentMetadata".to_string(),
        Value::Array(files.clone()),
    );
    if let Some(external_thread_id) = external_thread_id {
        insert_string(&mut payload, "externalThreadId", external_thread_id);
    }
    if let Some(error) = error {
        insert_string(&mut payload, "error", error);
    }

    Ok(Value::Object(payload))
}

fn append_orchestration_plan_builder_started_event(
    conn: &Connection,
    build_package_id: &str,
    stage_run_id: &str,
    conversation_id: &str,
    started_at: &str,
) -> Result<String, String> {
    let mut payload = Map::new();
    insert_string(&mut payload, "buildPackageId", build_package_id);
    insert_string(&mut payload, "stageRunId", stage_run_id);
    insert_string(&mut payload, "stageId", "plan-builder");
    insert_string(&mut payload, "conversationId", conversation_id);
    insert_string(&mut payload, "startedAt", started_at);
    insert_string(
        &mut payload,
        "runtimeRoute",
        "start_orchestration_plan_builder_run",
    );

    create_event(
        conn,
        "run_started",
        started_at,
        None,
        None,
        None,
        Some(conversation_id),
        None,
        None,
        payload,
    )
}

fn append_orchestration_artifact_created_event(
    conn: &Connection,
    build_package_id: &str,
    stage_run_id: &str,
    conversation_id: &str,
    artifact_id: &str,
    artifact_kind: &str,
    label: &str,
    content_length: Option<i64>,
) -> Result<String, String> {
    let occurred_at = now_iso();
    let mut payload = Map::new();
    insert_string(&mut payload, "buildPackageId", build_package_id);
    insert_string(&mut payload, "stageRunId", stage_run_id);
    insert_string(&mut payload, "stageId", "plan-builder");
    insert_string(&mut payload, "artifactKind", artifact_kind);
    insert_string(&mut payload, "artifactId", artifact_id);
    insert_string(&mut payload, "label", label);
    if let Some(content_length) = content_length {
        insert_i64(&mut payload, "contentLength", content_length);
    }

    create_event(
        conn,
        "artifact_created",
        &occurred_at,
        None,
        None,
        None,
        Some(conversation_id),
        Some(artifact_id),
        None,
        payload,
    )
}

fn append_orchestration_plan_builder_completed_event(
    conn: &Connection,
    build_package_id: &str,
    stage_run_id: &str,
    conversation_id: &str,
    outcome: &str,
    error: Option<&str>,
    exit_code: Option<i64>,
) -> Result<String, String> {
    let completed_at = now_iso();
    let mut payload = Map::new();
    insert_string(&mut payload, "buildPackageId", build_package_id);
    insert_string(&mut payload, "stageRunId", stage_run_id);
    insert_string(&mut payload, "stageId", "plan-builder");
    insert_string(&mut payload, "outcome", outcome);
    insert_string(&mut payload, "completedAt", &completed_at);
    if let Some(error) = error {
        insert_string(&mut payload, "error", error);
    }
    if let Some(exit_code) = exit_code {
        insert_i64(&mut payload, "exitCode", exit_code);
    }

    create_event(
        conn,
        "run_completed",
        &completed_at,
        None,
        None,
        None,
        Some(conversation_id),
        None,
        None,
        payload,
    )
}

fn update_orchestration_conversation_from_runtime_result(
    conn: &Connection,
    conversation_id: &str,
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
            conversation_id
        ],
    )
    .map_err(sql_error("update orchestration Plan Builder conversation"))?;

    Ok(())
}

fn insert_orchestration_draft_snapshot(
    conn: &Connection,
    build_package_id: &str,
    title: &str,
    folder_path: &str,
    source_prompt: &str,
    snapshot: &Value,
    created_at: &str,
) -> Result<(), String> {
    let stored_snapshot = orchestration_draft_snapshot_for_storage(snapshot);
    let snapshot_json = serde_json::to_string(&stored_snapshot)
        .map_err(|error| format!("Unable to serialize orchestration draft snapshot: {error}"))?;

    conn.execute(
        "
INSERT INTO orchestration_drafts (
  id, title, folder_path, source_prompt, status, provenance, snapshot_json,
  created_at, updated_at, canceled_at
) VALUES (?1, ?2, ?3, ?4, 'integration_pending', 'persisted_snapshot', ?5, ?6, ?6, NULL)
",
        params![
            build_package_id,
            title,
            folder_path,
            source_prompt,
            snapshot_json,
            created_at
        ],
    )
    .map_err(sql_error("create orchestration draft"))?;

    Ok(())
}

fn update_orchestration_draft_snapshot(
    conn: &Connection,
    build_package_id: &str,
    snapshot: &Value,
    updated_at: &str,
) -> Result<(), String> {
    let stored_snapshot = orchestration_draft_snapshot_for_storage(snapshot);
    let snapshot_json = serde_json::to_string(&stored_snapshot)
        .map_err(|error| format!("Unable to serialize orchestration draft snapshot: {error}"))?;
    let changed = conn
        .execute(
            "
UPDATE orchestration_drafts
SET title = ?1,
    folder_path = ?2,
    source_prompt = ?3,
    status = ?4,
    provenance = ?5,
    snapshot_json = ?6,
    updated_at = ?7
WHERE id = ?8 AND canceled_at IS NULL
",
            params![
                snapshot_string_field(snapshot, "title")?,
                snapshot_string_field(snapshot, "folderPath")?,
                snapshot_string_field(snapshot, "sourcePrompt")?,
                orchestration_snapshot_client_state_field(snapshot, "status")
                    .unwrap_or("integration_pending"),
                orchestration_snapshot_client_state_field(snapshot, "provenance")
                    .unwrap_or("persisted_snapshot"),
                snapshot_json,
                updated_at,
                build_package_id
            ],
        )
        .map_err(sql_error("update orchestration draft"))?;

    if changed == 0 {
        return Err(orchestration_draft_not_found(build_package_id));
    }

    Ok(())
}

fn orchestration_draft_snapshot_for_storage(snapshot: &Value) -> Value {
    let mut stored_snapshot = snapshot.clone();

    if let Some(object) = stored_snapshot.as_object_mut() {
        object.remove("stageRuns");
    }

    stored_snapshot
}

fn build_persisted_orchestration_draft_snapshot(
    id: &str,
    created_at: &str,
    title: &str,
    folder_path: &str,
    prompt: &str,
    files: &[UploadedOrchestrationDraftFileInput],
) -> Value {
    let plan_preview = vec![
        first_meaningful_line(prompt).unwrap_or(title).to_string(),
        "Separate strategic problem structure from executable work-slice planning.".to_string(),
        "Prepare instantiator-ready files only after a supported backend writes them.".to_string(),
    ];

    json!({
        "id": id,
        "title": title,
        "folderPath": folder_path,
        "sourcePrompt": prompt,
        "createdAt": created_at,
        "updatedAt": created_at,
        "clientState": persisted_orchestration_client_state(
            id,
            created_at,
            vec![orchestration_registry_notice()],
        ),
        "messages": [
            {
                "id": format!("message-{}", Uuid::new_v4()),
                "role": "system",
                "body": "Draft persisted locally. Plan-builder runtime has not started yet.",
                "createdAt": created_at,
                "state": "completed",
                "truth": persisted_draft_truth_state(),
            },
            {
                "id": format!("message-{}", Uuid::new_v4()),
                "role": "user",
                "body": prompt,
                "createdAt": created_at,
                "state": "completed",
                "truth": { "status": "ready", "provenance": "user_input" },
            },
            {
                "id": format!("message-{}", Uuid::new_v4()),
                "role": "system",
                "body": "Backend integration pending. The prompt is saved, but no plan-builder output, generated files, or Codex threads exist yet.",
                "createdAt": created_at,
                "state": "completed",
                "truth": unsupported_pending_truth_state(),
            }
        ],
        "files": files.iter().map(uploaded_orchestration_file_value).collect::<Vec<_>>(),
        "stages": initial_orchestration_build_stages(),
        "stageRuns": [],
        "runtimeRoutes": [blocked_plan_builder_runtime_route(created_at)],
        "generatedFiles": expected_orchestration_output_slots(),
        "planPreview": plan_preview,
    })
}

fn initial_orchestration_build_stages() -> Vec<Value> {
    vec![
        json!({
            "id": "plan-builder",
            "title": "Plan Builder",
            "state": unsupported_pending_truth_state(),
            "summary": "Prompt is saved; no plan-builder output exists yet.",
            "detail": "The backend persisted the draft. Plan-builder execution is still unsupported in this adapter.",
        }),
        json!({
            "id": "plan-review",
            "title": "Review Pending",
            "state": { "status": "blocked", "provenance": "unsupported" },
            "summary": "No plan-builder output is available to review.",
            "detail": "Review waits for real plan-builder output from a supported backend path.",
        }),
        json!({
            "id": "instantiator",
            "title": "Instantiator",
            "state": { "status": "blocked", "provenance": "unsupported" },
            "summary": "Instantiation is not available in this UI path yet.",
            "detail": "No files have been generated. The future instantiator step needs backend support before it can write to the selected folder.",
        }),
        json!({
            "id": "root-startup",
            "title": "Root Startup",
            "state": { "status": "blocked", "provenance": "unsupported" },
            "summary": "Live root startup has not been prepared.",
            "detail": "No root orchestration or record threads have been created from this draft.",
        }),
    ]
}

fn expected_orchestration_output_slots() -> Vec<Value> {
    vec![]
}

fn persisted_orchestration_client_state(id: &str, updated_at: &str, notices: Vec<Value>) -> Value {
    json!({
        "id": id,
        "status": "integration_pending",
        "provenance": "persisted_snapshot",
        "currentAction": "Draft is persisted locally; no explicit task/worktree route is linked for plan-builder, so no Codex run can start.",
        "updatedAt": updated_at,
        "persisted": true,
        "runtimeSupported": false,
        "notices": notices,
        "primaryAction": {
            "id": "request-build-stage",
            "label": "Plan-builder route required",
            "enabled": false,
            "reason": "Plan-builder requires an explicit linked task/worktree route. This draft has none.",
        },
    })
}

fn runtime_orchestration_client_state(
    id: &str,
    updated_at: &str,
    status: &str,
    provenance: &str,
    current_action: &str,
    notices: Vec<Value>,
) -> Value {
    let mut state = json!({
        "id": id,
        "status": status,
        "provenance": provenance,
        "currentAction": current_action,
        "updatedAt": updated_at,
        "persisted": true,
        "runtimeSupported": true,
        "notices": notices,
    });

    if status == "completed" {
        if let Some(object) = state.as_object_mut() {
            object.insert(
                "primaryAction".to_string(),
                json!({
                    "id": "start-instantiation",
                    "label": "Confirm build plan and start instantiating",
                    "enabled": true,
                }),
            );
        }
    }

    state
}

fn orchestration_registry_client_state() -> Value {
    json!({
        "status": "integration_pending",
        "provenance": "persisted_snapshot",
        "currentAction": "Persisted orchestration drafts are available. Runtime execution, generated files, and Codex threads are not connected yet.",
        "updatedAt": now_iso(),
        "persisted": true,
        "runtimeSupported": false,
        "notices": [orchestration_registry_notice()],
    })
}

fn orchestration_plan_builder_waiting_notice() -> Value {
    json!({
        "id": "plan-builder-runtime-waiting",
        "kind": "missing_capability",
        "title": "Waiting for final response",
        "message": "The backend accepted the Plan Builder runtime request. Live streaming is not available in this increment, so the UI waits for final output.",
        "truth": { "status": "waiting_for_event", "provenance": "backend_response" },
    })
}

fn orchestration_plan_builder_completed_notice() -> Value {
    json!({
        "id": "plan-builder-runtime-completed",
        "kind": "missing_capability",
        "title": "Plan Builder completed",
        "message": "Plan Builder completed with persisted prompt, raw Codex JSONL, final response, and stage-run evidence. Review the proposal before approving instantiation.",
        "truth": { "status": "completed", "provenance": "backend_response" },
    })
}

fn unsupported_orchestration_continuation_notice() -> Value {
    json!({
        "id": "unsupported-plan-builder-continuation",
        "kind": "missing_capability",
        "title": "Runtime continuation unsupported",
        "message": "Feedback was preserved locally, but it was not sent to the same Plan Builder runtime conversation because continuation is unsupported in this path.",
        "truth": unsupported_pending_truth_state(),
    })
}

fn orchestration_plan_builder_failed_notice(error: &str) -> Value {
    json!({
        "id": "plan-builder-runtime-failed",
        "kind": "error",
        "title": "Plan Builder runtime failed",
        "message": format!("The draft remains saved, but the Plan Builder runtime failed. {error}"),
        "truth": { "status": "failed", "provenance": "backend_response" },
    })
}

fn orchestration_registry_notice() -> Value {
    json!({
        "id": "runtime-integration-pending",
        "kind": "missing_capability",
        "title": "Runtime integration pending",
        "message": "Orchestration drafts are persisted locally, but no explicit task/worktree route is linked for plan-builder execution yet.",
        "truth": unsupported_pending_truth_state(),
    })
}

fn missing_orchestration_runtime_notice(stage_id: &str, stage_title: &str) -> Value {
    if stage_id == "plan-builder" {
        return json!({
            "id": "missing-plan-builder-route",
            "kind": "blocker",
            "title": "Plan-builder route required",
            "message": "Plan builder cannot start because this draft has no explicit linked task/worktree route. No Codex run was started.",
            "truth": { "status": "blocked", "provenance": "unsupported" },
        });
    }

    if stage_id == "instantiator" {
        return json!({
            "id": "missing-instantiator-runtime",
            "kind": "missing_capability",
            "title": "Instantiator runtime unavailable",
            "message": "The build plan approval was accepted, but instantiation cannot start because no instantiator runtime route is implemented. No files were generated.",
            "truth": unsupported_pending_truth_state(),
        });
    }

    json!({
        "id": format!("missing-{stage_id}-runtime"),
        "kind": "missing_capability",
        "title": "Runtime integration pending",
        "message": format!("{stage_title} cannot advance because the orchestration runtime adapter is not implemented yet. Your draft remains saved."),
        "truth": unsupported_pending_truth_state(),
    })
}

fn live_orchestration_runtime_notice() -> Value {
    json!({
        "id": "missing-live-runtime",
        "kind": "missing_capability",
        "title": "Live orchestration runtime unavailable",
        "message": "The persisted draft cannot create Codex threads or live orchestration roots until runtime integration is implemented.",
        "truth": unsupported_pending_truth_state(),
    })
}

fn persisted_draft_truth_state() -> Value {
    json!({ "status": "draft", "provenance": "persisted_snapshot" })
}

fn unsupported_pending_truth_state() -> Value {
    json!({ "status": "integration_pending", "provenance": "unsupported" })
}

fn blocked_plan_builder_runtime_route(updated_at: &str) -> Value {
    json!({
        "stageId": "plan-builder",
        "status": "blocked",
        "truth": { "status": "blocked", "provenance": "unsupported" },
        "reason": "No explicit Open Task/worktree runtime route is linked to this orchestration draft. The selected folder is not treated as a runnable cwd.",
        "runtimeCommand": "startCodexTaskRun",
        "updatedAt": updated_at,
    })
}

fn plan_builder_runtime_route(updated_at: &str, status: &str, provenance: &str) -> Value {
    json!({
        "stageId": "plan-builder",
        "status": "supported",
        "truth": { "status": status, "provenance": provenance },
        "reason": "Plan Builder uses the orchestration-specific Tauri command and non-interactive Codex JSON mode. Live streaming and continuation are unsupported in this increment.",
        "runtimeCommand": "startOrchestrationPlanBuilderRun",
        "updatedAt": updated_at,
    })
}

fn set_orchestration_snapshot_updated_state(
    snapshot: &mut Value,
    id: &str,
    updated_at: &str,
    notices: Vec<Value>,
) -> Result<(), String> {
    let object = snapshot.as_object_mut().ok_or_else(|| {
        "Persisted orchestration draft snapshot must be a JSON object.".to_string()
    })?;
    object.insert(
        "updatedAt".to_string(),
        Value::String(updated_at.to_string()),
    );
    object.insert(
        "clientState".to_string(),
        persisted_orchestration_client_state(id, updated_at, notices),
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_orchestration_plan_builder_runtime_state(
    snapshot: &mut Value,
    id: &str,
    status: &str,
    provenance: &str,
    current_action: &str,
    stage_summary: &str,
    stage_detail: &str,
    notices: Vec<Value>,
) -> Result<(), String> {
    let updated_at = now_iso();
    let object = snapshot.as_object_mut().ok_or_else(|| {
        "Persisted orchestration draft snapshot must be a JSON object.".to_string()
    })?;
    object.insert("updatedAt".to_string(), Value::String(updated_at.clone()));
    object.insert(
        "clientState".to_string(),
        runtime_orchestration_client_state(
            id,
            &updated_at,
            status,
            provenance,
            current_action,
            notices,
        ),
    );
    object.insert(
        "runtimeRoutes".to_string(),
        Value::Array(vec![plan_builder_runtime_route(
            &updated_at,
            status,
            provenance,
        )]),
    );

    let stages = snapshot_array_mut(snapshot, "stages")?;
    for stage in stages {
        let Some(stage_object) = stage.as_object_mut() else {
            continue;
        };
        let is_plan_builder = stage_object
            .get("id")
            .and_then(Value::as_str)
            .is_some_and(|stage_id| stage_id == "plan-builder");

        if is_plan_builder {
            stage_object.insert(
                "state".to_string(),
                json!({ "status": status, "provenance": provenance }),
            );
            stage_object.insert(
                "summary".to_string(),
                Value::String(stage_summary.to_string()),
            );
            stage_object.insert(
                "detail".to_string(),
                Value::String(stage_detail.to_string()),
            );
        } else if status == "completed"
            && stage_object
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|stage_id| stage_id == "plan-review")
        {
            stage_object.insert(
                "state".to_string(),
                json!({ "status": "ready", "provenance": "backend_response" }),
            );
            stage_object.insert(
                "summary".to_string(),
                Value::String("Plan-builder proposal is ready for user review.".to_string()),
            );
            stage_object.insert(
                "detail".to_string(),
                Value::String(
                    "Review the final Plan Builder response. Instantiation starts only after explicit user approval."
                        .to_string(),
                ),
            );
        }
    }

    Ok(())
}

fn apply_orchestration_plan_builder_terminal_message(
    snapshot: &mut Value,
    created_at: &str,
    role: &str,
    body: &str,
    status: &str,
    provenance: &str,
) -> Result<(), String> {
    snapshot_array_mut(snapshot, "messages")?.push(json!({
        "id": format!("message-{}", Uuid::new_v4()),
        "role": role,
        "body": body,
        "createdAt": created_at,
        "state": "completed",
        "truth": { "status": status, "provenance": provenance },
    }));
    Ok(())
}

fn snapshot_array_mut<'a>(
    snapshot: &'a mut Value,
    key: &str,
) -> Result<&'a mut Vec<Value>, String> {
    snapshot
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("Persisted orchestration draft snapshot is missing {key}."))
}

fn snapshot_string_field<'a>(snapshot: &'a Value, key: &str) -> Result<&'a str, String> {
    snapshot
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Persisted orchestration draft snapshot is missing {key}."))
}

fn orchestration_snapshot_client_state_field<'a>(
    snapshot: &'a Value,
    key: &str,
) -> Option<&'a str> {
    snapshot.get("clientState")?.get(key)?.as_str()
}

fn uploaded_orchestration_file_value(file: &UploadedOrchestrationDraftFileInput) -> Value {
    match file.last_modified {
        Some(last_modified) => json!({
            "id": file.id,
            "name": file.name,
            "size": file.size,
            "lastModified": last_modified,
        }),
        None => json!({
            "id": file.id,
            "name": file.name,
            "size": file.size,
        }),
    }
}

fn uploaded_orchestration_file_input_key(file: &UploadedOrchestrationDraftFileInput) -> String {
    format!(
        "{}:{}:{}",
        file.name,
        file.size,
        file.last_modified
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    )
}

fn uploaded_orchestration_file_key(file: &Value) -> Option<String> {
    let name = file.get("name")?.as_str()?;
    let size = file.get("size")?.as_i64()?;
    let last_modified = file
        .get("lastModified")
        .and_then(Value::as_i64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    Some(format!("{name}:{size}:{last_modified}"))
}

fn orchestration_stage_title(stage_id: &str) -> Result<&'static str, String> {
    match stage_id {
        "plan-builder" => Ok("Plan builder"),
        "plan-review" => Ok("Plan review"),
        "instantiator" => Ok("Instantiator"),
        "root-startup" => Ok("Root startup"),
        _ => Err(format!("Unknown orchestration build stage: {stage_id}")),
    }
}

#[allow(dead_code)]
fn validate_orchestration_stage_run_truth(status: &str, provenance: &str) -> Result<(), String> {
    if !is_valid_orchestration_status(status) {
        return Err(format!("Invalid orchestration stage run status: {status}"));
    }

    if !is_valid_orchestration_provenance(provenance) {
        return Err(format!(
            "Invalid orchestration stage run provenance: {provenance}"
        ));
    }

    if matches!(
        status,
        "starting" | "waiting_for_event" | "running" | "completed"
    ) && !matches!(provenance, "backend_response" | "runtime_event")
    {
        return Err(format!(
            "Runtime stage status {status} requires backend_response or runtime_event provenance."
        ));
    }

    Ok(())
}

#[allow(dead_code)]
fn is_valid_orchestration_status(status: &str) -> bool {
    matches!(
        status,
        "draft"
            | "ready"
            | "starting"
            | "waiting_for_event"
            | "running"
            | "blocked"
            | "failed"
            | "completed"
            | "integration_pending"
            | "mock_preview"
    )
}

#[allow(dead_code)]
fn is_valid_orchestration_provenance(provenance: &str) -> bool {
    matches!(
        provenance,
        "user_input"
            | "local_draft"
            | "persisted_snapshot"
            | "backend_response"
            | "runtime_event"
            | "mock_fixture"
            | "unsupported"
    )
}

fn first_meaningful_line(value: &str) -> Option<&str> {
    value.lines().map(str::trim).find(|line| !line.is_empty())
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

fn insert_optional_string_value(
    payload: &mut Map<String, Value>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        insert_string(payload, key, &value);
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

fn orchestration_draft_not_found(build_package_id: &str) -> String {
    format!("Orchestration draft not found: {build_package_id}")
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
    sql: &'static str,
}

fn app_migrations() -> [Migration; 8] {
    [
        Migration {
            id: "001_repo_sync_schema",
            sql: REPO_SYNC_SCHEMA,
        },
        Migration {
            id: "002_open_tasks_schema",
            sql: TASK_SCHEMA,
        },
        Migration {
            id: "003_task_runs_conversations_schema",
            sql: RUN_CONVERSATION_SCHEMA,
        },
        Migration {
            id: "004_artifacts_validation_runs_schema",
            sql: ARTIFACT_VALIDATION_SCHEMA,
        },
        Migration {
            id: "005_events_schema",
            sql: EVENT_SCHEMA,
        },
        Migration {
            id: "006_orchestration_drafts_schema",
            sql: ORCHESTRATION_DRAFT_SCHEMA,
        },
        Migration {
            id: "007_orchestration_stage_runs_schema",
            sql: ORCHESTRATION_STAGE_RUN_SCHEMA,
        },
        Migration {
            id: "008_agent_sessions_schema",
            sql: AGENT_SESSION_SCHEMA,
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

const ORCHESTRATION_DRAFT_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS orchestration_drafts (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  folder_path TEXT NOT NULL,
  source_prompt TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('draft', 'ready', 'starting', 'waiting_for_event', 'running', 'blocked', 'failed', 'completed', 'integration_pending', 'mock_preview')),
  provenance TEXT NOT NULL CHECK (provenance IN ('user_input', 'local_draft', 'persisted_snapshot', 'backend_response', 'runtime_event', 'mock_fixture', 'unsupported')),
  snapshot_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  canceled_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_orchestration_drafts_active_updated
ON orchestration_drafts (canceled_at, updated_at DESC, created_at DESC);
";

const ORCHESTRATION_STAGE_RUN_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS orchestration_stage_runs (
  id TEXT PRIMARY KEY,
  build_package_id TEXT NOT NULL,
  stage_id TEXT NOT NULL CHECK (stage_id IN ('plan-builder', 'plan-review', 'instantiator', 'root-startup')),
  status TEXT NOT NULL CHECK (status IN ('draft', 'ready', 'starting', 'waiting_for_event', 'running', 'blocked', 'failed', 'completed', 'integration_pending', 'mock_preview')),
  provenance TEXT NOT NULL CHECK (provenance IN ('user_input', 'local_draft', 'persisted_snapshot', 'backend_response', 'runtime_event', 'mock_fixture', 'unsupported')),
  status_reason TEXT,
  prompt_artifact_id TEXT,
  output_artifact_id TEXT,
  raw_event_artifact_id TEXT,
  task_id TEXT,
  task_run_id TEXT,
  conversation_id TEXT,
  event_ids_json TEXT NOT NULL DEFAULT '[]',
  evidence_json TEXT NOT NULL DEFAULT '{}',
  started_at TEXT,
  completed_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (build_package_id) REFERENCES orchestration_drafts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_orchestration_stage_runs_build_stage
ON orchestration_stage_runs (build_package_id, stage_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_orchestration_stage_runs_task_links
ON orchestration_stage_runs (task_id, task_run_id, conversation_id);
";

const AGENT_SESSION_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS agent_sessions (
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

CREATE TABLE IF NOT EXISTS agent_session_cli_logs (
  id TEXT PRIMARY KEY,
  agent_session_id TEXT NOT NULL,
  stream_id TEXT NOT NULL,
  stdout TEXT NOT NULL,
  stderr TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (agent_session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
);
";

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::named_params;
    use serde_json::json;
    use std::cell::RefCell;

    const CREATED_AT: &str = "2026-07-02T10:00:00.000Z";

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
    fn agent_session_uses_codex_exec_without_requiring_a_terminal() {
        let input = StartAgentSessionCommandInput {
            stream_id: None,
            session_id: None,
            prompt: "Test prompt".to_string(),
            cwd: None,
            additional_args: None,
            env: None,
        };

        assert_eq!(
            build_agent_session_args(&input),
            vec![
                "exec".to_string(),
                "--json".to_string(),
                "Test prompt".to_string()
            ]
        );
    }

    #[test]
    fn agent_session_resume_uses_codex_exec_resume_with_additional_args() {
        let input = StartAgentSessionCommandInput {
            stream_id: None,
            session_id: Some("agent-session-1".to_string()),
            prompt: "Continue the task".to_string(),
            cwd: None,
            additional_args: Some(vec!["--model".to_string(), "gpt-5.5".to_string()]),
            env: None,
        };

        assert_eq!(
            build_agent_session_args(&input),
            vec![
                "exec".to_string(),
                "--json".to_string(),
                "--model".to_string(),
                "gpt-5.5".to_string(),
                "resume".to_string(),
                "agent-session-1".to_string(),
                "Continue the task".to_string(),
            ]
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
    fn orchestration_drafts_are_persisted_and_rehydrated_without_runtime_progress() {
        let conn = open_memory_database();

        let created = create_orchestration_draft_record(
            &conn,
            CreateOrchestrationDraftCommandInput {
                title: "Persist orchestration draft".to_string(),
                folder_path: "C:/orchestrations/persist".to_string(),
                prompt: "Keep this prompt after reload.".to_string(),
                files: vec![UploadedOrchestrationDraftFileInput {
                    id: "file-1".to_string(),
                    name: "handoff.md".to_string(),
                    size: 12,
                    last_modified: Some(1_788_888_000_000),
                }],
            },
        )
        .expect("create orchestration draft");

        assert_eq!(created["clientState"]["persisted"], true);
        assert_eq!(created["clientState"]["runtimeSupported"], false);
        assert_eq!(created["clientState"]["provenance"], "persisted_snapshot");
        assert_eq!(
            created["stages"][0]["state"]["status"],
            "integration_pending"
        );
        assert_eq!(created["stages"][0]["state"]["provenance"], "unsupported");
        assert!(created["generatedFiles"]
            .as_array()
            .expect("generated files")
            .is_empty());
        assert!(created["stageRuns"].as_array().unwrap().is_empty());
        assert_eq!(created["runtimeRoutes"][0]["stageId"], "plan-builder");
        assert_eq!(created["runtimeRoutes"][0]["status"], "blocked");
        assert_eq!(
            created["runtimeRoutes"][0]["truth"],
            json!({ "status": "blocked", "provenance": "unsupported" })
        );
        assert!(created["runtimeRoutes"][0].get("cwd").is_none());
        assert!(created["runtimeRoutes"][0].get("taskId").is_none());
        assert!(created["runtimeRoutes"][0].get("worktreeId").is_none());

        let registry = load_orchestration_registry_snapshot(&conn).expect("registry snapshot");
        assert_eq!(registry["orchestrations"].as_array().unwrap().len(), 0);
        assert_eq!(registry["buildPackages"].as_array().unwrap().len(), 1);
        assert!(registry["buildPackages"][0]["stageRuns"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(
            registry["buildPackages"][0]["sourcePrompt"],
            "Keep this prompt after reload."
        );
        assert_eq!(registry["clientState"]["persisted"], true);
    }

    #[test]
    fn load_orchestration_snapshot_returns_explicit_integration_pending_error() {
        let conn = open_memory_database();
        let created = create_orchestration_draft_record(
            &conn,
            CreateOrchestrationDraftCommandInput {
                title: "Persist orchestration draft".to_string(),
                folder_path: "C:/orchestrations/persist".to_string(),
                prompt: "Keep this prompt after reload.".to_string(),
                files: vec![],
            },
        )
        .expect("create orchestration draft");
        let build_package_id = created["id"].as_str().unwrap().to_string();

        let error = load_orchestration_snapshot(&conn, &build_package_id)
            .expect_err("live orchestration loading is not implemented");

        assert!(error.contains("integration-pending"));
        assert!(error.contains("does not persist or fabricate live orchestration snapshots"));
    }

    #[test]
    fn orchestration_draft_note_and_files_update_persisted_snapshot() {
        let conn = open_memory_database();
        let created = create_orchestration_draft_record(
            &conn,
            CreateOrchestrationDraftCommandInput {
                title: "Persist orchestration draft".to_string(),
                folder_path: "C:/orchestrations/persist".to_string(),
                prompt: "Keep this prompt after reload.".to_string(),
                files: vec![],
            },
        )
        .expect("create orchestration draft");
        let build_package_id = created["id"].as_str().unwrap().to_string();

        let noted = add_orchestration_draft_note_record(
            &conn,
            AddOrchestrationDraftNoteCommandInput {
                build_package_id: build_package_id.clone(),
                body: "Additional local context.".to_string(),
            },
        )
        .expect("add note");
        assert_eq!(noted["messages"].as_array().unwrap().len(), 5);
        assert_eq!(
            noted["messages"][3]["truth"],
            json!({ "status": "draft", "provenance": "persisted_snapshot" })
        );
        assert_eq!(
            noted["messages"][4]["truth"],
            json!({ "status": "integration_pending", "provenance": "unsupported" })
        );
        assert!(noted["messages"][4]["body"]
            .as_str()
            .unwrap()
            .contains("not sent to the same Plan Builder runtime conversation"));

        let with_files = attach_orchestration_draft_files_record(
            &conn,
            AttachOrchestrationDraftFilesCommandInput {
                build_package_id: build_package_id.clone(),
                files: vec![
                    UploadedOrchestrationDraftFileInput {
                        id: "file-1".to_string(),
                        name: "roadmap.md".to_string(),
                        size: 24,
                        last_modified: None,
                    },
                    UploadedOrchestrationDraftFileInput {
                        id: "file-2".to_string(),
                        name: "roadmap.md".to_string(),
                        size: 24,
                        last_modified: None,
                    },
                ],
            },
        )
        .expect("attach files");
        assert_eq!(with_files["files"].as_array().unwrap().len(), 1);

        let registry = load_orchestration_registry_snapshot(&conn).expect("registry snapshot");
        assert_eq!(
            registry["buildPackages"][0]["messages"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert_eq!(
            registry["buildPackages"][0]["files"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn orchestration_runtime_requests_return_unsupported_without_completing_stages() {
        let conn = open_memory_database();
        let created = create_orchestration_draft_record(
            &conn,
            CreateOrchestrationDraftCommandInput {
                title: "Persist orchestration draft".to_string(),
                folder_path: "C:/orchestrations/persist".to_string(),
                prompt: "Keep this prompt after reload.".to_string(),
                files: vec![],
            },
        )
        .expect("create orchestration draft");
        let build_package_id = created["id"].as_str().unwrap().to_string();

        let stage_result = request_orchestration_build_stage_record(
            &conn,
            RequestOrchestrationBuildStageCommandInput {
                build_package_id: build_package_id.clone(),
                stage_id: "plan-builder".to_string(),
            },
        )
        .expect("request build stage");
        assert_eq!(
            stage_result["clientState"]["notices"][0]["truth"],
            json!({ "status": "blocked", "provenance": "unsupported" })
        );
        assert_eq!(stage_result["clientState"]["notices"][0]["kind"], "blocker");
        assert_eq!(
            stage_result["stages"][0]["state"],
            json!({ "status": "blocked", "provenance": "unsupported" })
        );
        assert!(stage_result["stages"]
            .as_array()
            .expect("stages")
            .iter()
            .all(|stage| stage["state"]["status"] != "completed"));
        assert!(stage_result["stageRuns"].as_array().unwrap().is_empty());
        assert!(stage_result["runtimeRoutes"][0].get("cwd").is_none());
        assert!(stage_result["runtimeRoutes"][0].get("taskId").is_none());
        assert!(stage_result["runtimeRoutes"][0].get("worktreeId").is_none());
        assert!(stage_result["generatedFiles"]
            .as_array()
            .expect("generated files")
            .iter()
            .all(|file| file["state"]["status"] != "completed"));

        let start_result = start_orchestration_record(
            &conn,
            StartOrchestrationCommandInput {
                build_package_id: build_package_id.clone(),
            },
        )
        .expect("start orchestration");
        assert!(start_result.get("orchestration").is_none());
        assert_eq!(start_result["clientState"]["runtimeSupported"], false);
        assert_eq!(
            start_result["clientState"]["notices"][0]["truth"],
            json!({ "status": "integration_pending", "provenance": "unsupported" })
        );
    }

    #[test]
    fn orchestration_stage_run_evidence_is_owned_and_rehydrated_without_implied_outputs() {
        let conn = open_memory_database();
        let created = create_orchestration_draft_record(
            &conn,
            CreateOrchestrationDraftCommandInput {
                title: "Persist orchestration draft".to_string(),
                folder_path: "C:/orchestrations/persist".to_string(),
                prompt: "Keep this prompt after reload.".to_string(),
                files: vec![],
            },
        )
        .expect("create orchestration draft");
        let build_package_id = created["id"].as_str().unwrap().to_string();

        let with_evidence = insert_orchestration_stage_run_evidence(
            &conn,
            stage_run_evidence_record(
                &build_package_id,
                "completed",
                "backend_response",
                Some("Plan-builder command returned a structured output artifact.".to_string()),
            ),
        )
        .expect("insert stage run evidence");

        assert_eq!(with_evidence["stageRuns"].as_array().unwrap().len(), 1);
        assert_eq!(with_evidence["stageRuns"][0]["stageId"], "plan-builder");
        assert_eq!(
            with_evidence["stageRuns"][0]["state"],
            json!({ "status": "completed", "provenance": "backend_response" })
        );
        assert_eq!(
            with_evidence["stageRuns"][0]["promptArtifactId"],
            "artifact-prompt-1"
        );
        assert_eq!(
            with_evidence["stageRuns"][0]["outputArtifactId"],
            "artifact-output-1"
        );
        assert_eq!(
            with_evidence["stageRuns"][0]["rawEventArtifactId"],
            "artifact-events-1"
        );
        assert_eq!(
            with_evidence["stageRuns"][0]["eventIds"],
            json!(["event-1", "event-2"])
        );
        assert_eq!(
            with_evidence["stageRuns"][0]["evidence"]["schema"],
            "orchestration-stage-run-evidence/v1"
        );
        assert_eq!(
            with_evidence["stages"][0]["state"],
            json!({ "status": "integration_pending", "provenance": "unsupported" })
        );
        assert!(with_evidence["generatedFiles"]
            .as_array()
            .expect("generated files")
            .iter()
            .all(|file| file["state"]["status"] != "completed"));

        let registry = load_orchestration_registry_snapshot(&conn).expect("registry snapshot");
        assert_eq!(
            registry["buildPackages"][0]["stageRuns"][0]["taskRunId"],
            "task-run-1"
        );
        assert_eq!(
            registry["buildPackages"][0]["stageRuns"][0]["conversationId"],
            "conversation-1"
        );
    }

    #[test]
    fn orchestration_stage_run_evidence_rejects_runtime_claims_without_runtime_provenance() {
        let conn = open_memory_database();
        let created = create_orchestration_draft_record(
            &conn,
            CreateOrchestrationDraftCommandInput {
                title: "Persist orchestration draft".to_string(),
                folder_path: "C:/orchestrations/persist".to_string(),
                prompt: "Keep this prompt after reload.".to_string(),
                files: vec![],
            },
        )
        .expect("create orchestration draft");
        let build_package_id = created["id"].as_str().unwrap().to_string();

        let error = insert_orchestration_stage_run_evidence(
            &conn,
            stage_run_evidence_record(&build_package_id, "running", "persisted_snapshot", None),
        )
        .expect_err("reject unsupported runtime claim");

        assert!(error.contains("requires backend_response or runtime_event provenance"));
        let registry = load_orchestration_registry_snapshot(&conn).expect("registry snapshot");
        assert!(registry["buildPackages"][0]["stageRuns"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn start_orchestration_plan_builder_run_persists_final_output_raw_stream_and_metadata_only_attachments(
    ) {
        let conn = open_memory_database();
        let created = create_orchestration_draft_record(
            &conn,
            CreateOrchestrationDraftCommandInput {
                title: "Persist orchestration draft".to_string(),
                folder_path: "C:/orchestrations/persist".to_string(),
                prompt: "Turn this migration handoff into an orchestration plan.".to_string(),
                files: vec![UploadedOrchestrationDraftFileInput {
                    id: "file-1".to_string(),
                    name: "handoff.md".to_string(),
                    size: 12,
                    last_modified: Some(1_788_888_000_000),
                }],
            },
        )
        .expect("create orchestration draft");
        let build_package_id = created["id"].as_str().unwrap().to_string();
        let stdout = completed_codex_stdout(
            "thread-plan-builder",
            "Draft plan output with orchestrationPlanDraft JSON.",
        );
        let runner = FakeCodexRunner::new(Ok(CodexCommandRunResult {
            stdout: stdout.clone(),
            stderr: String::new(),
            exit_code: Some(0),
            signal: None,
        }));

        let result = start_orchestration_plan_builder_run_with_runner(
            &conn,
            StartOrchestrationPlanBuilderRunCommandInput {
                build_package_id: build_package_id.clone(),
            },
            &runner,
        )
        .expect("start Plan Builder");

        assert_eq!(result["clientState"]["status"], "completed");
        assert_eq!(result["clientState"]["provenance"], "backend_response");
        assert_eq!(result["clientState"]["runtimeSupported"], true);
        assert_eq!(
            result["stages"][0]["state"],
            json!({ "status": "completed", "provenance": "backend_response" })
        );
        assert_eq!(
            result["stages"][1]["state"],
            json!({ "status": "ready", "provenance": "backend_response" })
        );
        assert_eq!(
            result["clientState"]["primaryAction"]["id"],
            "start-instantiation"
        );
        assert_eq!(result["stageRuns"].as_array().unwrap().len(), 1);
        let stage_run = &result["stageRuns"][0];
        assert_eq!(
            stage_run["state"],
            json!({ "status": "completed", "provenance": "backend_response" })
        );
        assert!(stage_run["promptArtifactId"].as_str().is_some());
        assert!(stage_run["rawEventArtifactId"].as_str().is_some());
        assert!(stage_run["outputArtifactId"].as_str().is_some());
        assert_eq!(stage_run["conversationId"].as_str().is_some(), true);
        assert_eq!(
            stage_run["evidence"]["externalThreadId"],
            "thread-plan-builder"
        );
        assert_eq!(stage_run["evidence"]["attachmentContentsSent"], false);
        assert_eq!(
            stage_run["evidence"]["attachmentMetadata"][0]["name"],
            "handoff.md"
        );
        assert_eq!(
            artifact_content_by_id(&conn, stage_run["rawEventArtifactId"].as_str().unwrap()),
            stdout
        );
        assert_eq!(
            artifact_content_by_id(&conn, stage_run["outputArtifactId"].as_str().unwrap()),
            "Draft plan output with orchestrationPlanDraft JSON."
        );
        assert!(result["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message["body"]
                .as_str()
                .is_some_and(|body| body.contains("orchestrationPlanDraft"))));
        assert_eq!(
            result["runtimeRoutes"][0]["runtimeCommand"],
            "startOrchestrationPlanBuilderRun"
        );

        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].command, "codex");
        assert_eq!(calls[0].args[0..2], ["exec", "--json"]);
        assert_eq!(calls[0].cwd, None);
        assert!(calls[0].args[2].contains("metadata-only"));
        assert!(calls[0].args[2].contains("handoff.md"));
        assert!(calls[0].args[2].contains("Do not instantiate"));

        let registry = load_orchestration_registry_snapshot(&conn).expect("registry snapshot");
        assert_eq!(
            registry["buildPackages"][0]["stageRuns"][0]["rawEventArtifactId"],
            stage_run["rawEventArtifactId"]
        );
        assert_eq!(
            registry["buildPackages"][0]["clientState"]["status"],
            "completed"
        );
    }

    #[test]
    fn instantiation_request_records_approval_without_generated_output() {
        let conn = open_memory_database();
        let created = create_orchestration_draft_record(
            &conn,
            CreateOrchestrationDraftCommandInput {
                title: "Persist orchestration draft".to_string(),
                folder_path: "C:/orchestrations/persist".to_string(),
                prompt: "Keep this prompt after reload.".to_string(),
                files: vec![],
            },
        )
        .expect("create orchestration draft");
        let build_package_id = created["id"].as_str().unwrap().to_string();

        let result = request_orchestration_build_stage_record(
            &conn,
            RequestOrchestrationBuildStageCommandInput {
                build_package_id,
                stage_id: "instantiator".to_string(),
            },
        )
        .expect("request instantiator");

        assert_eq!(
            result["stages"][1]["state"],
            json!({ "status": "completed", "provenance": "backend_response" })
        );
        assert_eq!(
            result["stages"][2]["state"],
            json!({ "status": "integration_pending", "provenance": "unsupported" })
        );
        assert!(result["clientState"]["notices"][0]["message"]
            .as_str()
            .unwrap()
            .contains("No files were generated"));
        assert!(result["stageRuns"].as_array().unwrap().is_empty());
        assert!(result["generatedFiles"].as_array().unwrap().is_empty());
    }

    #[test]
    fn start_orchestration_plan_builder_run_preserves_draft_when_codex_launch_fails() {
        let conn = open_memory_database();
        let created = create_orchestration_draft_record(
            &conn,
            CreateOrchestrationDraftCommandInput {
                title: "Persist orchestration draft".to_string(),
                folder_path: "C:/orchestrations/persist".to_string(),
                prompt: "Keep this prompt after launch failure.".to_string(),
                files: vec![UploadedOrchestrationDraftFileInput {
                    id: "file-1".to_string(),
                    name: "handoff.md".to_string(),
                    size: 12,
                    last_modified: None,
                }],
            },
        )
        .expect("create orchestration draft");
        let build_package_id = created["id"].as_str().unwrap().to_string();
        let runner = FakeCodexRunner::new(Err("Access is denied".to_string()));

        let result = start_orchestration_plan_builder_run_with_runner(
            &conn,
            StartOrchestrationPlanBuilderRunCommandInput {
                build_package_id: build_package_id.clone(),
            },
            &runner,
        )
        .expect("failed Plan Builder launch returns snapshot");

        assert_eq!(
            result["sourcePrompt"],
            "Keep this prompt after launch failure."
        );
        assert_eq!(result["files"].as_array().unwrap().len(), 1);
        assert_eq!(result["clientState"]["status"], "failed");
        assert_eq!(result["clientState"]["provenance"], "backend_response");
        assert_eq!(
            result["clientState"]["notices"][0]["message"]
                .as_str()
                .unwrap()
                .contains("Access is denied"),
            true
        );
        assert_eq!(
            result["stages"][0]["state"],
            json!({ "status": "failed", "provenance": "backend_response" })
        );
        let stage_run = &result["stageRuns"][0];
        assert_eq!(
            stage_run["state"],
            json!({ "status": "failed", "provenance": "backend_response" })
        );
        assert!(stage_run["promptArtifactId"].as_str().is_some());
        assert!(stage_run.get("rawEventArtifactId").is_none());
        assert!(stage_run.get("outputArtifactId").is_none());
        assert_eq!(stage_run["evidence"]["error"], "Access is denied");
        assert!(result["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message["body"]
                .as_str()
                .is_some_and(|body| body.contains("Access is denied"))));
    }

    #[test]
    fn cancel_orchestration_draft_removes_it_from_active_registry() {
        let conn = open_memory_database();
        let created = create_orchestration_draft_record(
            &conn,
            CreateOrchestrationDraftCommandInput {
                title: "Persist orchestration draft".to_string(),
                folder_path: "C:/orchestrations/persist".to_string(),
                prompt: "Keep this prompt after reload.".to_string(),
                files: vec![],
            },
        )
        .expect("create orchestration draft");
        let build_package_id = created["id"].as_str().unwrap().to_string();

        let registry =
            cancel_orchestration_draft_record(&conn, &build_package_id).expect("cancel draft");

        assert_eq!(registry["buildPackages"].as_array().unwrap().len(), 0);
        assert!(select_orchestration_draft_snapshot(&conn, &build_package_id).is_err());
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

    fn stage_run_evidence_record(
        build_package_id: &str,
        status: &str,
        provenance: &str,
        status_reason: Option<String>,
    ) -> OrchestrationStageRunEvidenceRecord {
        OrchestrationStageRunEvidenceRecord {
            id: "stage-run-1".to_string(),
            build_package_id: build_package_id.to_string(),
            stage_id: "plan-builder".to_string(),
            status: status.to_string(),
            provenance: provenance.to_string(),
            status_reason,
            prompt_artifact_id: Some("artifact-prompt-1".to_string()),
            output_artifact_id: Some("artifact-output-1".to_string()),
            raw_event_artifact_id: Some("artifact-events-1".to_string()),
            task_id: Some("task-1".to_string()),
            task_run_id: Some("task-run-1".to_string()),
            conversation_id: Some("conversation-1".to_string()),
            event_ids: vec!["event-1".to_string(), "event-2".to_string()],
            evidence: json!({
                "schema": "orchestration-stage-run-evidence/v1",
                "notes": "Test-only source-backed links; no generated files are implied.",
            }),
            started_at: Some(CREATED_AT.to_string()),
            completed_at: Some("2026-07-02T10:05:00.000Z".to_string()),
            created_at: CREATED_AT.to_string(),
            updated_at: "2026-07-02T10:05:00.000Z".to_string(),
        }
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

    fn artifact_content_by_id(conn: &Connection, artifact_id: &str) -> String {
        conn.query_row(
            "SELECT content FROM artifacts WHERE id = ?1",
            params![artifact_id],
            |row| row.get::<_, String>(0),
        )
        .expect("artifact content")
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
