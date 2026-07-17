# 0005: Sprint 5 initiation scope revision

Status: accepted current Sprint 5 decision; supersedes the initiation-related future-state
wording, schema-version-1 assumption, and native-query/v1 current-state statement in
`0001-durable-state-and-native-query.md`. Records the minimum non-MCP initiation state authorized
by the user. Earlier decisions remain historical unless explicitly superseded.

## Decision

Sprint 5 re-evaluation authorized and implemented the minimum truthful initiation path:

- active-v3 schema version `2` preserves v1 fields additively while retaining the native-query/v2
  contract;
- initiation records the exact proposal material snapshot and correlates one initiation command,
  result, event, and provenance chain;
- a successful initiation leaves the planning draft terminally `initiated`, creates one Epic, and
  creates its ordered preparatory Sprints;
- the canonical product projection reads the native query and exposes this initiated state;
- no Work Units are created and no execution is started.

This decision records durable initiation and restart recovery as accepted product facts. It does
not authorize broad material generation, artifacts, execution, scheduling, runners, or
continuation.

## Explicit deferrals

MCP correction or integration, external MCP investigation, and the unresolved Plan Builder MCP
failure are deferred to Epic Planner authority. They are not accepted Sprint 5 evidence. Tool-
specific, Harness, provider/live-proof, and MCP declarative drift remains deferred; this record
does not edit those surfaces.

WU-S5-17 UI is accepted. No additional UI or G3 retest is required for this decision.

## History and links

The prior records remain historical: [0001](0001-durable-state-and-native-query.md),
[0002](0002-mcp-transport-and-access.md), [0003](0003-managed-codex-invocation-and-proof.md),
and [0004](0004-plan-builder-tools-and-contract-evaluation.md). Current non-MCP evidence is in
[Sprint 5 convergence evidence](../sprint-5-convergence-evidence.md).
