# Main application review

Baseline: `main` at `f23f5fd`

## What the work has achieved

The application now has a coherent first vertical orchestration path:

1. A user opens **Plan an Epic** and discusses an Epic with a managed Plan Builder Agent Session.
2. Ordinary conversation remains conversation. It cannot become structured product state.
3. The Plan Builder may submit a structured proposal through its narrowly exposed product tool.
4. The proposal is persisted and projected beside the conversation as an Epic with predicted
   Sprints and concerns.
5. Initiation—whether requested by the agent or by the button—goes through one application-owned
   confirmation modal.
6. After confirmation, the application deterministically prepares the Epic root and approved input
   material.
7. A sandboxed Bootstrap Generator receives those inputs. It can complete material generation only
   through its application-owned semantic tool.
8. The application accepts material only when the same Bootstrap attempt has both semantic
   completion and a successful terminal Agent Session lifecycle.
9. Exactly one separate, read-only Epic Runner Agent Session is then created and launched.
10. The current implementation stops before starting a product Sprint.

The final integrated confirmation click and post-click observation remain a human-gated online
proof. The deterministic implementation was accepted; the complete real-provider happy flow was not
claimed.

## Sprint timeline

### Sprint 1 — Product data and controller integration

Established the canonical product-read composition boundary and separated read models from
controllers. Agent Control commands, artifact access, continuation requests, and their results no
longer share ambiguous fixture-shaped boundaries.

### Sprint 2 — Epic and Sprint terminology

Replaced the old Epoch-oriented product language with:

`Orchestration capability → Epic → Sprint → Work Unit`

The migration was a clean break because no production data required compatibility aliases.

### Sprint 3 — Agent access boundary and Codex CLI adapter

Made Agent Session provider- and orchestration-role neutral. The application depends on an
`AgentRuntime` port; Codex CLI is the only concrete adapter. Codex-specific executable discovery,
capability probing, arguments, JSONL parsing, external-context resume, and process reconciliation
stay inside that adapter.

The current supervisor owns direct children. It does not claim Windows descendant-tree ownership.

### Sprint 4 — Epic Plan Builder foundation

Added the conversation-primary **Plan an Epic** workspace using the reusable Agent Session
components. A structured proposal appears adjacent to the conversation through an injected source.
Transcript prose cannot update it.

The first successful send creates and durably binds the Plan Builder Agent Session. Opening the
screen alone creates no session or draft.

### Sprint 5 — Durable proposal and initiation foundation

Added durable proposal revisions, strict native query decoding, initiation facts, restart
projection, and the canonical initiated-Epic overview. Initiation remains a plan-level fact; it does
not mean materials were accepted, a Runner launched, or a Sprint started.

Sprint 5 was partially closed because its MCP/live path was deliberately deferred rather than
overclaimed.

### Sprint 6 — Managed Plan Builder through Epic Runner launch

Completed the bounded product-owned flow:

- versioned role harnesses;
- Plan Builder tools `submit_epic_plan_proposal` and `request_epic_initiation`;
- server-derived identity and authorization;
- one shared confirmation coordinator and modal;
- durable button-origin context delivery on the next managed query;
- deterministic Epic-root preparation;
- sandboxed Bootstrap Generator with `complete_epic_bootstrap`;
- attempt-bound retry and recovery;
- accepted material inventory with hashes;
- exactly one launch-accepted, read-only Epic Runner;
- truthful transition status in the UI.

No product Sprint is created or started.

## Important semantic boundaries

- **Agent Session is an interaction context**, not an orchestration role.
- Product roles are supplied by a product-owned Conversation Harness around an Agent Session.
- Product-to-agent provider access and agent-to-product tools are separate boundaries.
- Tool exposure is not authorization.
- Agent prose is not authoritative product state.
- An Agent Control command may lead to an application MCP call, which produces an Orchestration
  Event. Read models project those events for the UI.
- Requested, confirmed, applied, persisted, projected, observed, reviewed, and accepted are
  different facts.
- `initiated` means the proposal was durably initiated—nothing later.
- Agent Session completion alone cannot prove semantic work or material acceptance.

## Current stop line

The product can reach an Epic Runner launch. It does not yet run the Epic through Sprints or Work
Units. It does not implement automatic continuation, parallel scheduling, approval routing, or
product-native pause/resume.

## Offline-reviewable versus online-only

### Reviewable offline

- Orchestration overview and navigation.
- Plan an Epic layout, hierarchy, copy, and relationship between conversation and proposal.
- Recorded Agent Session presentation.
- Terminology and status vocabulary.
- Architectural decisions and durable-state contracts.
- The shape of the initiation and Bootstrap-to-Runner flow through documentation and tests.

### Requires connectivity and a live provider

- A real Plan Builder discussion.
- Actual skill selection by Codex.
- Live MCP tool selection and submission behavior.
- The real confirmation-to-Bootstrap-to-Runner integrated flow.
- Provider processing, external-context resume, concurrency, and cancellation proof.

## What to decide while reviewing

Use the [decision worksheet](DECISION-WORKSHEET.md) to record:

- whether the Plan Builder hierarchy feels subordinate to the conversation;
- whether proposed Sprints and concerns provide the right planning overview;
- whether confirmation and transition vocabulary is understandable;
- whether the current role/harness separation matches your mental model;
- whether the next product work should finish the Epic Runner → Sprint flow before broadening tools;
- which structural liabilities deserve cleanup before the product surface expands.
