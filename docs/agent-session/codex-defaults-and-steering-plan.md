# Session defaults and interactive Codex execution

Planning revision 1 — 2026-09-08. Assessment and proposed implementation only.

## Target and evidence baseline

Make Agent Sessions the clear owner of session handling, with profile-derived defaults, native Codex inheritance, retained working directories, turn steering, and runtime approval/input interactions. Keep workflow decisions in Workflows and technical tool mediation in the Harness Engine.

Baseline: `codex/workflow-continuation-files`, commit `9502879eff3b156c32662314776541e255b5c1ab`. This includes the OTP introduction at `532e559`. The OTP picker/Stop session work described in `../session-event-model/otp-element-selection-plan.md` is actively changing the worktree. Its edits are not part of this plan's implementation evidence and must be preserved. Recheck that work before implementing overlapping configuration, notification, cancellation, or composition changes.

This assessment inspected source, the installed `codex-cli 0.144.0`, and its previously generated app-server schemas. It did not run a provider invocation or product tests. The earlier live continuation report is historical evidence for the old transport, not proof of this design. The [official app-server documentation](https://learn.chatgpt.com/docs/app-server) corroborates the steering and request/response protocol; the selected executable's schema and behavior remain the compatibility boundary.

Accepted product requirements:

- The capability view requires one saved Capability Profile to be designated as the default. Standalone creation uses it. A workflow node supplies its associated Capability and Node Profiles.
- Profiles determine session defaults and automated workflow configuration. They do not impose a ceiling on direct user choices. Preserve the existing broader direct-user model/reasoning behavior.
- Unspecified settings inherit Codex defaults. No additional product-wide execution-settings layer is introduced.
- Steering adds text to the active invocation and is a distinct event recognized by the workflow engine. It has no additional workflow effect in this build.
- Approval and user-input requests must be answerable, or produce a specific integration limitation without leaving the invocation silently waiting.
- A new session without a working-directory target receives a retained empty directory under an orchestration-owned user-home folder. Orchestration skills have a sibling home there.
- Built-in browser integration, attachment UI, Astra support, CLI upgrades, and sharing task histories across products are outside this build.

## Current structure and problems

There is already an appropriate session area. `src-tauri/src/agent_sessions/` owns identity, invocation lifecycle, history, storage, and the provider-neutral runtime port. `src-tauri/AGENTS.md` explicitly assigns these responsibilities there. Extend that area; do not create a competing conversation manager.

| Current surface                                               | Finding                                                                                                                                                      | Required change                                                                                                                              |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `agent_sessions/application/lifecycle.rs`                     | Approximately 1,400 lines combine creation, preference updates, launch preparation, idempotent delivery, cancellation, restart recovery, and error handling. | Split by use case while retaining one application facade and the existing lifecycle invariants.                                              |
| `agent_sessions/application/session_profile.rs`               | Standalone creation synthesizes `application:standalone`; profile-aware sending wraps the general send API and emits Codex reasoning arguments.              | Use the persisted default profile; consolidate configuration resolution; move Codex serialization into the runtime adapter.                  |
| `agent_sessions/session_event_adapter.rs`                     | Address SQL, profile resolution, session creation, identity assignment, and delivery launch are combined. It duplicates direct-user selection resolution.    | Keep an adapter over Session Events ports; move address SQL into session persistence and delegate creation/invocation to session use cases.  |
| `execution_configuration/native_codex.rs` and `active_app.rs` | Runtime exposure is injected fixture data. A product execution-mode selection is represented as a runtime sandbox lock.                                      | Observe native capabilities/defaults and distinguish them from genuine provider restrictions and product choices.                            |
| `execution_configuration/capability_profile.rs`               | Stores allowed-capability sets, but has neither a designated-default relationship nor scalar starting defaults.                                              | Add the required default selection and explicit default/inheritance semantics to the existing configuration model.                           |
| `harness_engine/session_binding.rs` and `service.rs`          | Any pinned profile creates session-wide MCP mediation. Mediation clears inherited MCP configuration and uses a fixed profile digest.                         | Project the configuration for the invocation being launched; preserve native tools unless explicit configuration replaces them.              |
| `runtime/codex/` and `runtime/processes/`                     | Runs `exec --json`; child stdin is null. Process ownership and provider completion are coupled through invocation lifetime.                                  | Add duplex app-server transport and distinguish turn completion from connection/process termination.                                         |
| `agent_sessions/application/observation.rs`                   | A terminal invocation is currently projected as process-terminal evidence.                                                                                   | Represent invocation completion independently; report process exit only when actually observed.                                              |
| `native_profiles.rs`                                          | Selecting a home for consumers also demands authentication, sandbox initialization, workspace-write canary, and product MCP readiness.                       | Separate home identity/continuity from operation-specific readiness. Native-default sessions must not require unrelated orchestration setup. |
| Session React controller/composer and profile client          | Multiple send routes; input disabled throughout execution; no runtime-request response surface.                                                              | One session command client/controller with explicit send, steer, cancel, and respond operations.                                             |
| `workflows/event_sources.rs`                                  | Consumes session notifications through the OTP integration; presently recognizes invocation-terminal notifications only.                                     | Recognize steering as a separate session fact, with an explicit no-action case.                                                              |

## Responsibility boundaries

| Owner                    | Owns                                                                                                                                                            | Receives or delegates                                                                                                                                |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Agent Sessions           | Session and invocation identity, workspace assignment, creation defaults snapshot, ordered inputs, runtime requests, lifecycle, durable history, notifications. | Receives creation intent and input provenance. Calls configuration resolution and the runtime port. Does not compile workflows or encode Codex JSON. |
| Execution Configuration  | Capability/Profile vocabulary, default-profile selection, inheritance rules, resolution of starting defaults and invocation choices, configuration provenance.  | Reads observed runtime information. Node definitions remain stored with workflow authoring. Does not launch processes.                               |
| Native Codex integration | Selected executable, home identity, observed capabilities/defaults, Codex protocol/config projection.                                                           | Preserves existing home binding/continuity. Reports unsupported or unknown features truthfully.                                                      |
| Harness Engine           | Product-managed MCP endpoints, selected tool exposure, grants and cleanup for the concrete launch.                                                              | Consumes a technical exposure plan. Does not decide whether a user's choice is allowed by a workflow node.                                           |
| Session Events           | Addressing, prompt materialization, delivery identity, fan-out and delivery records.                                                                            | Uses the Agent Session directory/dispatcher adapter. Does not own live-turn steering.                                                                |
| Workflows and OTP host   | Node/instance meaning, automated defaults, configured continuation, interpretation of session facts.                                                            | Calls session commands through existing host boundaries. User steering is recognized without dispatching another action.                             |
| Process infrastructure   | Spawn, duplex handles, serialized writes, readers, terminate/reap, shutdown ownership.                                                                          | Emits process facts. Does not decide that an agent turn succeeded.                                                                                   |
| Tauri/React transport    | Commands, read models, request cards, composer state, transcript presentation.                                                                                  | Uses shared session APIs; neither frontend nor DTO mapper resolves execution policy.                                                                 |

The dependency shape remains:

```text
Agent Session UI --------------------+
                                    v
Workflow / OTP -> Session Events -> Agent Session application
                                    |          |
                         Execution Configuration
                                    |          v
                              Harness plan -> AgentRuntime port
                                               |
                                          Codex adapter
                                               |
                                        Process supervision

Runtime updates -> Agent Session persistence -> UI + Workflow observers
```

Use composition helpers beneath `active_app.rs` for constructing these groups. Keep `active_app.rs` as wiring. Extract its session-notification fan-out from `ManagedPlanBuilderNotifier` into a neutrally named composition adapter; retain the existing consumers without making a Plan Builder-specific type the owner of all session notifications.

## Configuration behavior

Use the existing Session Profile concept for immutable creation provenance and default selections. Revise the claim that it freezes every available runtime choice forever. Store explicit product selections and inherited references separately from observations of what Codex actually used.

For scalar defaults, an absent product selection means inherit; never materialize today's native value as a permanent product override merely to populate the UI. For tool/skill sets, distinguish inheritance/addition from explicit replacement, including an intentionally empty replacement. An empty list cannot mean both inherit everything and expose nothing.

Resolution order for a new invocation is the attached Codex environment, the saved session defaults derived from Capability/Node Profiles, then explicit invocation choices. Direct-user choices are validated against actual runtime support and actual runtime constraints, not the profile's narrower set. Automated deliveries use the saved workflow defaults. Invocation choices do not rewrite the creation snapshot or alter later automated deliveries.

Inherited native values are refreshed at invocation boundaries. Changes to which Capability Profile is designated default affect subsequent session creation; edits to a Capability or Node definition do not silently rewrite existing sessions. An existing session's inherited fields still follow its bound Codex environment. These rules reconcile native synchronization with immutable product provenance.

Add one durable default-profile reference owned by Execution Configuration, rather than duplicating a Boolean across profiles. Updating the reference and deletion checks must be atomic. New standalone creation reads that reference and profile revision consistently. If no default is configured, show the required configuration state in the capability view and block new standalone creation with a useful route there; do not silently choose an arbitrary existing profile. Existing histories remain readable and existing sessions are not dependent on the current default pointer.

Replace the fixture runtime source with adapter-owned discovery. Keep observed model/reasoning support, native tools/skills, and product OTP capabilities distinguishable by source. Incomplete discovery is unknown, not an empty allowlist. Do not prevent an inherited native-default launch solely because a catalogue is incomplete. Do not advertise a model as usable solely because it appears in a hardcoded list. Astra remains outside acceptance.

User freedom must survive launch as well as validation: the harness cannot silently reapply a narrower saved MCP set after a user's supported override was accepted. Project bindings for the selected invocation configuration and preserve the immutable baseline. Retain authenticated workflow context on product tools; changing technical choices does not change the session's workflow identity.

Native MCP configuration stays owned by Codex. Product MCP entries are additions unless a Capability/Node selection explicitly specifies replacement. Avoid name collisions and never copy credentials into UI configuration provenance. Preserve CLI-discovered skills/hooks/plugins through native inheritance. Discovering an installed plugin is not proof that every desktop capability can execute in this host. Additional orchestration skill exposure must use a verified adapter mechanism; folder placement alone is not enough.

## Session application and storage

Retain one `AgentSessionApplication` facade, implemented in focused modules:

```text
agent_sessions/
  application/
    creation.rs          # default-profile or supplied node intent; workspace; persistence
    invocation.rs        # prepare/start/resume; provenance; delivery idempotency
    interaction.rs       # steer and runtime-request responses
    configuration.rs     # calls the shared resolver; no CLI argument strings
    lifecycle.rs         # cancel, availability, recovery and shutdown
    observation.rs       # independent execution/provider/process facts
    update_sink.rs       # ordered persistence before notification
  ports/
    repository.rs
    runtime.rs
    workspace.rs
    notifications.rs
  repository/
    addressing.rs
    inputs.rs
    interactions.rs
    ...existing storage...
  session_event_adapter.rs  # thin directory/dispatcher translation
  transport/               # DTOs and commands only
```

These are proposed ownership cuts, not a requirement for one interface per file. Keep the small domain together unless added input/request types make a focused split useful. Move code with its tests; do not create new abstractions merely to shorten files.

Add durable steering-input records with a stable input ID, session/invocation correlation, order, text, and delivery state: pending, accepted, rejected, or uncertain. Preserve the original invocation input and provenance; steering does not rewrite an application invocation into a user invocation. Link accepted provider echoes where supported to avoid rendering the same input twice. Retain the one-active-invocation database constraint.

Add provider-neutral runtime-request records for supported approval and question types, their offered choices, lifecycle, and response. Correlate them to the runtime connection generation as well as session/turn/request identity. Reused numeric JSON-RPC IDs on a later connection must not make old cards actionable. Resolve/expire cards on provider resolution, cancellation, terminal turn, or connection loss. Persist only the fields needed for interaction/history, not arbitrary authentication material.

All external writes occur outside database transactions and notification locks. Persist the intent, perform the runtime call, then record its acknowledgement or uncertainty. A process crash or lost response is not proof that input was rejected. Do not automatically resend uncertain steering or approvals.

Revise the requested/effective configuration read model: show product default source, user choice, and runtime-observed values separately. Do not label a preflight prediction as an observed native default. This is a concrete invocation record, not a universal mutable configuration object for the product.

## Working directories and orchestration home

Proposed home layout:

```text
C:\Users\user\.codex-orchestrator\
    workspaces\<session-id>\
    skills\<skill-name>\SKILL.md
```

The existing Tauri application database stays in its current location. The selected `CODEX_HOME` also stays unchanged. This work does not migrate all product state or duplicate Codex authentication/configuration.

A filesystem adapter allocates the workspace; session creation owns when that happens and persists the absolute path before launch. Use the persisted path for later turns and application restarts. Explicit targets remain explicit. Creation retries reuse the same owned allocation; prevent collisions across isolated product instances. The home resolver is injectable so tests and development instances can use temporary roots without introducing another user settings page.

Keep generated session directories empty: do not create hidden instructions or copy skills into them. Skills live in the sibling catalogue and are exposed through the relevant configuration. Do not treat cancellation or archive as permission to delete a workspace. Filesystem allocation and session/address persistence must preserve the existing idempotent creation boundary; failed persistence must not lead to recursive deletion of a directory that could now contain user files.

For historical sessions with a blank directory, first recover a reliable prior cwd from native thread metadata or recorded evidence. Persist that context if established. If it cannot be established, present an explicit compatibility state requiring a target before continuation; never silently move an existing conversation into a fresh directory. Versioned profile decoding must likewise preserve existing snapshots/digests rather than retroactively applying the newly chosen default.

## Interactive runtime

Add an app-server adapter under `runtime/codex/`, with separate connection, protocol mapping, configuration/discovery, and interaction handling. The AgentRuntime port gains typed steering and runtime-request response operations and semantic support reporting. It never exposes JSON-RPC IDs or CLI argument arrays to React or workflows.

The initial implementation should use one app-server connection for each active invocation. Initialize it, start/resume the external thread, start the turn, and retain the connection for steering and runtime requests. After the terminal turn and required final events settle, release invocation resources and shut down/reap the connection. Subsequent invocations resume the same external thread with freshly prepared configuration. This avoids a shared process whose global configuration leaks between independently configured sessions. Prove whether this lifetime preserves relevant native state before committing to it; do not promise background-terminal persistence in this build.

The existing process supervisor remains the owner of children. Extend its factory/handle boundary to support stdin and serialized writes with reader processing independent of response waits. Keep process-exit handling separate from the runtime turn state machine. A successful process exit without an observed turn result does not imply a completed turn; a process cleanup failure after a completed turn does not fabricate a second workflow completion. Preserve shutdown failure reporting and the documented direct-child ownership limit.

Map Codex's external thread identity to the existing session binding and its active turn identity to the invocation. Resume must be tested with a conversation created by the current `exec` adapter. Record actual turn acceptance separately from process spawn and persist the external turn binding before advertising that steering is available.

The protocol sends steering with the expected active turn identity. On success it remains the same invocation. If the target turn has finished, return a stale-turn result and retain the user's draft/input; do not start another turn automatically. Mid-turn model, cwd, and sandbox changes are not supported by this steering operation. Show configuration changes as choices for a subsequent invocation, not as changes already applied to the running one.

Handle command/file/permission approvals and tool questions using typed interaction variants. Support the MCP elicitation forms that the chosen runtime and UI can actually answer. Unsupported request variants receive a protocol-appropriate decline/error and a visible limitation, rather than being ignored. Do not auto-approve requests to simulate native defaults. Client-owned tool calls or host-specific auth requests are not evidence of support merely because they exist in the schema.

Normalize streaming messages, usage, tool progress, and successful file-change facts into the existing history model. Proposed or denied file edits are not completed file-history entries. Preserve the authoritative file-change attribution used by workflow file inputs.

Remove the active session dependency on `exec` argument generation after migration. Existing native setup/canary commands may still legitimately use exec. Inventory live `RuntimeLaunchExtension` producers in `orchestration/application.rs`, `bootstrap_transition.rs`, `sprint_runner_transition.rs`, and `work_unit_execution_harness.rs`: migrate their current configuration to typed inputs, or isolate an exact compatibility translator at the Codex boundary. Do not maintain two silently selected session runtimes or ignore old launch flags during the switch.

## Workflow and UI integration

Add a typed steering-accepted notification containing the session, invocation, input identity and ordering. The workflow event-source adapter recognizes it explicitly and returns without invoking an OTP, materializing prompts, creating a delivery, or triggering continuation. Failed/uncertain submissions remain input-history facts, not accepted-steering notifications.

The existing `SessionEventCommand` describes dispatch/fan-out. Do not route steering through that command solely to call it an event. Keep session observations distinct from requests to activate a workflow. No new selectable steering trigger or OTP action is needed for this build. Durable session input history remains the source if a later workflow feature consumes steering events.

Use the same session command client and controller in Agent Sessions and embedded/profiled panes. The composer sends a new invocation while idle and explicit steering while an active turn is available. Waiting-for-turn-start, steering-pending, stale-turn, and cancellation states have distinct feedback. A failed steering submission retains the text. Runtime request cards remain answerable while streaming continues, and reloading the view reconstructs pending cards from session queries. Keep model/reasoning selectors based on runtime choices rather than node profile membership.

The capability view owns the required default-profile selector and inherited/explicit default fields. Reuse the ongoing OTP picker components and their runtime-tool groups when that work stabilizes. Do not build a competing MCP selector. Revise wording such as "allowed ceiling" where it incorrectly describes the options of a user interacting with a session.

## Implementation sequence and acceptance

| Step                                                    | Outcome and scope                                                                                                                                                                                                                               | Dependencies                                             | Acceptance evidence                                                                                                                                                                                                                                                              |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1 — Establish executable contracts                     | Confirm app-server start/resume/steer/interrupt, native configuration inheritance and request variants on 0.144.0; decide per-invocation connection lifetime; inventory old extension consumers.                                                | Current baseline and disposable test context.            | Version-labelled schemas/fixtures and a bounded protocol exercise; ordinary existing-thread resume, inherited values after a profile change, and supported tool configuration have explicit outcomes. No Astra requirement.                                                      |
| S2 — Consolidate session use cases                      | Split creation/invocation/lifecycle responsibilities, extract address persistence and notification fan-out, introduce typed runtime inputs, and remove duplicate configuration decisions. Preserve current behavior before switching transport. | S1's producer/consumer contract.                         | Existing session creation, idempotent deliveries, native-home continuity, cancellation and restart tests pass through the consolidated application API. No SQL or Codex argument assembly remains in the session event adapter.                                                  |
| S3 — Implement defaults and workspace ownership         | Required default-profile reference; inheritance/default fields; observed runtime source; invocation-local user choices; retained workspace allocation and historical compatibility.                                                             | S2; S1 discovery findings.                               | Default selection survives reopen; replacement/deletion invariants hold; native changes affect inherited fields; explicit choices stay explicit; both standalone and node creation persist cwd; historical sessions are not silently relocated.                                  |
| S4 — Implement interactive Codex execution              | Duplex process transport, app-server adapter, external turn correlation, approval/input requests, persisted steering states, turn/process observation separation.                                                                               | S1 and S2; integrate S3's typed configuration.           | Fake-server tests cover interleaved responses/events, wrong or stale turn, multiple steering inputs, unsupported requests, cancellation, disconnect, restart and late events. Same invocation is retained; acknowledgements and outcomes are not conflated.                      |
| S5 — Converge tools, workflow and product UI            | Invocation-specific harness plans; inherited native tools plus configured OTP exposure; workflow steering observation; shared composer/controller; request cards; capability default UI.                                                        | S3 and S4; stabilized OTP picker/Stop session contracts. | A user can select supported choices outside the profile defaults; later automated delivery retains its defaults. Accepted steering is recognized once and causes zero extra workflow deliveries. Native tool inheritance, OTP continuation and file provenance survive together. |
| S6 — Prove the integrated build and retire replacements | Migrations, existing-thread resume, native end-to-end/UI evidence, documentation and obsolete active-path removal.                                                                                                                              | Coherent S5 candidate.                                   | The acceptance matrix below passes on the actual selected CLI. Obsolete send routes/fixture configuration/exec-session dispatch are removed or explicitly confined to necessary compatibility.                                                                                   |

S3 and the transport portion of S4 can proceed independently after the S2 contract is stable. Both converge at S5; schema and composition edits need one owner at integration. S1 is the first work unit because native resume and inheritance semantics could otherwise invalidate a large refactor. This plan does not launch or delegate implementation.

Integrated acceptance scenarios:

1. Choose a default Capability Profile, create an unattached session, inspect the actual native settings and its empty absolute cwd, exchange another turn, restart, and reuse the same external context and directory.
2. Change an inherited Codex setting between invocations; observe the new value without editing a product standard. An explicit product default or user choice remains distinguishable. Missing/default-profile changes do not rewrite historical sessions.
3. Create a node-managed session; send direct user input using a supported option outside that node's defaults; return to an automated delivery and verify the original defaults and workflow identity.
4. Steer an active standalone and node-managed invocation more than once. Observe one invocation, ordered input history, separate steering notifications, and no extra workflow delivery. Exercise completion-race and lost-acknowledgement handling.
5. Trigger and answer supported approval/question requests; exercise decline, stale response, unsupported request, view reload and runtime disconnect. No response is silently replayed or approved.
6. Observe an inherited native MCP tool alongside configured product OTP tools; exercise explicit replacement separately. Verify an orchestration skill through the selected runtime mechanism and preserve native discovered skills/hooks. Record specific host-dependent limitations.
7. Repeat the existing real continuation/file-input scenario on the new adapter: trusted calling-node context, correct fan-out, same-session continuation, successful edit attribution, and historical queries after reopening storage.
8. Exercise Cancel from the shared UI and the ongoing OTP Stop session action. Both use session handling and target the intended invocation; neither targets a later turn after a race.

Use focused Rust and frontend tests at each owning step. At convergence run the repository's frontend build/tests/lint and relevant Rust suite, then a disposable live Codex exercise with a supported model and visible product interactions. Unit tests are not substitutes for provider, packaged/restart, or UI evidence. Record those categories separately. No live test or implementation was performed while writing this plan.

## Removal and completion criteria

Remove the synthetic `application:standalone` creation path, duplicate direct-user resolution, Codex reasoning argument assembly in Agent Sessions, production fixture capability exposure, unconditional inherited-MCP clearing, and the assertion that every terminal invocation proves a process exit. Consolidate overlapping session clients rather than extending each independently.

Preserve versioned readers and historical evidence where required. Retain old harness/native setup code only for verified remaining consumers; do not use this slice to rewrite the quarantined application in `lib.rs`. Update the conceptual model and module comments with the actual ownership and new default-versus-user-choice semantics once implemented. No blanket new AGENTS or skill instructions are needed.

The result is complete when a reader can locate session creation, invocation, interaction, persistence, native protocol, tool mediation, and workflow reaction in their owning areas; each operation has one active route; and the integrated scenarios demonstrate the requested behavior. A smaller composer patch or merely replacing `exec` with `app-server` does not meet this target.
