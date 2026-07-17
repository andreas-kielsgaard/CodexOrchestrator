# Agent Session Harness Inspector exploration

Status: bounded read-only product integration on `codex/explore-harness-inspector`.

## Direction

A product-context wrapper places **Inspect harness** over an Agent Session pane only when that
context supplies a `ConversationHarnessInspectorSource`. Opening it replaces the conversation with
one inspector; **Back to conversation** restores the same pane.

The Agent Session contracts and components remain neutral. The wrapper owns the product-specific
control, and the inspector receives a read model through an application boundary.

The recorded demonstration surface remains available in Vite development mode at
`/?harness-inspector`. Product composition instead supplies a separate application-owned source
only to the Epic Plan Builder pane.

## Architecture evidence

- `src/application/conversationHarnesses/harnessInspector.ts` defines available/unavailable reads,
  provenance, validation, scope, editability, and unsupported apply state.
- `src-tauri/src/orchestration/application.rs` resolves the durable managed Plan Builder binding,
  current catalog profile and revision, and first-query launch-acceptance evidence without creating
  state.
- `src-tauri/src/orchestration/transport.rs` exposes that read through
  `load_managed_plan_builder_harness_inspection`.
- `src/infrastructure/conversationHarnesses/tauriConversationHarnessInspectorSource.ts` validates
  and adapts the product query to the existing inspector read model.
- `src/features/conversationHarnesses/HarnessAwareAgentSessionPane.tsx` owns the conditional control
  and pane replacement. It shows the control only after a successful bound read.
- `src/features/conversationHarnesses/ConversationHarnessInspector.tsx` presents prompt/context,
  skills, MCP tools, model/reasoning, sandbox/authority, hooks, validation, and provenance.
- `src/dev/conversationHarnesses/recordedHarnessInspectorSource.ts` parses the checked-in v2 catalog
  through a recorded adapter and binds it only to one recorded session.
- `src/app/ApplicationRoot.tsx` activates the recorded composition only in development mode.

## State and safe-apply semantics

- Initial prompt/context is a **configured profile value**. Configuration scope, durable delivery
  evidence, and editability are separate read facts.
- The recorded source reports delivery as **not evidenced**. A fixture session does not prove that
  the configured prefix reached it.
- The product source reports **delivered** only when the first query has the separate durable Agent
  Session launch-acceptance fact. `started_at`, terminal state, and transcript content are not
  delivery evidence.
- A first query with a durable preflight or launch rejection is **not delivered**. A bound session
  with no first query is also **not delivered**.
- A missing launch-acceptance fact or a binding that postdates the first query is **not evidenced**.
  Older history is not upgraded from timestamps or terminal state.
- Launch acceptance proves that the runtime accepted the first managed query. It does not retain
  the exact prompt bytes, so the inspector presents the current catalog value and delivery evidence
  as separate facts.
- Context durably evidenced as delivered cannot be rewritten for that existing session. This
  exploration does not claim that evidence and keeps the configured value read only.
- Skills, MCP allow-list, model/reasoning, and sandbox settings are shown as **future invocation**
  configuration. Their controls are disabled in this exploration.
- Application hooks are **application owned**. Declarative completion criteria do not apply product
  effects.
- Catalog/profile shape checks can pass while skill discovery and session delivery remain
  unverified. Invalid validation is presented separately from unverified validation.
- Unavailable transport, invalid catalog, and unbound session are distinct read states. The product
  control is absent unless the bound read succeeds.
- A future apply must validate the complete profile, reject stale revisions, create a new version
  for future invocations, preserve product authority limits, and record configuration provenance
  separately from activation. This branch adds no apply command.

## Prototype limits

- There is no editor state, persistence command, authorization decision, runtime mutation, or live
  provider proof.
- The recorded session can exercise normal in-memory Agent Session controls; it does not establish
  harness mutation support.
- The recorded source mirrors the checked-in catalog at build time. The product source reads the
  Rust-owned compiled catalog but does not prove per-invocation repository skill discovery.
- Only the Epic Plan Builder profile is presented. No general settings framework or second design
  was created.

## Validation

- Read-only continuation Rust tests: 5 tests passed across the managed query and catalog
  validation.
- Read-only continuation frontend tests: 6 files / 23 tests passed.
- TypeScript, production Vite build, touched-file ESLint, Rust formatting, and diff checks passed.
- The initial recorded exploration also passed its serial frontend aggregate of 90 files / 609
  tests.
- Headless Edge review passed at 1440 × 1000 and 760 × 900. The control placement, pane
  replacement, return path, internal scrolling, responsive layout, and disabled apply state were
  inspected for the recorded demonstration. The corrected recorded view showed **Delivery not
  evidenced**, **Profile
  configuration · Read only**, and **Validation unverified**, with no delivered-context claim.
- No live provider invocation was used for the product read integration.

## User-review points

1. Is the over-pane **Inspect harness** control discoverable without competing with session actions?
2. Is one scrollable inspector clearer than nested tabs for this amount of configuration?
3. Are **Profile configuration**, **Future invocation**, and **Application owned** the right scope
   labels, with delivery evidence shown separately?
4. Should full prompt/context stay visible by default, or collapse behind a summary?
5. Which future settings should ever be editable, especially sandbox, MCP tools, and hooks?
6. Are the proposed stale-revision and new-version rules sufficient before any apply work?

## Remaining boundary

Editing remains deferred. A later slice may propose versioned draft/apply commands only after this
read-only provenance and authority boundary is accepted.
