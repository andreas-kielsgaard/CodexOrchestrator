use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_metadata,
            load_open_task_dashboard,
            create_open_task,
            update_open_task,
            archive_open_task
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

fn open_task_not_found(task_id: &str) -> String {
    format!("Open task not found: {task_id}")
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
}
