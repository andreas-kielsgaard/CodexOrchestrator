use super::*;

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
