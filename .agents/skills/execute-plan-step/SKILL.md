---
name: execute-plan-step
description: Execute one fixed Plan Step in an ad-hoc Codex Initiative. Use in a separately addressable Plan Step conversation after a Plan Slice conversation assigns a bounded implementation, exploration, review, integration, verification, documentation, or similar outcome.
---

# Execute Plan Step

Perform the fixed assignment and prepare the evidence needed for its evaluation.

## Work the assignment

Inspect the sources and current state needed to verify the premise. Perform the complete assigned work within its scope, authority, repository or worktree route, and acceptance criteria. Preserve unrelated work and make local implementation decisions that do not change the assigned outcome.

For setup, navigation, inspection, testing, lifecycle control, or similar work against software under development that might use desktop automation, apply `$agent-interface-first` before selecting tools. Treat bounded task-support tooling for an authorized operation as part of the assignment unless a hard boundary excludes it, and implement that interface before using desktop control.

Keep working while the remaining work is in scope and can advance safely. Settle each command or check started during the activation; resume a yielded operation until it completes, fails, or produces a concrete timeout or error that changes the disposition.

When completion is not ready, prepare an evaluable partial result only at a meaningful retained checkpoint or evidence boundary. Include the exact remaining work and next execution entry point. Use a blocker or clarification request when continuing requires a consequential scope, authority, or decision change.

Use validation as implementation feedback rather than only as a final gate. Run the earliest relevant check once a risky boundary is executable, and use its result before building dependent behavior.

Use actual prompts through the Codex Orchestrator product when they are relevant to the assigned evidence boundary. No additional user confirmation is required for bounded development and verification prompts. Keep prompt submission, provider receipt or activity, MCP exposure and result, application processing, downstream activation, durable state, and acceptance distinct according to what can actually be observed. This authority does not extend the Plan Step's scope or authorize unrelated external effects.

Choose proportionate commands, tests, checks, and validation methods according to the distinct evidence each adds. Prefer the smallest sequence that closes local acceptance. Treat a successful build-and-run command as compilation evidence for the targets it compiled; add another check, build, or broader test when it covers a different target, feature, configuration, integration boundary, or identified risk.

Run build-system commands serially when they share output or cache locks. Let a selected long-running command settle through the host's wait or continuation mechanism. Use its final result for the next decision rather than starting an overlapping substitute, treating quiet output as failure, or repeatedly replanning from partial output. Independent work may continue while it runs when that work is already in scope and does not depend on the pending result.

Treat validation and runtime output as task-owned working state wherever it is created. Prefer deterministic task-owned locations for substantial temporary repositories, databases, isolated build outputs, and analogous fixtures. Establish cleanup before starting the operation, including failure, timeout, resumed interruption, or similar abnormal exits. On each activation, reconcile owned leftovers before creating replacements. Keep only the reusable cache or retained evidence needed for continuation, and reclaim superseded reproducible copies before returning.

Validation cost does not reduce the assignment's implementation, deliverables, or local acceptance boundary. Report a genuinely unavailable or incomplete gate at its exact evidence level.

When acceptance depends on durable state, reopen behavior, recovery, or similar lifecycle facts, establish representative state through the productive persistence or materialization path early enough to guide implementation. Exercise fresh-open behavior once that state exists. Synthetic fixtures and private helpers may support the work, but they are not the sole authority for productive behavior.

When the assignment spans a producer-consumer contract, exercise the first real consumer once the boundary is runnable. Use source-shape or isolated producer checks as supporting evidence rather than postponing all downstream evidence until the implementation appears finished.

When the premise is stale or the outcome cannot be completed safely, return the concrete blocker, evidence, and smallest decision or correction needed rather than expanding scope.

## Commit completed changes

When the assignment changes repository state, inspect and attribute the final diff, stage only the Plan Step's changes, and commit them after validation with a concise step-scoped message. Preserve unrelated dirty state exactly. Do not push, merge, rebase, or perform unrelated repository integration.

Treat the commit as the completed work checkpoint returned for evaluation, not as acceptance. If the assignment produces no repository change, create no empty commit. If a safe attributable commit cannot be made, the Plan Step is not complete.

## Return the result

Report the disposition, outcome, artifacts or files changed, validation and results, commit identity and message or explicit no-change result, post-commit repository state, task-owned generated state or processes that remain including their exact external paths, risks, residuals, and unproven boundaries. For a partial result, report the retained state and exact continuation point. Produce this return only after ongoing work and started operations have reached the chosen disposition boundary. Distinguish requested, performed, observed, committed, and accepted facts.
