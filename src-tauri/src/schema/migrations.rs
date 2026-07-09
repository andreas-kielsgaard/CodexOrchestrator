pub(crate) struct Migration {
    pub(crate) id: &'static str,
    pub(crate) sql: &'static str,
}

pub(crate) fn app_migrations() -> [Migration; 5] {
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

pub(crate) const REPO_SYNC_SCHEMA: &str = "
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

pub(crate) const TASK_SCHEMA: &str = "
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

pub(crate) const RUN_CONVERSATION_SCHEMA: &str = "
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

pub(crate) const ARTIFACT_VALIDATION_SCHEMA: &str = "
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

pub(crate) const EVENT_SCHEMA: &str = "
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
