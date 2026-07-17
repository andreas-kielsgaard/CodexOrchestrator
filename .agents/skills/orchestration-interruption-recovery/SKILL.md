---
name: orchestration-interruption-recovery
description: Pause, stop, resume, or recover an orchestration run from the root orchestrator. Use as a subagent of the root orchestrator when the user needs to shut down, pause, loses connection, runs out of context, restarts after an unplanned interruption, or needs running tasks and records wrangled back into a coherent state.
---

# Orchestration Interruption Recovery

## Role

Run as a subagent of the root orchestrator. Handle interruption mechanics so the root thread does not fill with noisy thread checks, worker prompts, or recovery scans.

Support two modes:

- `pause`: user intentionally pauses or shuts down.
- `resume`: user restarts after an intentional pause, forced disconnect, token exhaustion, app restart, or unknown stoppage.

Do not perform normal planning or implementation. Return a concise recovery summary and the next root action.

## Inputs

Expect:

- mode: `pause` or `resume`
- orchestration home path, or repo locator path if home must be rediscovered
- root record thread id or prompt path, if known
- active worker, delegation, planner, or record-maintainer thread ids, plus current delegation stage, if known
- user instruction: pause, stop, resume, recover, inspect, or restart work

If orchestration home is unknown, first try the current root context. If running inside a participating repo, read only `.codex-orchestrator/orchestration-link.json` or the configured locator to rediscover it.

## Pause Mode

When pausing:

1. Identify active orchestration threads: root, record root, delegation, worker roots, planner forks, and record-maintainer threads, plus any in-progress delegation stages such as review, merge, reconciliation, reporting, or planner notification.
2. Use available thread-management tools when present; otherwise rely on visible thread references, records, and prompts.
3. Ask running workers to stop at a safe boundary and report current status when possible.
4. Do not cancel or kill work unless the user explicitly asked to stop/cancel.
5. Create or update `<orchestration-home>/stoppage.md`.
6. Ask or prompt the record root to record the pause state if it can be reached.
7. Return only the pause summary to root.

Record unknown thread state as unknown rather than inventing certainty.

## Resume Mode

When resuming:

1. Rediscover the orchestration home.
2. Read `<orchestration-home>/stoppage.md` if present.
3. If no stoppage file exists, inspect available thread state, root records, sub-agent context, repo locators, and recent reports to reconstruct what was active.
4. Classify each active or possibly active item as complete, still running, paused safely, needs follow-up, stale, failed, or unknown.
5. Identify work that should be restarted, work that should be left alone, and work that requires human choice.
6. Produce prompts needed to restart or re-contact worker/delegation threads or continue an interrupted delegation stage.
7. Clean up the stoppage file after the root accepts the recovery plan: delete it, move it to an archive, or mark it resolved according to local convention.

If cleanup would hide uncertainty, keep the stoppage file and mark the unresolved items clearly.

## Stoppage File

Use this path:

```text
<orchestration-home>/stoppage.md
```

Include:

- timestamp and reason
- pause/resume mode
- root orchestrator thread id, if known
- root record thread id, if known
- orchestration home
- active thread inventory
- known running or paused work
- prompts sent to workers, if any
- unresolved unknowns
- recommended resume procedure
- cleanup status

Keep it compact. It is a recovery anchor, not a full archive.

## Reasoning Guidance

Use medium reasoning for normal pause/resume. Use high reasoning when stoppage was unplanned, thread state conflicts, multiple workers may have progressed independently, or restarting the wrong work could cause duplicate/conflicting changes.

When this recovery helper is started through thread tooling, the launcher should request the chosen reasoning level as launch metadata and omit model overrides unless the human explicitly requested a model.

## Output Contract

Return:

- mode handled
- orchestration home used
- stoppage file status
- active thread/work inventory
- actions taken
- unresolved unknowns
- restart prompts or follow-up prompts
- recommended next root action
- whether stoppage cleanup is complete or still pending
