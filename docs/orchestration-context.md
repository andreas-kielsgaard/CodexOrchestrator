# Codex Orchestrator Control Context

## Purpose

This repository is building a local-first control plane for Codex-driven work. The orchestration
thread acts as the control room: choose the next useful slice, delegate focused worker tasks,
review results, merge accepted work, keep state recoverable, and prevent product drift.

As an orchestrator, your task is to

1: Keep track of the overall implementation state of the project
2: Identify what work slice(s) are ready to be executed
3: Delegate completion of work slice ready to be executed to other agents.
4: When an agent reports work slice complete, review the work and determine next slice
5: When more work slices are ready to be started, delegate completion of work slice ready to be executed to other agents.
6: If the review finds that corrections are required, delegate corrections to the worker that is to be corrected
7: If merge is required before starting next slice, perform merge. Otherwise delegate merging to the original worker.

When delegating work slice to another agent, make sure to provide clear instructions that when the work slice is completed, they should report back to you, the orchestrator conversation, so that you know when to continue with step 4.

The project you are orchestrating is outline in  `docs/first-slice-completion-plan.md`, `docs/implementation-roadmap.md

Work that has been started and not finished yet, should be outlined in `docs/active-task-map.md`.

If there is a non-trivial blocker, start a parallel work slice investigating how to resovle. If resolution is not clear or requires human input, pause orchestration process  and ask for human support (allow running parallel tasks to finish).

### Delegate Clearly

When delegating a task slice consider including

- task goal
- context about non-trivial boundaries
- context critical to intelligent completion of the task
- targeted branch/worktree
- targeted result log under `docs/task-logs/`
- proposed verification commands
- requirement to commit the completed slice unless blocked or exploratory
- a dedicated "Report back" instruction telling the worker to message the orchestration thread when
  complete

Workers should receive enough context to act well, not a transcript dump.
Only provide the worker with the context required to do the targeted unit of work.
Do not provide the worker with orchestration relevant context.
Workers should always be celarly instructed to notify the orchestrator when their work slice is completed, or when they encounter blockers that are not trivial to solve.

### Review Before Merge

- Inspect worker output, result log, and branch state before merging.
- Run independent verification appropriate to the change.
- Make small orchestrator corrections on the worker branch when that is faster and clearer than
  relaunching.
- Merge only accepted work.
- After merge, verify on `main`, update logs/maps, then clean branch/worktree state when safe.
- Leave worker/admin chats visible unless the user explicitly asks to archive them.

## Worker Completion Contract

Workers should at minimum report a brief summary of the work they completed. Ask for any additional
details needed for post-work verification in the worker prompt.

## Judgment Cues

- Use the lowest reasoning effort that fits the risk: low for mechanical work, medium for normal
  slices, high for architecture/runtime/security, xhigh only for unusually tangled failures or major
  redesign.
- Keep trivial orchestration work in the main thread when it is faster than launching a worker:
  branch/status checks, small doc updates, active-task-map edits, merge notes, and simple follow-up
  prompts.
- Split out admin or review workers only when the work is parallelizable and nontrivial, such as
  messy merge repair, broad verification, or state reconciliation across several branches.
- For UI-heavy slices, decide before launch whether the work needs reusable components, isolated
  review, or a separate usability pass.
- If conceptually separate work can be done independently in parallel, you may launch several
  workers in parallel.
