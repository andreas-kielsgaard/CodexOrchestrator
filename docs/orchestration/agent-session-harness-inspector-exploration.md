# Agent Session Harness Inspector exploration

Status: bounded adjacent product exploration on `codex/explore-harness-inspector`.

## Direction

A product-context wrapper places **Inspect harness** over an Agent Session pane only when that
context supplies a `ConversationHarnessInspectorSource`. Opening it replaces the conversation with
one inspector; **Back to conversation** restores the same pane.

The Agent Session contracts and components remain neutral. The wrapper owns the product-specific
control, and the inspector receives a read model through an application boundary.

The in-app development surface is available only from the recorded development composition. Run
the app in Vite development mode and open `/?harness-inspector`; production composition does not
receive the tab or source.

## Architecture evidence

- `src/application/conversationHarnesses/harnessInspector.ts` defines available/unavailable reads,
  provenance, validation, scope, editability, and unsupported apply state.
- `src/features/conversationHarnesses/HarnessAwareAgentSessionPane.tsx` owns the conditional control
  and pane replacement without changing Agent Session.
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
- Context durably evidenced as delivered cannot be rewritten for that existing session. This
  exploration does not claim that evidence and keeps the configured value read only.
- Skills, MCP allow-list, model/reasoning, and sandbox settings are shown as **future invocation**
  configuration. Their controls are disabled in this exploration.
- Application hooks are **application owned**. Declarative completion criteria do not apply product
  effects.
- Catalog/profile shape checks can pass while skill discovery and session delivery remain
  unverified. Invalid validation is presented separately from unverified validation.
- An unavailable source produces an explicit unavailable state and retains the return path.
- A future apply must validate the complete profile, reject stale revisions, create a new version
  for future invocations, preserve product authority limits, and record configuration provenance
  separately from activation. This branch adds no apply command.

## Prototype limits

- There is no product query, durable delivery observation, editor state, persistence command,
  authorization decision, runtime mutation, or live provider proof.
- The recorded session can exercise normal in-memory Agent Session controls; it does not establish
  harness mutation support.
- The source mirrors the checked-in catalog at build time. It does not prove runtime catalog load or
  repository skill discovery.
- Only the Epic Plan Builder profile is presented. No general settings framework or second design
  was created.

## Validation

- Focused corrected inspector and app tests: 3 files / 7 tests passed.
- Serial frontend aggregate: 90 files / 609 tests passed.
- TypeScript, production Vite build, and touched-file ESLint passed.
- Headless Edge review passed at 1440 × 1000 and 760 × 900. The control placement, pane
  replacement, return path, internal scrolling, responsive layout, and disabled apply state were
  inspected. The corrected recorded view showed **Delivery not evidenced**, **Profile
  configuration · Read only**, and **Validation unverified**, with no delivered-context claim.
- The aggregate retained existing non-failing React `act(...)` and Node SQLite experimental
  warnings.

## User-review points

1. Is the over-pane **Inspect harness** control discoverable without competing with session actions?
2. Is one scrollable inspector clearer than nested tabs for this amount of configuration?
3. Are **Profile configuration**, **Future invocation**, and **Application owned** the right scope
   labels, with delivery evidence shown separately?
4. Should full prompt/context stay visible by default, or collapse behind a summary?
5. Which future settings should ever be editable, especially sandbox, MCP tools, and hooks?
6. Are the proposed stale-revision and new-version rules sufficient before any apply work?

## Exact next product slice

Add read-only Epic Plan Builder integration only:

1. expose an application-owned product query that resolves the bound profile, catalog revision, and
   durable first-query delivery evidence for one Agent Session;
2. adapt that query to `ConversationHarnessInspectorSource`;
3. wrap the Epic Plan Builder Agent Session pane and show the control only for a successful bound
   read;
4. prove unavailable, invalid-catalog, unbound-session, delivered, and not-yet-delivered states.

Do not add editing in that slice. A later slice may propose versioned draft/apply commands after the
read-only provenance and authority boundary is accepted.
