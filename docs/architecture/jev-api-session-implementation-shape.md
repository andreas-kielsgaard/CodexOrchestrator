# Jev API sessions: implementation shape

Status: revised with `plan-implementation-shape`, then implemented on `feature/jev-integration`, based on main `79bc4eb0`. The voice branch tip `431803f8` is an ancestor of that base. Focused and live application validation passed; broader regression failures remain below.

## Target

Provide backend Jev API sessions for the reasoned voice feature. Reuse Agent Session identity, immutable configuration snapshots, invocation lifecycle, persistence, notifications, cancellation, and shutdown. Each invocation evaluates caller-supplied state and typed questions in a fresh HTTP request.

OTP functionality, session/provider UI, Technical Settings UI, and voice routing policy are outside this slice.

## Proposed choices

- Put Jev request/response contracts, HTTP behavior, and `AgentRuntime` implementation under `crates/orchid-engine/src/providers/jev/`, matching the existing Codex and Claude runtime ownership. Keep desktop configuration, registration, and application consumption in desktop modules.
- Use the existing Rust HTTP stack (`reqwest`, Tokio, and cancellation tokens). TypeSafe documents a direct HTTP API and Python/JavaScript SDKs; this slice needs no SDK bridge process or additional language runtime.
- Use `TYPESAFE_API_KEY` from Orchid's process environment and the fixed TypeSafe evaluation endpoint. Default to `jev-latest`; allow a Jev model ID when creating the session. Pin that selection for subsequent evaluations. Do not persist or transport the credential through session options.
- Provide focused typed `create`, `invoke`, and `load` methods on `AgentSessionApplication`, with thin Tauri commands for backend consumers. Callers receive durable session/invocation identities and read a typed result after completion.
- Keep native request serialization inside the Jev boundary. The application wrapper serializes `JevEvaluation` into the existing invocation input; the runtime decodes it. This avoids expanding every conversational input contract for one typed provider. The wrapper is the supported entry point; free-form sends, attachments, conversational context, tools, skills, reasoning controls, and native resume are unsupported.
- Store the native response as event evidence and the typed result under the final normalized message's `structuredOutput`. Consumers use the typed load method. Persist resolved model and token usage with the result.
- Reusing an invocation ID with the same input returns the existing acknowledgement. A different ID makes another evaluation. Never append previous Jev evaluations to the new API state; the caller supplies any desired history explicitly.
- Use a bounded request timeout and cancellable HTTP future, with no automatic retry. Cancellation stops Orchid's request handling; it does not claim to reverse vendor processing or billing.

## Decisions remaining with the voice feature

The voice caller owns questions, state assembly, thresholds, routing, and whether to reuse an Orchid session or create a new one. This integration does not prescribe those decisions. Jev has no vendor conversation ID to resume; repeated invocations in an Orchid session remain independent evaluations.

## Prescribed decisions

- Integrate with Agent Session infrastructure.
- Make Jev API sessions runnable without adding UI support.
- Add no OTP functionality.
- Base the feature branch on main if it contains the voice work, otherwise on the voice branch. Verified main contains it.

## Current code and reuse

`ProviderRegistrations` and `ExecutionEndpoints` already resolve runtime and configuration independently. Register Jev there, with no native launch or continuation implementation. Do not advertise setup records through `ProviderConfigurationSource::setups`; current UI route catalogues remain unchanged.

`SessionProfileResolver` already seals an immutable configuration snapshot. Build a minimal application-owned Jev profile at session creation: one model, no tools/skills/access modes. It does not require a saved, user-authored Capability Profile or a new database table.

`InvocationDeliveryService` already persists and dispatches application invocations, rejects concurrent active invocations, records launch acceptance, and supplies `PersistedRuntimeUpdateSink`. Use that direct application delivery path. Ordinary composer preparation and worktree switching are unnecessary for a typed API evaluation.

`voice_sessions/application.rs` currently delivers conversational input and state to a Codex/Claude session. Its future Jev caller can assemble `JevEvaluation`, submit through the typed facade, and consume the existing terminal notifications. This slice leaves that caller's behaviour unchanged.

`project_invocation_observation` currently assumes most provider activity comes from stdout. Recognize semantic final messages and usage from an HTTP adapter too, without treating an attempted HTTP launch as provider activity.

## Concrete ownership and changes

| File | Responsibility/change |
|---|---|
| `crates/orchid-engine/src/providers/jev/contracts.rs` | Typed evaluation, question, answer, usage, and result validation. |
| `crates/orchid-engine/src/providers/jev/client.rs` | Fixed API protocol, authentication headers, bounded response reads, sanitized HTTP failures. |
| `crates/orchid-engine/src/providers/jev/runtime.rs` | Async HTTP invocation ownership, supported options, cancellation, terminal delivery, shutdown. |
| `crates/orchid-engine/src/providers/jev/mod.rs` | Provider exports and identity. |
| `crates/orchid-engine/src/providers/mod.rs`, `crates/orchid-engine/Cargo.toml` | Declare Jev and reuse the HTTP/runtime dependencies already used by the desktop. |
| `src-tauri/src/runtime/providers/jev/configuration.rs` | Model-specific environment configuration snapshot; no native setup discovery. |
| `src-tauri/src/runtime/providers/jev/mod.rs` | Re-export engine types and register runtime/configuration. |
| `src-tauri/src/agent_sessions/application/jev.rs` | Typed creation, idempotent invocation delivery, typed result lookup. |
| `src-tauri/src/agent_sessions/transport/jev.rs` | Thin backend commands; no views or navigation. |
| `src-tauri/src/active_app/execution_configuration.rs`, `src-tauri/src/active_app.rs` | Register provider parts and backend commands. |
| `src-tauri/src/agent_sessions/application/observation.rs` | Normalize HTTP provider activity from semantic events. |
| `src-tauri/src/agent_sessions/repository/tests.rs` | Update the runtime-source message observation expectation to recognize provider activity. |
| `src-tauri/src/runtime/providers/jev/tests.rs` | Mock API/session persistence, invocation identity, error, cancellation, shutdown, and restart validation. |

Add module declarations in the existing application, transport, and provider module roots. Update both Cargo lockfiles for the engine's HTTP dependencies. Retain existing persistence schemas, delivery services, and voice implementation.

Move the initially created desktop Jev contracts/client/runtime into the engine. Remove those competing desktop implementations; keep desktop configuration and session integration there. No existing Codex or Claude implementation needs extraction for this slice.

## Delivery and validation

1. Finish the engine/desktop ownership split and typed facade.
2. Verify a mocked API evaluation through the actual SQLite session/invocation path, including all three question types, usage, resolved model, and result readback after reopening storage.
3. Verify duplicate invocation IDs do not repeat an API call; a subsequent invocation sends only its explicit state. Validate missing credentials, HTTP failures, malformed results, timeout, cancellation, and shutdown.
4. Run the Rust session/provider regression suite and compile the composed application. Check the provider source boundary and whitespace.
5. Document the backend calls and environment requirement. A live vendor call requires a configured key; none was present when inspected. Report live verification separately from mocked lifecycle validation.

## Validation results

- Final Jev integration tests: 8 passed, covering actual SQLite delivery/readback and mocked HTTP.
- Shared engine suite: 82 passed, 1 ignored.
- Final session repository tests: 20 passed. Final observation tests: 4 passed.
- Composed application Rust compile check and diff whitespace check passed.
- The broader desktop run completed with 828 passed, 6 failed, 1 ignored. Its runtime-source observation expectation was updated and passed in the final repository run.
- Five broader failures remain: the inventory JSON assertion omits `createdAt`; the storage table assertion omits `voice_session_state`; three Codex app-server interaction tests expect accepted/uncertain steering or successor launch that does not occur. The inventory/storage source is unchanged from main. All three Codex failures also reproduced in the earlier build before the engine split and observation change.
- Initial process environment had no key. On the user's later request, the existing Firefox Jobnet plugin credential was read locally and passed directly into an isolated release executable's environment, without printing or persisting it in Orchid.
- Pre-merge release executable: the application CLI submitted one real evaluation containing Noul, Choice, and Score questions. The durable invocation completed with model `jev-1.13.0`, all three typed answers, and usage of 414 input / 69 output tokens. Replaying the same ID returned the same result with one invocation and three events retained.
- `review-tools/app-inspector/jev-query.mjs` consumes the existing registered native commands. It adds an executable-level CLI workflow without another product transport or a direct TypeSafe call.
- No Jev UI flow or voice routing behaviour was exercised or changed.

The backend consumption examples are in [Jev API sessions](../jev-api-sessions.md).

Source contracts: [TypeSafe API reference](https://docs.typesafe.ai/api), [TypeSafe models](https://docs.typesafe.ai/models), checked 2026-10-07.
