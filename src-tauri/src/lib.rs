use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::HashSet, fs, path::PathBuf};
use tauri::{AppHandle, Manager};
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
    title: String,
    summary: String,
    execution_state: Option<String>,
    attention_state: Option<String>,
    priority: Option<String>,
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

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskDashboardSnapshot {
    groups: Vec<DashboardGroup>,
    projects: Vec<TaskDashboardProject>,
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
    name: String,
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
        codex_runtime: "adapter-pending",
    }
}

#[tauri::command]
fn load_open_task_dashboard(app: AppHandle) -> Result<TaskDashboardSnapshot, String> {
    with_app_database(&app, |conn| load_dashboard_snapshot(conn))
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_metadata,
            load_open_task_dashboard,
            create_open_task,
            update_open_task,
            archive_open_task,
            load_task_run_detail
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

    conn.execute(
        "
INSERT INTO tasks (
  id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, due_at, snoozed_until, created_at, updated_at
) VALUES (?1, ?2, NULL, NULL, NULL, ?3, ?4, ?5, ?6, ?7, NULL, NULL, ?8, ?8)
",
        params![
            task_id,
            input.project_id,
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
        .prepare("SELECT id, name FROM repos ORDER BY id")
        .map_err(sql_error("prepare repos query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(RepoRow {
                id: row.get(0)?,
                name: row.get(1)?,
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

fn sql_error(context: &str) -> impl FnOnce(rusqlite::Error) -> String + '_ {
    move |error| format!("Unable to {context}: {error}")
}

struct Migration {
    id: &'static str,
    sql: &'static str,
}

fn app_migrations() -> [Migration; 5] {
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

    const CREATED_AT: &str = "2026-07-02T10:00:00.000Z";

    #[test]
    fn load_dashboard_returns_empty_snapshot_for_new_database() {
        let conn = open_memory_database();

        let snapshot = load_dashboard_snapshot(&conn).expect("snapshot");

        assert_eq!(snapshot.total_open_tasks, 0);
        assert_eq!(snapshot.projects, Vec::<TaskDashboardProject>::new());
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

    fn open_memory_database() -> Connection {
        let conn = Connection::open_in_memory().expect("memory database");
        initialize_database(&conn).expect("initialize database");
        conn
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
