# Execution Configuration and Session Event target

Status: working target direction for `codex/session-event-model-overhaul`.

This directory describes the replacement for the mixed Harness and generic Workflow Role models.
It is an implementation guide, not a claim that every described integration or UI already exists.
The functional foundation is developed before the UI is recomposed; temporary UI breakage on this
branch is acceptable.

## How to use this guidance

The accepted conceptual distinctions and explicitly deferred concepts are the stable input. File
names, module decomposition, UI composition, implementation mechanics and delivery subdivision are
recommended shapes. Agents with more targeted evidence may adapt those details when the result
preserves the ownership and runtime-truth intentions. Record a short rationale when choosing a
materially different boundary so later work can understand whether the target changed or only its
implementation did.

The documents intentionally avoid specifying every future extension. Prefer the smallest concrete
implementation supported by current use over abstractions justified only by possible later needs.

## Read by task

| Work                             | Read first                                        | Then                                                                                                                                              |
| -------------------------------- | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Domain or backend implementation | [Conceptual model](conceptual-model.md)           | [Functional happy flow](functional-happy-flow.md), [contract rules](contract-rules.md)                                                            |
| Workflow implementation          | [Conceptual model](conceptual-model.md)           | [delivery sequence](delivery-sequence.md), [UI mapping](ui-mapping.md)                                                                            |
| Agent Session implementation     | [Functional happy flow](functional-happy-flow.md) | [contract rules](contract-rules.md)                                                                                                               |
| UI implementation                | [UI mapping](ui-mapping.md)                       | [codebase map](codebase-map.md), [contract rules](contract-rules.md)                                                                              |
| Review or integration            | [Delivery sequence](delivery-sequence.md)         | [codebase map](codebase-map.md), [contract rules](contract-rules.md)                                                                              |
| Mounted-model repairs            | [Repair plan](repair-plan/README.md)              | [Work packages](repair-plan/work-packages.md), [codebase projection](repair-plan/codebase-projection.md), [validation](repair-plan/validation.md) |

## Accepted direction

- **Runtime Profile** is the observed native runtime available to the application. Initially this is
  the one globally selected Codex profile.
- **Capability Profile** is a separately managed reusable technical capability ceiling.
- **Node Profile** is embedded in a Workflow node. It can be copied between nodes but is not a
  reusable referenced entity in the first implementation.
- **Session Profile** is the immutable resolution pinned when an Agent Session is created.
- **Session Events** are the generic foundation for creating, addressing and messaging Sessions.
- **Workflow** is an authoring abstraction that compiles nodes and connections into Session Event
  definitions.
- **Agent Identity** is independent presentation and assignment data. It does not determine
  capabilities or operational behavior.
- Generic Workflow Roles and the mixed Harness aggregate are removed rather than adapted.
- Direct user messages may select model and reasoning per message from the attached runtime
  capability. They do not mutate the Session Profile.

## Current implementation anchors

- `src-tauri/src/execution_configuration/`: Capability, Node and Session Profile vocabulary and
  resolution.
- `src-tauri/src/session_events/`: event definitions, materialization, addressing, dispatch ports
  and records.
- `src-tauri/src/workflows/compiler.rs`: Workflow-to-Session-Event compilation foundation.
- `src-tauri/src/workflows/address_references.rs`: Workflow identity translation.
- `src-tauri/src/agent_sessions/session_event_adapter.rs`: Agent Session implementation of Session
  Event ports.
- `src/components/CollapsibleSection.tsx`: reusable disclosure UI.
- `src/features/workflows/editor/`: reusable Workflow editor behavior.
- `src/features/agentSessions/`: reusable Session workspace and transcript behavior.

The current frontend still contains legacy `conversationHarnesses` and Workflow Role/Harness
contracts. Their useful controls are extraction sources, not target domain compositions.

## Current checkpoint

The replacement services and controls were mounted by `aaca806`. Review checkpoint `e77a725`
records missing instance/canvas flows and creation, event-wiring, draft-state and layout failures.
The earlier unmounted foundation proof did not establish these mounted flows. See the
[regression review](../regression-review/README.md) for observed evidence and the
[repair plan](repair-plan/README.md) for the agreed scope. Repairs were implemented in
`codex/session-event-regression-review` for consolidation into `codex/harness-ux-workflow-convergence`. The [repair record](../regression-review/repairs/README.md)
maps the changed boundaries and tested flows, including real file-source and local managed-MCP
integration. A fresh native-window walkthrough and user acceptance remain unverified; broad legacy
retirement remains later.

## Documentation authority

These documents are the primary working guide for the target shape of this overhaul. Existing
records that describe session-owned Harness editing or reusable Workflow Roles are historical
implementation evidence. Git history remains the source for their original detail.
