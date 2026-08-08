---
name: retire-plan-slice
description: Retire storage retained by an accepted ad-hoc Plan Slice. Use in the Overall Plan conversation after accepting a published Slice result, or before launching new isolated work when accepted routes are consuming needed local storage.
---

# Retire Plan Slice

Reclaim accepted Slice storage without weakening provenance or stranding active work.

## Establish the retirement set

Start from the accepted Slice report, its retirement manifest, and current host and repository evidence. Identify each distinct Slice and Plan Step route and the task that owns it.

For every route, verify its resolved absolute path, current task activity, owned processes, Git state including untracked files, exact retained checkpoint, publication or other durable reachability, and any reason its local state remains necessary. An idle, unloaded, archived, or clean task is not by itself evidence that its route can be released.

Classify the route as retained, generated-state reclaimable, or worktree releasable. Preserve a route when it contains dirty or unpublished work, an unevaluated return, an active correction, retained runtime evidence, or an analogous unresolved ownership or provenance need. Unknown state remains retained.

## Reclaim safely

Measure free storage before and after the operation. Resolve and verify every deletion target before issuing a destructive command.

Reclaim reproducible generated state first, such as a task-owned Cargo target, disposable dependency installation, or validation output. Prefer the producer's scoped cleanup command when available. Stop only processes owned by a completed route and only when their retained runtime evidence is no longer needed.

A retained route does not retain its reproducible build copies. Reclaim superseded candidate or validation output inside an otherwise retained route when continuation and evidence no longer depend on it.

Release a registered worktree only when its task is inactive, its accepted checkpoint is durably reachable, its route is clean, and the host evidences that release will not strand the task. Prefer a harness-supported release. If that boundary is unavailable, retain the worktree and reclaim only its proven disposable artifacts.

Treat an unregistered directory as an orphan candidate rather than disposable state. Remove it only when exact task ownership, durable retention, inactivity, and absence of required evidence are established.

Keep saved checkouts, dirty or unpublished work, branches, remote refs, databases, recordings, credentials, and non-reproducible evidence intact. Avoid force deletion when ordinary lifecycle cleanup cannot establish safety.

## Return the result

Update the Overall Plan's retained-route ledger with what was reclaimed, what remains, each retention reason, and measured storage change. Keep exact deletion detail in this operation rather than later Slice briefs.
