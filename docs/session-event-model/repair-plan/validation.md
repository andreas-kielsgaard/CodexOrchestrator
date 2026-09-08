# Validation and completion checklist

Part of [repair plan revision 1](README.md). The matrix is the agreed proof target. Actual results and limits are now recorded in [repair verification](../../regression-review/repairs/verification.md); proposed test file names below may differ from the implemented names.

## Required proof cases

| ID / owner | Scenario                                                                                         | Required observation                                                                                                                                                  | Test placement                                                                              |
| ---------- | ------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| V1 / R1    | Create a standalone Session, complete first message, send second                                 | Both invocations launch through the intended route; same pinned profile and working folder                                                                            | `agent_sessions/application/profiled_creation_tests.rs`; mounted `AgentSessionScreen` test  |
| V2 / R1/R3 | First/direct message chooses a different model/reasoning, then a managed message                 | Override is within attached runtime, affects only its invocation, and does not alter stored defaults/digest                                                           | Profile application tests with recording runtime; per-message UI payload test               |
| V3 / R2    | Create, list and reopen an instance; activate another recipe revision                            | Creation launches nothing; exact target and recipe snapshot survive reopen; old/new instances use their own recipe versions                                           | `workflows/instances/tests.rs` with real SQLite                                             |
| V4 / R3    | Create Session, edit/delete Capability Profile, address existing Session, then request new birth | Existing send succeeds from pinned data without catalogue read; new birth uses current rules or reports the missing profile                                           | Compiler/execution + real Session Event adapter, not fake dispatcher                        |
| V5 / R4    | Complete A normally, deliver to B; repeat with two instances and repeated notification           | Correct instance/source/target, one occurrence per source/connection, no duplicate launch, A's success preserved                                                      | `workflows/event_sources_tests.rs` through the normal notifier                              |
| V6 / R4    | User/completion/application sources, creation vs continuation, target selector variants          | Ordered source text/references, initial text only on birth, exact/first/all/newest/last-addressed/running/missing policies retained                                   | Session Event tests plus application-event entry integration                                |
| V7 / R4/R6 | File content/path input and invalid trigger/source pairs                                         | Supported references resolve in the instance worktree; escape/missing file fails visibly before receiver launch; impossible pair/group trigger rejected at activation | Temporary-file reader tests, compiler/materializer table tests and editor validation        |
| V8 / R5    | Bind allowed MCP tool; call it through local managed transport                                   | Binding precedes launch; correct proxy config/tool allowlist; trusted source context reaches the new event receiver; disallowed/cross-instance call rejected          | Technical binding + local MCP/sidecar fixture with actual application consumer              |
| V9 / R6    | Edit during save, change selection while waiting, activate while dirty, leave/reopen, fail save  | Newer text and correct selection remain; saved revision is accurate; activation does not discard edits; failure retains draft                                         | Mounted Workflow/Capability Profile screen tests with deferred promises                     |
| V10 / R6   | Narrow profile, remove default, switch/copy node profile, exclude locked choice                  | No forbidden enabled option, no hidden invalid default, locked values cannot be excluded, invalid loaded draft is understandable                                      | Editor unit tests plus saved-payload assertions                                             |
| V11 / R7   | Place/drag/connect/copy/delete/edit nodes, save and reopen                                       | Layout/connection identity persists; copied config is independent; no Role-era adapter is required                                                                    | Controlled canvas tests and browser flow                                                    |
| V12 / R7   | Create instance with target, open Session, Back, switch instances, reload saved destination      | Same ID/target/results on return; pending old results cannot affect another instance; instance creation has no launch side effect                                     | Mounted App with new clients; browser journey                                               |
| V13 / R8   | Load/edit Session identity and receive new delivery while open                                   | Session assignment displayed/persisted independently; post-store refresh works; errors/retry shown; stale replies ignored                                             | Identity reuse, delivery hook and Session screen tests                                      |
| V14 / R9   | Collapse fields and use editors at 1280, 958, 850 and 640 px widths                              | Hidden content has zero visible footprint/no tab stops; checkbox labels readable; picker/create/toolbar reachable with scroll                                         | Real browser with computed layout, keyboard and screenshots                                 |
| V15 / R10  | Combined native application services and mounted product routes                                  | Stored instance → profiled Session → completion → handoff → durable reopen; new clients actually mounted; no provider needed for the bounded proof                    | `workflows/runtime_flow_tests.rs`, `App.sessionEventModel.test.tsx`, full build/test checks |

The full target-selector matrix already has generic tests. Reuse them and add adapter-level cases where the new wiring could alter behavior; do not duplicate every combination in every UI test.

## Failure and scope checks

- Invalid creation inputs leave no orphan Session/address and launch nothing.
- A missing attached native profile reports unavailable; it does not select a different provider behind the user's back.
- A handoff error leaves the sender's terminal fact unchanged and is visible in instance evidence.
- Duplicate occurrence suppression must be tested in the supported notification path. Do not claim crash-safe exactly-once delivery or automatic recovery.
- Historic unprofiled Sessions remain readable; no migration or fallback silently assigns them a new profile.
- Existing valid pinned Sessions remain readable. Do not relabel all pre-repair Sessions as unprofiled.
- No test reads arbitrary user files or sends credentials/private repository content to a provider.
- Generic Session Event tests should use non-Workflow reference namespaces too. A directory/import check should confirm Workflow types have not leaked into `session_events`.
- New Workflow consumers must not import old Role/mixed Harness types or use legacy `workflow_adapter` to manufacture binding authority.

## Test seams and feasibility

Available now: recording/fake runtime patterns, SQLite temporary repositories, injected frontend clients, existing browser observation fixture, managed MCP sidecar fakes and Node/Cargo tooling. The previous review proved browser tooling locally, not native execution of the new route.

Required package work: R1 adds the complete birth/send fixture; R4 exposes the normal notifier/receiver seam to tests; R5 exercises binding and local transport; R10 joins the same production services. Do not create a second miniature workflow executor inside tests.

Prefer production composition factories accepting existing ports over a special product-wide fake-runtime mode. Browser tests may use fake clients; label that evidence separately from actual Rust transport/application tests. Native no-provider smoke can create/load an instance in its isolated database but must not dispatch a real provider message.

If Cargo/native dependencies prevent a required check, record the exact failed command and environment blocker. A frontend pass does not close the Rust package. No such blocker has been established during planning.

## Command plan

Run focused tests during their package, then the joined checks. Filters refer to planned owning modules; confirm exact test names after implementation.

```powershell
npm test -- src/features/executionConfiguration src/features/workflowAuthoring src/features/workflowInstances src/features/agentSessions src/features/sessionEvents
npm test -- src/app/App.sessionEventModel.test.tsx src/bootstrap/productApplicationComposition.test.ts
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --profile test-fast --lib execution_configuration
cargo test --manifest-path src-tauri/Cargo.toml --profile test-fast --lib session_events
cargo test --manifest-path src-tauri/Cargo.toml --profile test-fast --lib agent_sessions
cargo test --manifest-path src-tauri/Cargo.toml --profile test-fast --lib workflows
cargo test --manifest-path src-tauri/Cargo.toml --profile test-fast --lib harness_engine
cargo build --manifest-path src-tauri/Cargo.toml
git diff --check
```

Audit existing tests before running a broad Rust filter so it cannot invoke a live provider or modify real user data. Keep `live-tests` disabled. Add a documented browser command when the repair checks are promoted; avoid committing an absolute dependency path as the only replay route.

The isolated `regression-review` runtime manifest already exists under `.dev/worktree-runtime/`. Recheck it before native smoke, build/start it through the worktree runtime tool, and keep it separate from `session-events-demo`. Starting the app is not evidence that a Workflow ran. Live-provider smoke, if later authorized, gets its own named scenario and evidence.

## Finding-to-package coverage

| Existing finding                      | Repair owners | Closing proof                                     |
| ------------------------------------- | ------------- | ------------------------------------------------- |
| B1: second message blocked            | R1/R8         | V1/V2/V13                                         |
| B2: connection triggers unwired       | R4/R5/R10     | V5/V8/V15                                         |
| B3: working directory lost            | R2/R3         | V3/V15 launch capture                             |
| B4: shared edits block pinned Session | R3            | V4                                                |
| B5: invalid source/trigger pairs      | R4/R6         | V7/V10                                            |
| F1: activation erases edits           | R6            | V9                                                |
| F2: save response erases newer input  | R6            | V9                                                |
| F3: node widens profile               | R3/R6         | V10                                               |
| F4: hidden invalid default            | R3/R6         | V10                                               |
| F5: changed instance / old result     | R2/R7         | V12                                               |
| F6: draft lost across navigation      | R6/R7         | V9/V12                                            |
| F7: identity entry removed            | R8            | V13                                               |
| F8: stale delivery history            | R4/R8         | V13                                               |
| U1: canvas removed                    | R7            | V11                                               |
| U2: instance creation/list missing    | R2/R7         | V3/V12                                            |
| U3: narrow picker hidden              | R7/R9         | V14                                               |
| U4: collapse CSS broken               | R9            | V14                                               |
| U5: oversized checkbox                | R9            | V14                                               |
| Stale implementation status           | R10           | Documentation/source cross-check at repair commit |

## Completion record

For each package record: commit, commands, pass/fail, observed behavior and untested limits. Do not overwrite the baseline screenshots/results to make them look like repair proof.

- [x] G0 consumed contracts and planning defaults confirmed in implementation.
- [x] R1–R5 have local functional proof through real services and fake inference.
- [x] R6–R9 have mounted component and real-browser proof.
- [x] R10 service composition and SQLite reopen passed.
- [ ] Native build and isolated no-provider smoke recorded.
- [ ] No unexplained failures, duplicate old/new routing or unexplained disabled functionality remain; any uncommitted work is identified in the handoff.
- [x] Documentation identifies the repair state and remaining deferrals.
- [ ] User has reviewed the repaired build (separate from technical completion).

The native build passed; its window-level smoke remains unverified. The full Rust run had one timeout, which passed in isolation, so that run is not recorded as all-green. User review remains separate. The earlier 980-test frontend pass and nine reproduced bugs are baseline evidence; the repair now has its own tests and screenshots.
