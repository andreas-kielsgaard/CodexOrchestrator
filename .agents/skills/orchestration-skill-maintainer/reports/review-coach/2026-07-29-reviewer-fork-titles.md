# Reviewer Fork Titles

## Observation

Acceptance-review forks inherit the main task title `Coach: Orchestrator review`. Current reviewer tasks for unrelated corrections therefore appear indistinguishable in the task list even though each owns a different bounded review.

## Theory

The reviewer contract defined ownership, evidence, callbacks, and continuation but did not assign responsibility for the fork's user-visible title. The desktop harness preserves the parent title when forking, so the missing instruction reliably produces generic duplicate names.

## Revision

The main coach now passes a bounded review subject when creating the reviewer fork. When the harness exposes title control, the reviewer renames its current task on entry to a concise subject-specific title, such as `Review: File/diff correction` or a similarly recognizable description. The current desktop harness supports this through self-targeting `set_thread_title`.

## Evaluation

This is a small, low-risk use of an existing harness capability. It improves task-list orientation without changing review ownership, callback routing, or lifecycle behavior. The conditional wording preserves compatibility with harnesses that cannot rename tasks.

The target is the general Codex `review-coach` skill. Existing running reviewer tasks remain unchanged by skill maintenance.
