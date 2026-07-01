# Codex Orchestrator Implementation Roadmap

Date: 2026-07-01

## Current Starting Point

The GitHub repository `andreas-kielsgaard/CodexOrchestrator` is currently empty. This roadmap treats the project as a greenfield local-first control plane for Codex.

The product goal is not to replace Codex. The goal is to build a customizable UI and workflow engine around Codex, Git worktrees, task state, conversations, and review/validation artifacts.

## Core Product Shape

The application should keep these concepts separate:

- `Task`: the unit of attention.
- `Project`: the human purpose boundary.
- `Repo`: the Git repository boundary.
- `Branch`: the code-change lineage.
- `Worktree`: the isolated execution surface.
- `Conversation`: reasoning or execution provenance.
- `TaskRun`: one attempt to do a task.
- `Artifact`: final response, diff, validation log, note, screenshot, handoff, or summary.

The dashboard should answer: "What needs my attention?"

The project/repo/branch drilldown should answer: "Where does this work live technically?"

## Guiding Decisions

1. Keep Codex as the execution engine.
2. Do not read or manage Codex credentials directly.
3. Use Git directly for repository, branch, and worktree facts.
4. Put all Codex integrations behind a `CodexRuntime` adapter.
5. Start with a local-first app and local database.
6. Make workflows data-driven from the beginning.
7. Store task/run history as events, then derive dashboard state.

## Proposed Stack

Initial recommendation:

- Desktop shell: Tauri v2.
- UI: React, TypeScript, Vite.
- UI primitives: Tailwind CSS plus a small component library such as shadcn/ui.
- Backend/runtime layer: Rust commands for filesystem/process-heavy operations, or a Node sidecar if TypeScript-first iteration is preferred.
- Database: SQLite with migrations.
- Terminal/process streaming: PTY support for interactive logs where needed.
- Validation and parsing: structured adapters around Git and Codex output.

The stack can be revisited after the first spike. The important boundary is more important than the shell: UI should not directly own Git/Codex process logic.

## Phase 0: Bootstrap And Technical Spikes

Goal: create the project skeleton and prove the riskiest integration points before building product UI.

Deliverables:

- Create the monorepo/app structure.
- Add formatting, linting, testing, and basic CI.
- Add a minimal SQLite migration setup.
- Add a local settings file strategy.
- Spike `git worktree list --porcelain -z` parsing.
- Spike a single `codex exec --json` run in a throwaway Git repo.
- Spike the Codex SDK or app-server event stream.
- Document which Codex adapter should be MVP-first.

Acceptance checks:

- A developer can install dependencies and run the empty app.
- The app can inspect a Git repo path and return normalized repo/worktree facts.
- A test script can run Codex in JSONL mode and persist events.
- The project has a decision record for `exec` vs SDK/app-server MVP integration.

## Phase 1: Domain Model And Persistence

Goal: define durable state before UI complexity arrives.

Deliverables:

- Database schema and migrations for:
  - `projects`
  - `repos`
  - `branches`
  - `worktrees`
  - `tasks`
  - `task_runs`
  - `conversations`
  - `artifacts`
  - `validation_runs`
  - `events`
- Type-safe domain models.
- Repository layer for CRUD and query operations.
- Seed/demo data generator.
- Event append API.
- Derived query API for dashboards and drilldowns.

Acceptance checks:

- Tasks can link to project/repo/branch/worktree/conversation records.
- Execution state and attention state are stored separately.
- Events can reconstruct the last known state for a task run.
- Demo data can populate the dashboard without Codex running.

## Phase 2: Git And Worktree Manager

Goal: make Git state first-class and reliable.

Deliverables:

- Repo registry: add/remove/list repos under projects.
- Repo scanner:
  - root path
  - remotes
  - default branch
  - local branches
  - current branch
  - status snapshot
  - worktree list
- Branch registry sync.
- Worktree create/archive/remove flows.
- Worktree lock/reason support for app-managed worktrees.
- Dirty/missing/prunable state detection.
- Windows path normalization.

Acceptance checks:

- The app can scan a repo and list active branches/worktrees.
- The app can create a new branch in a new worktree for a task.
- The app does not assume multiple active worktrees can normally check out the same branch.
- The UI can group worktrees under branches while preserving Git-native repo -> worktree facts.

## Phase 3: Task Dashboard MVP

Goal: build the first useful control-plane screen before deep Codex control.

Deliverables:

- Open Tasks dashboard grouped by:
  - Needs action now
  - Review / decide
  - Working
  - Waiting
  - Later
- Task create/edit/archive.
- Attention state transitions:
  - `needs_action_now`
  - `needs_review`
  - `waiting_on_agent`
  - `waiting_on_external`
  - `consider_later`
  - `snoozed`
  - `reference_only`
- Execution state transitions:
  - `draft`
  - `queued`
  - `running`
  - `blocked`
  - `completed`
  - `failed`
  - `abandoned`
  - `archived`
- Link task to project/repo/branch/worktree/conversation.
- Filtering, search, and basic sorting.

Acceptance checks:

- A task can be open even when no Codex process is running.
- A completed task can still require review.
- A running task can be marked as waiting on agent.
- The dashboard can be driven entirely by persisted state.

## Phase 4: Project, Repo, And Branch Drilldowns

Goal: add the technical/provenance view that complements the attention dashboard.

Deliverables:

- Project detail:
  - repos
  - open tasks
  - recent conversations
  - active/stale branches
- Repo detail:
  - branches
  - worktrees
  - dirty states
  - recent validation failures
  - unlinked sessions or artifacts
- Branch detail:
  - intent summary
  - linked tasks
  - active worktrees
  - conversations
  - current diff
  - commits
  - validation history
- Task detail:
  - intent
  - linked technical anchors
  - runs
  - conversations
  - artifacts
  - attention history

Acceptance checks:

- The same task appears naturally in dashboard and drilldown views.
- Branch pages show why a branch exists, not just its Git facts.
- Worktree and conversation records can exist before or after being linked to tasks.

## Phase 5: Codex Runtime Adapter MVP

Goal: start and observe Codex runs through a narrow interface.

Initial adapter interface:

```ts
interface CodexRuntime {
  startRun(input: StartRunInput): AsyncIterable<CodexEvent>;
  resumeRun(input: ResumeRunInput): AsyncIterable<CodexEvent>;
  interruptRun(input: InterruptRunInput): Promise<void>;
}
```

MVP implementation:

- Prefer `codex exec --json` for the first durable run adapter.
- Store JSONL event streams as raw artifacts.
- Normalize event types into internal events.
- Capture final response.
- Capture thread/session IDs when emitted.
- Capture command executions, file changes, plan updates, failures, and token usage when available.
- Support `--output-schema` for structured run summaries.

Acceptance checks:

- A user can start a Codex run for a task in a chosen worktree.
- The task moves to `running` and `waiting_on_agent`.
- On completion with changes, the task moves to `completed` and `needs_review`.
- On failure, the task moves to `failed` and `needs_action_now`.
- Raw Codex output is preserved for debugging.

## Phase 6: Rich Codex Control

Goal: support interactive sessions, approvals, steering, and live state.

Deliverables:

- Add SDK/app-server adapter behind the same runtime boundary.
- Generate version-pinned app-server schemas during development.
- Thread start/resume/list support.
- Turn start/stream support.
- Approval request detection.
- Accept/decline/cancel approval actions.
- Interrupt active turn.
- Follow-up/steer active or completed turns.
- Conversation transcript store.

Acceptance checks:

- Approval requests appear in `Needs action now`.
- The user can approve or decline from the orchestrator UI.
- The user can steer a run without losing task/run linkage.
- App-server protocol changes are isolated to the adapter package.

## Phase 7: Workflow Engine And Customization

Goal: make the app configurable enough to support personal orchestration styles.

Deliverables:

- Workflow definition format.
- Prompt templates.
- Branch naming templates.
- Worktree strategies:
  - use existing worktree
  - create new worktree
  - run in project root
  - manual selection
- Codex profile/sandbox/approval policy selection.
- Preflight commands.
- Post-run validation commands.
- Attention-state transition rules.
- Cleanup policies.
- Workflow library:
  - plan only
  - implement small feature
  - review diff
  - fix validation failure
  - summarize branch
  - prepare PR
  - continue existing branch

Acceptance checks:

- A workflow can be edited without changing application code.
- A workflow can create a branch/worktree, start Codex, run validation, and update attention state.
- Workflow outcomes are visible as events and artifacts.

## Phase 8: Validation, Diffs, And Review Surface

Goal: turn stopped Codex runs into actionable review work.

Deliverables:

- Diff collector.
- File change summary.
- Validation command runner.
- Validation history.
- Test/lint result artifacts.
- Review queue.
- Commit helper.
- PR preparation helper.
- Risk/next-action summary generated from structured Codex output where useful.

Acceptance checks:

- Completed runs with diffs land in review state.
- Failed validation moves the task to `needs_action_now`.
- The user can see diff, final message, and validation logs in one task view.

## Phase 9: ChatGPT Export Import

Goal: support conversation management without relying on private ChatGPT internals.

Deliverables:

- Import official ChatGPT data export ZIP.
- Parse conversations into local records.
- Full-text search.
- Link imported conversations to projects/repos/tasks/branches.
- Generate summaries and inferred links.

Acceptance checks:

- Imported ChatGPT conversations are searchable.
- Imported conversations can be linked to tasks and branches.
- The feature is clearly archive/search/linking, not live ChatGPT management.

## Phase 10: Notifications, Remote Views, And Automations

Goal: make open work easier to monitor over time.

Deliverables:

- Local notifications for:
  - approval needed
  - run completed
  - validation failed
  - review due
- Snooze and next-review scheduling.
- Optional local web UI.
- Optional remote/pairing bridge.
- Recurring/scheduled workflows.
- Automation inbox.

Acceptance checks:

- The user can leave runs unattended and return to a useful attention queue.
- Scheduled workflows produce normal tasks/runs/artifacts.
- Remote access never exposes Codex credentials directly.

## Phase 11: Hardening And Recovery

Goal: make the system trustworthy under crashes, interrupted runs, and Git messiness.

Deliverables:

- Process supervisor.
- Startup reconciliation:
  - running process missing
  - worktree missing
  - branch deleted
  - dirty worktree
  - stale lock
  - orphaned Codex session
- Error boundaries and retry paths.
- Backups/export of local database.
- Audit log.
- Security review.

Acceptance checks:

- Restarting the app does not lose task/run history.
- Orphaned worktrees and sessions can be re-linked or archived.
- Credentials remain owned by Codex, not the orchestrator.

## External References To Inspect During Implementation

Official Codex docs:

- Codex SDK: `https://developers.openai.com/codex/sdk`
- Codex app-server: `https://developers.openai.com/codex/app-server`
- Codex non-interactive mode: `https://developers.openai.com/codex/noninteractive`
- Codex CLI reference: `https://developers.openai.com/codex/cli/reference`
- Codex authentication: `https://developers.openai.com/codex/auth`
- Codex configuration: `https://developers.openai.com/codex/config-basic`

Open-source projects to mine for patterns:

- CodexFlow: desktop UI, session organization, worktree workflows, Windows/WSL handling.
- Codexia: backend control-plane patterns, scheduler, worktrees, app-server integration. Check license before reuse.
- Jean: Tauri/React desktop architecture for projects, worktrees, sessions, terminals, editor panels.
- Remodex: local-first remote bridge, attention notifications, steering/follow-up patterns.
- CodexMonitor: app-server-over-stdio patterns and workspace/thread reconciliation.

## First Build Slice

The first implementation slice should be:

1. Bootstrap the app skeleton.
2. Add SQLite persistence and domain models.
3. Add repo registry and Git scanner.
4. Add task dashboard with demo data.
5. Add worktree creation for a task.
6. Add `codex exec --json` run adapter.
7. Show run events, final output, diff state, and attention-state transitions.

This slice produces a real local dashboard even before rich interactive Codex control is added.
