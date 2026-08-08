# Frontend experience map

## Initial operational-line surfaces

`src/main.tsx` mounts `ApplicationRoot`, which constructs a product, recorded-development, Human Review, or isolated Worktree Build composition. The resulting `App` supports these surface identifiers:

| Surface | User purpose | Normal product composition |
| --- | --- | --- |
| `epics` | Orchestration overview, Plan Builder, Epic/Sprint/Work Slice/Work Unit inspection and controls | Primary surface |
| `agent-sessions` | Browse, open, continue, and inspect durable Agent Sessions | Primary surface |
| `native-settings` | Manage native Codex homes, execution modes, readiness, canaries, and MCP reporting | Primary on the operational/native-profile line |
| `file-review` | Inspect contextual file and Git evidence | Entered contextually; direct navigation appears only when an explicit source is injected |
| `harness-inspector` | Exercise full Harness Management | Direct top-level surface only in recorded development composition |
| `worktree-review` | Prepare and operate review instances | Development composition and debug backend only |

Production still supplies a read-only Harness Management source to Agent Session and Plan Builder contexts. The absence of a top-level production Harness surface does not mean Harness inspection is absent.

## Orchestration information architecture

- `OrchestrationSection` owns overview and selected-detail routing.
- `EpicPlanBuilder` embeds the reusable Agent Session workspace alongside durable proposal and initiation controls.
- `EpicDetail` presents Epic state, plan, concerns, continuation, sessions, and supporting detail.
- `SprintWorkspace` combines plan, flow, activity, documents, related sessions, transition evidence, and File Review entry.
- `WorkSlicePlanningPointDetailWorkspace` presents planning-point relationships and associated agents.
- `WorkUnitDetailWorkspace` presents Handler/Implementer sessions, outcome activity, review, integration, incomplete dispositions, and retries.
- `SharedAgentSessionPanel` provides a common embedded conversation surface for orchestration-associated sessions.
- `DetailWorkspace` and `ResizableSplitSurface` provide repeated detail-plus-conversation layouts.

The product therefore already has a recognizable design grammar: orientation/detail navigation, activity and evidence inspection, embedded agent conversation, progressive disclosure, state badges, and contextual technical evidence.

## Agent Session system

- `StandaloneAgentSessionScreen` combines hierarchical session selection with a reusable `AgentSessionWorkspace`.
- `useAgentSessionController` owns selection, loading, update subscription, send, cancel, and draft behavior.
- `ConversationViewport`, `AgentSessionTranscript`, `ProcessingDisclosure`, and `TechnicalDiagnosticDisclosure` separate conversational and technical layers.
- `SessionSelector` projects product associations and supports returning to related orchestration locations.
- `AgentSessionWorkspace` is reused by standalone sessions, Plan Builder, Harness-aware panes, and orchestration-associated panels.

## Harness experience

`ConversationHarnessManagement` is a complete management interface rather than a small inspector. It contains effective configuration, editable fields, policy cards, model controls, versions, session identity, name pools, skills, tools, and confirmation dialogs.

The product source is read-only because `tauriConversationHarnessInspectorSource` supplies inspection but no dispatch path. The recorded source supplies mutations, commits, publication, queueing, and related state. The experience therefore spans productive inspection and development-only management behavior in the same component.

## Reusable elements and physical ownership

| Element | Reuse | Ownership concern |
| --- | --- | --- |
| `AgentSessionWorkspace` | Standalone, Plan Builder, Harness-aware panes, embedded orchestration | Appropriately centered on the Agent Session feature |
| `ConversationViewport` | Shared transcript viewport and follow behavior | Strong reusable primitive |
| `SharedAgentSessionPanel` | Orchestration details | Feature-owned integration component |
| `DetailWorkspace` | Epic/Sprint/subdetail layouts | Orchestration-owned but broadly useful layout |
| `ResizableSplitSurface` | Orchestration and standalone Agent Sessions | Agent Sessions import a layout primitive from Orchestration |
| `MarkdownContent` / `AgentMarkdown` / `MarkdownEditor` | Sessions, File Review, Harness Management | Generic Markdown behavior is partly owned by Agent Sessions |
| `AgentIdentityBadge` / `AgentIdentityMarker` | Sessions, Harnesses, Plan Builder | Two nearby identity presentation layers need relationship clarification |
| `ProductViewHeader` | Plan Builder and shared product presentation | Early shared presentation primitive |

## Sibling-line experience additions

The Product Decisions/navigation line adds or materially extends:

- A global typed product-navigation and Back history model.
- A primary command bar spanning product destinations.
- Exact contextual return from File Review and Agent Sessions to the originating Work Unit and selected turn.
- Read-only Agent Session turn inspection with privacy-aware technical detail.
- A richer Work Unit Activity and Evidence experience.
- Product Decisions views, history, evidence navigation, correction conversations, proposal acceptance, and a deliberately non-functional Publish placeholder.

These are cumulative product and design insights even though the initial operational baseline does not contain them.

## Early design questions

- Is the product navigation model from the sibling line the intended shell foundation?
- Should Harness Management remain contextual, become a primary surface, or separate operator management from everyday inspection?
- Are File Review and Worktree Review one evidence family or distinct user experiences?
- Can shared layout, Markdown, identity, badge, activity, and evidence components move to neutral ownership without flattening useful feature semantics?
- Which recorded workflow maps remain valuable design hypotheses after durable native-query projections matured?
- How should technical readiness and orchestration status coexist without making the product feel like a backend dashboard?

