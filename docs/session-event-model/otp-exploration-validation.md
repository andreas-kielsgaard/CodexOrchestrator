# Workflow OTP exploration validation

2026-09-08. Implemented locally on `codex/workflow-continuation-files`, based on
`f893e615feb0caf3f68ca5a74461af33fb8dfa22`. Changes are uncommitted. No merge, release,
main-checkout modification or user-database reset was performed.

## Result and ownership

The product explicitly imports one compiled Workflow OTP. Its declarations drive managed MCP
exposure and the designer catalogue. `otp_api/` is the narrow public contract; `otp_host/`
adapts product services; `otp_packages/workflow/` implements continuation, handoff,
completed-invocation consumption and destination-session selection using only that API.

Workflow observes and routes; the package supplies behavior and new/exact requests; the
common Session Event dispatcher owns actual session creation, prompting and delivery records.
Source-node/capability/output matching supports zero or multiple connections. Continuation
does not close a node or implement approval. File metadata still derives solely from stored
Harness file events, including historical editors and archived sessions.

Recipe contract 2 and the managed `workflow` server replace their predecessors directly.
The duplicated Workflow backend/frontend, special proxy participation protocol and fixed
trigger catalogue are removed. Existing unsupported database records are retained, omitted
from current lists and rejected explicitly when loaded by ID. No translators or aliases remain.

## Automated checks

| Check | Result |
| --- | --- |
| OTP package contracts | 5 passed |
| Workflow compiler, storage and file inputs | 14 passed |
| Shared Session Events | 17 passed |
| Harness engine | 33 passed |
| Storage migrations/preservation | 23 passed |
| Agent Session repair/integration | 14 passed; 1 live test ignored in this focused run |
| Codex Harness file-change normalization | 1 passed |
| Frontend authoring, Session Events, clients and product navigation | 17 passed across 10 files |
| Production Rust `cargo check --lib --profile test-fast` | Passed |
| TypeScript and Vite `npm run build` | Passed; existing large-chunk warning remains |

The seven focused Rust groups total 107 passing tests. Commands used `cargo test --profile
test-fast --features live-tests --lib <filter> -- --nocapture` with filters `otp_packages::`,
`workflows::`, `session_events::`, `harness_engine::`, `storage::tests` and
`agent_sessions::application::tests::repair_tests` and
`runtime::codex::protocol::file_change_tests`. The full unrelated Rust suite was not run.

Frontend command: `npm test -- src/features/workflowAuthoring src/features/sessionEvents
src/infrastructure/workflowAuthoring src/infrastructure/workflowInstances
src/app/App.sessionEventModel.test.tsx`.

Boundary coverage includes:

- Import omission/duplicates, descriptor-to-designer fixture parity, unchanged MCP arguments
  with independent trusted context, and API-only package execution.
- Managed proxy through real SQLite to fake provider: continuation fan-out/zero matches,
  handoff typed arrays, normal completion, repeated-notification deduplication and source scope.
- New sessions despite existing candidates; exact-session follow-ups; initial prompt once;
  busy-session rejection; selected requests and failures retained before/after dispatch.
- Compiler rejection of missing imports, outputs, actions, fields, bindings and invalid config.
- Historical/archived file associations, unchanged authorship for output-file arguments,
  file-read failures and pinned profile/recipe behavior after later authoring changes.
- Designer save/reload/activation, declaration-derived fields, file inputs, canvas navigation,
  and main-pane delivery inspection with multiple recorded event groups.

## Real managed-Codex exercise

The ignored `real_codex_continuation_exercise` test was run explicitly with the compatible
bundled `codex-cli 0.153.0-alpha.5`, using the existing authenticated model configuration.
It passed in 158.82 seconds. The disposable evidence directory is:

`C:/Users/user/.codex/tmp/otp-live-20260908-01/`

| Observed fact | Result |
| --- | --- |
| Real invocations | 8 completed |
| Sessions | 5 across Discussion, Overview, Clarification and Revision |
| Continuation calls | Discussion: 2 deliveries; Clarification: 1; Revision: 0 |
| Harness create/edit records | 7 |
| Recorded attempts / event groups | 8 / 8, delivered without errors |
| Fresh/exact selection | A second Discussion session was created despite an existing one, then prompted by its exact ID |

Discussion waited until the test's approval prompt before writing `spec.md` and invoking the
tool. Approval was an agent instruction, not engine enforcement. The call fanned out to
Overview and Clarification. A later clarification updated `decisions.md` and triggered
Revision. Revision received three node-file inputs with creator/editor/session metadata.
Its later edit of `spec.md` preserved Overview's historical editor association. The supplied
`claimed-only.md` argument created no file-history record.

An independent read-only SQLite/JSON verification checked invocation/session counts,
typed payloads, historical attribution and fresh/exact identity. No exercise provider process
remained after shutdown. Retained artifacts include `verified-results.json`, `catalogue.json`,
`recipe.json` (the full instance), `sessions.json`, `node-sessions.json`, `attempts.json`,
`deliveries.json`, `groups.json`, `history.json`, `historical-editor-selection.json`,
`live.sqlite` and the test documents. The earlier dated continuation report remains historical.

This is real provider/MCP/file-event evidence with an in-process managed proxy host.
Native child-sidecar startup and the native Tauri window were not exercised. Handoff and
completion-consumer behavior have managed-host/fake-provider coverage; they were not separate
live-provider scenarios. The final code cleanup only reused the existing node-address helper;
Workflow checks were repeated afterward. The later inspector layout change has frontend
test/build and visible-browser coverage.

## Visible designer walkthrough

Used the actual React designer and instance-panel components in a visible in-app browser at
`127.0.0.1:1488`, backed by the serialized real OTP catalogue. A temporary browser harness used
local recipe storage and the recorded live delivery JSON; it did not claim native command
transport. The server was stopped and the harness archived as `browser-harness/` in the
external evidence directory, then removed from the checkout.

Observed through screenshots and UI interaction:

1. Opened a canvas connection and selected the declared Workflow continuation output.
2. Bound Reviewer as output node, selected Prompt agent with `new` mode, and observed selection
   controls disappear.
3. Added `sourceNode` and Author's Created-or-edited file input.
4. Saved/activated revision 2, reloaded, reopened and verified those values persisted.
5. Opened the recorded Clarification-to-Revision delivery, verified its new-session target,
   prompt contributions and creator/last-editor JSON.
6. Found and fixed a layout defect: delivery details occupied the narrow session sidebar.
   Rechecked the inspector in the main pane, expanded the prompt/file details and closed it
   to return to conversation selection.

This proves the visible component flow and browser persistence. Rust storage/transport tests
cover the backend separately; no native-window, packaged restart or full accessibility claim
is made.

## Implementation deltas and remaining scope

Two unused alternate panels, `WorkflowRunPanel.tsx` and `WorkflowOutline.tsx`, were deleted
instead of adapted. Shared instance-dialog CSS was moved alongside canvas/drag helpers before
deleting the old UI. Generic Session Event DTOs existed in both application and feature modules;
both now represent the new target. Obsolete event-definition reference helpers were removed,
and production uses the shared Workflow node-address helper. The visible delivery layout fix
was added after the walkthrough exposed it.

The first local OTP slice is complete within the evidence boundaries above. The package is
compiled in the product crate and registered by an explicit local factory. Node bindings and
file resolution remain product controls; only the requested narrow handles are exposed.
No external package format, granular permission system, arbitrary inference-event subscriptions,
new deterministic tools, session transcripts, approval enforcement or revision coordination
were added. Those would need separate scope if requested.
