# Product-Orchestrated Codex Model

## Core Shift

Earlier orchestration skills assumed that a few long-running conversations would manage the orchestration loop by remembering topology, delegating to subagents, and reporting between semantic thread roles.

The product model moves that responsibility into Codex Orchestrator:

- The product owns the graph.
- The product owns lifecycle state.
- The product owns prompts as artifacts.
- The product owns routing.
- The product owns records and projections.
- Codex conversations perform bounded reasoning or execution stages.

This removes the need for root conversations to remember the whole process and reduces stalls caused by closed helper threads, missed callbacks, context compression, or ambiguous parent/child semantics.

## Execution Unit

A Codex conversation is an execution resource. It can be:

- a planner turn
- a worker turn or worker thread
- a review stage
- a merge/reconciliation stage
- a reporting stage
- a record/projection maintenance stage
- an intake/summarization stage

The product decides whether to create a new thread, resume a thread, fork a thread, or use a non-interactive run. The prompt packet should contain the context needed for that execution unit to succeed.

## Future Token-Economics Optimization

Treat this section as a hypothesis to test before changing live skills.

OpenAI pricing and telemetry make multi-run orchestration potentially expensive in two places:

- repeated turns may reprocess large cached or uncached input prefixes
- output tokens are comparatively expensive, especially when each stage writes a full narrative report

This suggests a future routing preference:

1. Prefer one sequential Codex execution when a stage chain is unlikely to exceed the context window.
2. Keep planner decisions, worker execution, and post-worker integration separable when they genuinely need different context or timing.
3. After a worker finishes, consider handling review, ordinary merge/reconciliation, report generation, and record-update packet creation in one continuation prompt when the evidence is compact and the repository state is straightforward.
4. Use product-side waiting and monitoring while a worker is running instead of model turns that only say "still waiting."
5. Reuse a conversation for adjacent sequential stages only when the retained context is valuable enough to justify the larger input footprint.
6. Start detached conversations when context isolation or parallelism is worth the additional prompt/output overhead.

Idle conversation cost is an open question. Do not assume that a visible but inactive Codex conversation keeps billable model memory alive, and do not assume it is free in product terms. Measure it through available Codex usage telemetry, `/status`, dashboard changes, and app-server event streams before optimizing around it.

The product controller should eventually estimate stage-chain cost before launch:

- expected input size
- expected output size
- cached input observed from prior turns
- context-window risk
- need for tool waits
- value of context continuity
- value of parallelism

This may lead to a "single-pass post-worker chain" route for small slices and a "split stages" route for larger, risky, or semantically distinct slices.

## Product-Owned Graph

Use a graph like:

```text
orchestration
  plan/problem map
  planner turn
    planner decision
    work slice
      delegation prompt packet
      worker conversation/turn
      review stage
      correction stage
      merge/reconciliation stage
      report stage
      record update stage
  record/projection entries
  human decision requests
```

Thread ids are attributes on graph nodes, not the graph itself.

## Structured Plan Artifacts

The plan-builder should produce a semantic `orchestrationPlanDraft` object. The instantiator should normalize it into product files, especially:

- `orchestration-plan.json`: the nested proposed problem/stage map.
- `orchestration-live-state.json`: the current projection over active lifecycle nodes, completed nodes, planner turns, work slices, record updates, and blocker ids that need product attention.
- `orchestration-blockers.json`: user-addressable blockers and their conclusions.
- `record-seed.json`: the structured seed for maintained records and refresh projections.

Use nested plan nodes for large-scale problems, stages, and sub-stages. A node can represent "Field Platform migration", "Pinned install implementation", or "Preserve Field-local adapter and tools". Nodes are not work slices by default. Planner turns may update node state, add sub-nodes, and attach work-slice records as execution reveals the real path.

Recommended `orchestration-plan.json` top level:

```json
{
  "schemaVersion": 1,
  "orchestrationId": "plan-slug",
  "title": "Human-readable title",
  "objective": "Target outcome",
  "scope": {
    "changeTargets": [],
    "readOnlyContext": [],
    "outOfScope": []
  },
  "planRoot": {
    "id": "plan-root",
    "title": "Overall problem",
    "kind": "objective",
    "summary": "What this node solves",
    "status": "proposed",
    "repoRouting": [],
    "dependencies": [],
    "decisionGates": [],
    "validationConcerns": [],
    "blockerIds": [],
    "children": []
  },
  "plannerAuthority": {
    "mayModifyPlanNodes": true,
    "mayCreateSubNodes": true,
    "mayAttachWorkSlices": true
  }
}
```

## Lifecycle States

Use states that describe product-owned progress:

- `draft`
- `ready`
- `queued`
- `running`
- `waiting-on-codex`
- `waiting-on-tool`
- `waiting-on-human`
- `accepted`
- `needs-correction`
- `merged`
- `reported`
- `recorded`
- `settled`
- `failed`
- `paused`
- `abandoned`

Conversation output should propose or supply the next state; the controller applies it.

## Prompt Artifacts

Every stage prompt should be an artifact. The artifact is useful for:

- reproducibility
- UI inspection
- retry
- audit
- comparison across skill revisions
- detached conversation startup

Prompt artifacts should be structured enough for deterministic routing. Suggested fields:

```text
orchestrationId
stageId
role
targetRuntime
targetConversationId
sourceNodeIds
destinationNodeId
repoRoute
inlineContext
contextRefs
taskOrDecision
acceptanceCriteria
expectedOutput
allowedActions
verification
```

## Stage Outputs

Stage outputs should return structured conclusions, not broad conversational summaries.

Examples:

- planner: decisions, selected work slices, parked items, prompt packets, human decision requests
- worker: implementation summary, changed files, validation, review payload
- review: accepted, correction prompt, sign-off, or merge route
- merge: repository integration result, commit/ref/status, validation
- reporter: completion report artifact and record update packet
- maintainer: projection updates and root/planner-visible deltas

## Product Blockers

A blocker is a product-owned decision or missing input that the user can resolve through the UI. Keep blockers separate from plan prose and lifecycle summaries so the product can link them to plan nodes, work slices, and the next planner prompt.

Recommended `orchestration-blockers.json` top level:

```json
{
  "schemaVersion": 1,
  "orchestrationId": "plan-slug",
  "blockers": [
    {
      "id": "blocker-legacy-treatment",
      "title": "Legacy Agent OS treatment",
      "state": "open",
      "severity": "high",
      "summary": "Decide how to handle the old downstream Agent OS folder after pinned install is proven.",
      "detail": "The old downstream material needs a product disposition before cleanup work can be finalized.",
      "resolutionQuestion": "Should the legacy material be deleted, archived, or kept behind a compatibility pointer?",
      "nextPlannerContext": "Use the conclusion when planning legacy cleanup after pinned install validation.",
      "associatedPlanNodeIds": ["plan-legacy-cleanup"],
      "associatedWorkSliceIds": [],
      "createdByRole": "plan-builder",
      "createdAt": "2026-07-07T00:00:00.000Z",
      "resolution": null
    }
  ]
}
```

When the user resolves a blocker in the UI, store a conclusion on the blocker record. The next planner prompt should include linked open blockers and linked resolved conclusions for the plan nodes it is evaluating.

## Records And Recording

Recording is a product projection concern. Codex can help summarize, prune, and normalize record material, but the product stores the results.

Separate:

- execution timeline: what the work-slice stages did
- recording activity: how the report changed maintained orchestration records
- orientation metadata: where a conversation sits in the graph
- task evidence: diffs, validation, raw event streams, reports

Do not store task payloads inside orientation metadata.

## UI Mapping

The orchestration UI should map directly to product records:

- Plan map reads `orchestration-plan.json` and shows the nested proposed problem/stage structure with state, blockers, active work, and completion.
- Live state reads `orchestration-live-state.json` and shows direct windows into currently running planner, worker, review, merge, report, and record stages.
- Blocker detail reads `orchestration-blockers.json` and lets the user record conclusions that become next-planner input.
- History reads planner turns and nested work slices.
- Work-slice timeline reads stage records and linked artifacts.
- Detail inspector reads prompt artifacts, outputs, raw event streams, diffs, validations, and record updates.
- Recording sidecar reads record update stages associated with the work slice.

## Skill Family Refactor Map

Use these replacements when updating old skills:

### Root Skills

Old: root orchestrator carries state and launches subagents.

New: product controller carries state and starts Codex executions. A root conversation can be a human-facing console, but not the source of truth.

### Planner

Old: planner fork owns delegation and waits for callbacks.

New: planner turn returns a structured next-work decision and prompt packets. Product creates work-slice lifecycle nodes and launches them.

### Delegation

Old: delegation thread coordinates worker, review, merge, report, and planner notification.

New: product work-slice controller advances stage by stage. Delegation skill can generate the worker prompt packet and initial slice metadata.

### Worker

Old and new: bounded execution unit. The worker should not need broad orchestration history.

### Review/Merge/Report

Old: sequential conversation continuation.

New: stage prompt packets attached to a work slice. Reuse a conversation only when context continuity is actually valuable.

### Record Maintenance

Old: record root and maintainer threads own records.

New: product owns records. Codex record-maintainer turns produce normalized record update proposals or summaries.

### Intake/Compression

Old: intake refresh keeps root context fresh.

New: product projections keep state fresh. Intake is a summarization or delta-generation stage when a human-facing or planner-facing prompt needs compact current context.

## Near-Term Compatibility

Current live skills may still use root/planner/delegation language. When editing them incrementally:

1. Preserve the active workflow unless the user asks to switch runtime behavior.
2. Add product-owned terminology where it reduces ambiguity.
3. Remove callback-heavy language when the product can route the next stage.
4. Prefer prompt-packet and lifecycle-stage language over subagent-parent semantics.
5. Keep current skills installable until the product has equivalent state and routing support.
