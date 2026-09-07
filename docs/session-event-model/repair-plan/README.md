# Functional and UI repair plan

Revision 1 — 7 September 2026. Implementation update: repairs are present for consolidation into Harness UX. See the [repair record](../../regression-review/repairs/README.md) for actual files, checks and limits. The package descriptions below retain the agreed plan.

This is one plan, split by reader need:

- This page: scope, decisions, concerns, dependencies and gates.
- [Work packages](work-packages.md): outcomes, tasks, boundaries and local acceptance.
- [Codebase projection](codebase-projection.md): proposed files, contracts, storage and reuse.
- [Validation](validation.md): proof cases, test locations and completion checklist.

Names and file splits below are recommendations. Keep the stated ownership and behavior; adapt the implementation when closer inspection supports a simpler shape. Record material changes to the plan rather than silently changing its scope.

## Objective and completion

Make the new Capability/Node/Session Profile and Session Event model work through the mounted product. Restore flow-based authoring and Workflow instance creation without restoring Roles or mutable Session Harness configuration.

Complete means a user can:

1. Create a standalone Session and send further messages.
2. Edit nodes and connections on a flow canvas without losing edits.
3. Activate a recipe, create an instance in a chosen worktree and reopen it.
4. Start a node, complete its turn and deliver work through a connection to the right Session(s).
5. Use the existing managed MCP handoff through the new model.
6. Inspect pinned settings, assigned identity, prompt sources and recorded deliveries.
7. Change model/reasoning for one direct message without changing later Workflow defaults.

These paths must have real application/repository checks with a fake runtime, plus mounted UI and browser checks. A compiler test or seeded screenshot alone cannot close a package. Live-provider acceptance and integration with parallel branches remain separate.

## Baseline and prior outcomes

- Worktree: `C:/Users/user/.codex/worktrees/session-event-regression-review/Codex Orchestrator`.
- Branch: `codex/session-event-regression-review`; baseline `e77a725`.
- Product source is the mounted UI checkpoint `aaca806`; `4bded63` saved the walkthrough and `e77a725` saved the review.
- Earlier comparison: combined Workflow/Harness checkpoint `9fc822f`.
- Review: [findings](../../regression-review/README.md), [backend](../../regression-review/backend-findings.md), [frontend](../../regression-review/frontend-findings.md), [layout](../../regression-review/ui-layout-findings.md).
- Previous checks: frontend build passed; 980 tests in 165 files passed; nine browser failures reproduced. These are baseline results, not repair acceptance.
- No Rust suite or live-provider flow was run in the review. Node and Cargo are available; this planning pass did not build native code.
- The review worktree was clean before these planning documents. Other worktrees, demo data and later walkthrough files are outside this plan.

## Scope and decisions

### Accepted constraints

- Capability Profiles remain separate technical capability definitions.
- Node Profiles stay embedded; copying does not create a reusable profile reference.
- Session configuration is pinned at creation. Direct user model/reasoning choices are per message and may use the full attached runtime exposure.
- Provider setup stays separate. Continue consuming the selected native profile; do not add provider discovery, credentials UI, CLI controls, skill management or compaction work.
- Session Events retain generic identities and ports. Workflow translates its instance/node identities at its own boundary.
- Restore the authoring canvas. Do not decide the future canonical run UI or redesign Epic here.
- No new Role or legacy Harness adapter in the replacement path. Broad legacy retirement and parallel-branch merges follow later.

### Planning defaults

These implement the accepted remedy direction, not additional product features:

| Decision                     | Planned behavior                                                                                                                                                                                                 | Where it matters                                                                  |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| D1 — standalone defaults     | Build an application-owned default from the selected runtime. Feed the ordinary creation resolver; do not invent a Workflow node or a user-managed default catalogue. Mark its origin as an application default. | R1; changing this to a user-selected profile needs a new creation UI.             |
| D2 — old unprofiled Sessions | Keep history readable. Clearly mark unsupported sending; do not silently fabricate a past profile or fall back to a policy-bypassing send. Do not migrate/delete old data in this slice.                         | R1, R8.                                                                           |
| D3 — pinning times           | Copy the active recipe into a new instance. Resolve Capability Profile values and runtime settings when each Session is born. Later messages use that Session's stored result.                                   | R2, R3. Freezing capability values for an entire run would be a different policy. |
| D4 — drafts                  | Keep explicit Save. Keep working edits apart from saved/active data and retain them across in-app navigation. Reload/discard/close must not silently erase dirty edits. Activation targets the saved revision.   | R6, R7. No autosave or crash-recovery system is required.                         |
| D5 — identity                | Restore assigned identity display and the existing Session-local identity picker. Identity edits change presentation only; no profile/recipe mutation.                                                           | R8. This retains the previously requested identity feature.                       |
| D6 — trigger support         | Wire user, successful-invocation completion, the existing managed MCP handoff and an explicit application-event entry. Group-completion remains disabled. Unsupported provider variants cannot be activated.     | R4, R5, R6.                                                                       |

The remaining group-completion decision is whether it means delivery finished or all recipient invocations finished, and how failures count. Do not infer it from `Delivered`. It does not block the other packages.

## Concern and uncertainty map

Low/medium/high describe planning risk, not implementation status. Changes are code-reversible; persistent-schema work needs additive migrations and test databases, not resets.

| Concern / evidence                            | Definition and ambiguity                                    | Complexity | Context and blast radius                                | Reversibility and proof                                         | Owner / decision            |
| --------------------------------------------- | ----------------------------------------------------------- | ---------- | ------------------------------------------------------- | --------------------------------------------------------------- | --------------------------- |
| C1: missing birth profile, B1                 | Cause clear; standalone policy was implicit                 | Medium     | Session creation, transport and UI                      | Additive route; first/second-message application test           | R1; D1/D2                   |
| C2: instance/target missing, B3/U2/F5         | Required fields clear                                       | Medium     | Workflow storage, navigation, Session working directory | Additive table; create/reopen/no-launch test                    | R2/R7; D3                   |
| C3: current catalog blocks pinned Session, B4 | Cause clear; timing now explicit                            | Medium     | Compiler and configuration resolution                   | Version creation payload; profile edit/delete tests             | R3; D3                      |
| C4: no real trigger route, B2                 | Completion path clear; caller correlation needs care        | High       | Native composition, Workflow, Session Events            | Fake runtime normal-notifier test; two-instance isolation       | R4                          |
| C5: MCP launch/handoff missing, B2            | Existing technical path available; old aggregate is coupled | High       | MCP proxy/binding, Session launch, Workflow             | Fake sidecar and local MCP request; inspect exact launch config | R5; no expanded CLI surface |
| C6: impossible prompt sources, B5             | Pairing clear; file reference convention needs naming       | Medium     | Authoring, materializer, file reader                    | Table tests and temporary-file tests                            | R4/R6                       |
| C7: lost edits, F1/F2/F6                      | Cause clear                                                 | Medium     | Feature controllers and top-level navigation            | Deferred-promise screen tests                                   | R6/R7; D4                   |
| C8: invalid choices/defaults, F3/F4           | Cause clear; avoid silent substitution                      | Low        | Controlled editors and validation                       | Small pure tests plus mounted payload test                      | R3/R6                       |
| C9: lost canvas, U1/U3                        | Desired presentation already known                          | Medium     | Old view extraction, new data contracts                 | Browser drag/connect/copy/reopen proof                          | R7                          |
| C10: identity/history, F7/F8                  | Read source clear; identity remains presentation-only       | Medium     | Session DTO, identity client, event notifications       | Assignment/edit/refresh tests                                   | R8; D5                      |
| C11: CSS, U4/U5                               | Cause clear                                                 | Low        | Shared controls and feature CSS                         | Computed browser layout and keyboard checks                     | R9                          |
| C12: misleading green tests / stale status    | Gap clear                                                   | Medium     | Composition and evidence docs                           | Tests of new mounted routes; status tied to proof               | R10                         |

## Packages and dependencies

Each package must deliver its own tests. R10 joins already-proven pieces; it is not where earlier packages first become testable.

| ID  | Outcome                                                    | Hard dependency                                   | Can overlap                                    |
| --- | ---------------------------------------------------------- | ------------------------------------------------- | ---------------------------------------------- |
| R1  | Profiled standalone and addressed Session creation         | G0 contracts                                      | R2, R6, R9                                     |
| R2  | Stored Workflow instances, pinned recipe and target        | G0 contracts                                      | R1, R6, R9                                     |
| R3  | Existing targets use pinned profiles; lazy creation inputs | R1 + R2 contracts; their services for final proof | R6 UI validation after contract agreement      |
| R4  | Real completion/application events and prompt sources      | R2, R3                                            | R5 after event-envelope agreement; R7/R8       |
| R5  | Existing managed MCP handoff through new profiles          | R1, R3; R4 event envelope                         | R4 source-specific work, R7/R8/R9              |
| R6  | Safe drafts and valid node/connection editing              | G0 contracts; R3/R4 validation rules              | Backend packages and R9                        |
| R7  | Canvas, instance UI and stable navigation                  | R2 contracts, R6 draft owner                      | Backend event adapters and R8                  |
| R8  | Session identity/settings/history stay correct             | R1 profile status; R4 record notice contract      | R5/R7/R9                                       |
| R9  | Collapse, form sizing and layout usability                 | Existing UI                                       | All non-overlapping feature work               |
| R10 | Mounted composition, full repair proof and status cleanup  | R1–R9 locally accepted                            | Documentation/test preparation can start early |

### Sequence and parallel lanes

1. **G0: contract checkpoint.** Agree the instance DTO, creation payload, event context, draft owner and delivery-change notice before parallel edits. No framework or standalone contract project is needed.
2. **Foundations:** R1 and R2 can run independently; R6's draft helper and R9 can proceed alongside them.
3. **Execution:** R3 joins creation to instances. R4 and R5 then share the event envelope but own separate source adapters. This is the main backend dependency chain.
4. **Presentation:** R7 starts against R2 contracts and R6 state. R8 starts against R1/R4 contracts. Fake clients enable early tests, not final acceptance of backend wiring.
5. **G1: functional gate.** R1–R5 application tests pass using real repositories and fake runtime/sidecar. This gates the next functional demo.
6. **G2: UI gate.** R6–R9 pass mounted-screen and real-browser checks. Run/recipe selection survives navigation; no hidden old route supplies the proof.
7. **G3: composition gate.** R10 proves the full path, storage reopen and the native no-provider smoke. Then present the repaired build for user review.

Shared files need coordinated edits: `active_app.rs`, `App.tsx`, composition files, module exports, DTO facades, `WorkflowAuthoringScreen.tsx` and Session Event record types. Give each one a current integration owner; other packages supply focused adapters/tests. Freeze only the small consumed contract, not every implementation detail.

### Gates that may change the plan

- **G0 decisions:** a change to D1, D3, D4 or D5 changes the affected packages; do not resolve it through an undocumented UI shortcut.
- **Native environment:** Rust/Tauri build and local MCP transport must run before their evidence is claimed. If an SDK/build issue prevents this, record it and hold the relevant acceptance; do not substitute a fake dispatcher for a real application check.
- **Group completion:** separate future decision/work. Disabled and accurately labelled is the bounded result here.
- **Live provider:** separate opt-in test after local proof. No credentials or private project content belong in test fixtures/screenshots.

## Risks, deferrals and current status

Highest rework risks are the MCP binding's dependence on old Harness records, eager compiler profile loading, and old canvas/controller types. Extract technical behavior or view behavior, not whole legacy aggregates.

Defer provider setup, new CLI controls, custom compaction, generalized scripting/plugins, a canonical run graph, old-data migration, comprehensive retry/recovery, broad legacy deletion, Epic redesign and parallel-branch integration. Preserve existing generic target selectors; do not add speculative new target-provider machinery.

R1–R9 now have implementation and focused proof. R10 has combined service tests, browser checks and a native build. A fresh native-window walkthrough and user review are still unverified, so the full G3/demo gate is not closed. No implementation commit has been made. The [repair record](../../regression-review/repairs/README.md) explains the smaller file/port choices made during implementation.

## Revision history

- Revision 1: first complete repair plan, based on `e77a725` and the accepted remedy discussion. Keep the earlier regression reports as baseline evidence. In particular, this plan clarifies that recipe pinning at instance creation does not move Session capability resolution earlier.
