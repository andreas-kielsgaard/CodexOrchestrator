use super::*;

pub(crate) struct CodexCommandRunInput {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) cwd: Option<String>,
    pub(crate) env: Option<BTreeMap<String, Option<String>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodexCommandRunResult {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) exit_code: Option<i64>,
    pub(crate) signal: Option<String>,
}

pub(crate) trait CodexCommandRunner {
    fn run(&self, input: CodexCommandRunInput) -> Result<CodexCommandRunResult, String>;
}

pub(crate) struct SystemCodexCommandRunner;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitDiffRunInput {
    pub(crate) worktree_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitDiffRunResult {
    pub(crate) diff: String,
}

pub(crate) trait GitDiffRunner {
    fn collect_tracked_diff(&self, input: GitDiffRunInput) -> Result<GitDiffRunResult, String>;
}

pub(crate) struct SystemGitDiffRunner;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidationCommandRunInput {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) cwd: String,
    pub(crate) env: Option<BTreeMap<String, Option<String>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidationCommandRunResult {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) exit_code: Option<i64>,
    pub(crate) signal: Option<String>,
}

pub(crate) trait ValidationCommandRunner {
    fn run(&self, input: ValidationCommandRunInput) -> Result<ValidationCommandRunResult, String>;
}

pub(crate) struct SystemValidationCommandRunner;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedPostRunWorktreePath {
    pub(crate) path: String,
    pub(crate) worktree_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StartedCodexTaskRun {
    pub(crate) task_id: String,
    pub(crate) project_id: String,
    pub(crate) task_run_id: String,
    pub(crate) conversation_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CodexRuntimeStatus {
    Completed,
    Failed,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CodexRuntimeResult {
    pub(crate) exit_code: Option<i64>,
    pub(crate) signal: Option<String>,
    pub(crate) status: CodexRuntimeStatus,
    pub(crate) status_reason: String,
    pub(crate) stdout_jsonl: String,
    pub(crate) stderr: String,
    pub(crate) summary: CodexJsonlSummary,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CodexJsonlSummary {
    pub(crate) thread_id: Option<String>,
    pub(crate) final_agent_message_text: Option<String>,
    pub(crate) terminal_status: Option<CodexJsonlTerminalStatus>,
    pub(crate) token_usage: Option<Map<String, Value>>,
    pub(crate) item_counts_by_type: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CodexJsonlTerminalStatus {
    Completed { line_number: usize },
    Failed { line_number: usize },
    Error { line_number: usize },
}
