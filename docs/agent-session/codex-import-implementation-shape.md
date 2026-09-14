# Import from Codex: implementation shape

Status: implemented and locally validated on 2026-09-14 in `feature/import-conversation-from-codex`.

Planning baseline: local HEAD `8965039`. Implementation worktree baseline: `3ecfd94`, including repository navigation, 2026-09-14. Existing documentation edits are separate. This plan records the agreed independent import and the proposed implementation boundaries.

## Functional target

Add **Import from Codex** beside session creation in Agent Sessions. Its modal accepts a link such as `codex://threads/01a0a149-469d-74d3-9924-e7415b61a4e9`, reads the source, and previews its title, working folder, and available history. Confirming creates a native Codex fork, populates Orchid's relevant tables, selects the imported session, and displays its history. No prompt is sent during import.

The fork is the continuation context; the original conversation stays independent. No synchronization, desktop bridge attachment, background polling, or promise of Codex desktop visibility. A conversation fork does not copy workspace files or create a Git worktree.

Initial scope is a locally accessible thread in Orchid's selected native Codex home. Show that home in the preview. An unavailable thread is an actionable error, not a reason to search unrelated homes or introduce remote-host support. Retain the source working folder; if unavailable, automatically allocate an empty workspace using Orchid's normal new-session behavior. Do not offer a folder choice. Use Orchid's default Capability Profile for future turns and show that choice in the preview.

Import produces an ordinary standalone session, without Workflow ownership, a logical Workflow address, or an inferred identity. If the repository-navigation plan lands first, use its ordinary Unfiled placement; a working folder alone does not imply organization.

## Evidence that determines the shape

| Existing responsibility | Consequence |
| --- | --- |
| `runtime/codex/app_server/mod.rs` owns one supervised process per invocation and performs start/resume followed by turn/start. | Do not add import branches to invocation initialization. Import needs a separate provider operation. |
| `app_server/environment.rs` already owns a bounded supervised RPC connection for discovery. | Extract that connection lifetime for discovery and history operations to share. |
| `agent_sessions/application/configuration.rs` resolves default Capability Profile settings before creating a session. | Reuse preparation without immediately inserting a session. |
| `native_profiles/session_binding.rs` refuses resume without a durable native-profile binding. | Import must establish the binding before the first Orchid prompt. |
| `agent_sessions/repository/mapping.rs` already has transaction-level session and invocation insert helpers. | Reuse them in an atomic history import, bypassing live lifecycle commands. |
| History reads and `transcriptProjector.ts` consume invocation/event records, sorted by creation time and ID. | Persist real history and explicit source ordering; a runtime thread ID or raw JSON blob alone is insufficient. |
| `application/observation.rs` distinguishes launch, provider, and process evidence. | Imported completion must not manufacture an Orchid launch or process exit. |

## Ownership and file changes

Paths below are relative to the repository; Rust paths are beneath `src-tauri/src/`.

### Frontend

- Create `src/application/agentSessions/importContracts.ts` with preview/import request and result DTOs and a narrow `AgentSessionImportClient`. Export it from the existing index. Ordinary conversation and embedded-session clients need not acquire import methods.
- Create `src/infrastructure/agentSessions/tauriAgentSessionImportClient.ts`. It translates calls to two Tauri commands; it owns no import policy.
- Create `src/features/agentSessions/ImportCodexSessionDialog.tsx` and its local stylesheet. Own link input, preview, automatic missing-folder allocation notice, pending/error state, and completion callback here. Include Escape, focus containment/restoration, and an accessible title. Keep provider payloads out of the UI.
- Adapt `SessionSelector.tsx` with an import callback/action and `AgentSessionScreen.tsx` to compose the dialog. After success, reuse collection reload/selection and the existing conversation view. Keep import state out of `useAgentSessionController.ts`.
- Wire the import client through `src/bootstrap/productApplicationComposition.ts` and the standalone screen's actual composition callers. Do not expose it to every embedded conversation.
- Adapt transcript contracts/projector/view only for imported provenance, source order, and history content the existing representation cannot show truthfully. Preserve ordinary live behavior and existing anchors.

### Application

- Create `agent_sessions/imports.rs` for typed import input, preview, source identity, and prepared history data; create `agent_sessions/ports/import.rs` for the provider history/fork and atomic import-store contracts. These are bounded import contracts, not a generic migration framework.
- Create `agent_sessions/application/import.rs`, composed as a dedicated `AgentSessionImportService`. It owns validation, selected-home continuity, preview, session preparation, provider fork, and persistence. It depends on the provider adapter and store through ports, not SQL or Codex wire JSON.
- Extract a reusable prepare-default-session operation from `application/configuration.rs` and `creation.rs`. Existing default creation and import both consume it. It returns the prepared session with pinned configuration before persistence; first-message validation in `start_direct_user_session` stays intact.
- Extend native profile authority in `native_profiles/session_binding.rs` to supply validated import context and a transaction-level binding insert helper. Reuse its profile ID/filesystem identity rules. Do not simulate a launch to acquire a binding or let the frontend choose an arbitrary CODEX_HOME.
- Create `agent_sessions/transport/import.rs` for preview/import DTO mapping and commands. Run blocking provider work off the UI command thread. Compose the service in `active_app/sessions.rs` and register commands in `active_app.rs`.

### Codex adapter

- Extract supervised RPC setup/cleanup from `app_server/environment.rs` into `app_server/client.rs`. Both capability discovery and the new `app_server/history.rs` use it. Retain `connection.rs`, executable resolution, process supervision, parent-variable stripping, timeouts, and bounded shutdown. Capability-specific reads and skill-root configuration remain in `environment.rs`.
- Create `app_server/history.rs` for thread/read, thread/fork, source decoding, and history normalization. Read preview without resuming. Materialize the exact fork history for persistence, rather than assuming an earlier preview is unchanged.
- Extract native item conversion from `app_server/notifications.rs` into `app_server/items.rs`; live notifications and stored-history decoding share the pure conversion. Keep live lifecycle dispatch and streaming normalization in their current owners. Imported snapshots must not travel through the runtime update sink.
- Retain native model context by resuming the returned fork ID on subsequent prompts. Do not reconstruct context by concatenating transcript text, directly editing Codex storage, or inheriting the desktop tools bridge.

### Persistence

Create `agent_sessions/repository/import.rs` implementing the import-store port on `SqliteAgentSessionRepository`. One ActiveDatabase transaction writes the usable Orchid result:

| Table | Imported content |
| --- | --- |
| `agent_sessions` | New Orchid identity, source title, validated folder, fork runtime binding, pinned Orchid configuration, import-time creation metadata. |
| `agent_session_invocations` | Historical turn records with user input, terminal outcomes, and known historical fields. No pending/running imported invocations. |
| `agent_session_runtime_events` | Ordered normalized historical messages/tool activity plus original provider item payloads. |
| `agent_session_native_profile_bindings` | Validated source home/profile identity, so the first real turn can resume the fork. |
| New `agent_session_imports` | Import request identity, source thread ID, fork ID, source profile identity, import time, and resulting Orchid session ID. |
| New `agent_session_imported_turns` | Invocation-to-source-turn mapping, source ordinal, and available original timestamps. |

Add schema definitions beside the import repository and register them in `storage.rs` for fresh databases and existing active databases. Use existing session/invocation insert helpers; extract an event insert helper if the current append path embeds lifecycle assumptions. Binding SQL stays owned by the native-profile module and participates in the same transaction; do not nest a separate service transaction.

Do not populate launch acceptances or native launch provenance for historical turns. They are written by future real Orchid invocations. Do not invent diagnostic, Workflow delivery, address, or file-authorship records from imported activity.

Reuse existing history loading, summaries, and transcript rendering. Extend their metadata with import origin/source order, rather than creating a second transcript store and reader. Old messages must survive an Orchid restart without fetching the source again.

## History fidelity and sequencing

- Map each native turn to an Orchid invocation. Preserve all user messages, including steering, and assistant/tool item order. The present single `submittedText` field is a display convenience, not authority to discard additional inputs: retain structured user items as ordered events and render them where needed.
- Keep import origin separate from `user`/`application` input provenance. Unknown source provenance must not be presented as known application ownership; extend the historical representation if actual provider fixtures require an unknown value.
- Preserve source turn ordinal explicitly. Do not use random UUID ordering or fabricated source timestamps. Orchid record creation time may be import time; original timestamps remain optional provenance and must be labeled accordingly.
- Preserve available attachments and unsupported items in their raw payloads. Render supported content; use a visible unavailable/unsupported indication where needed. No silent omission and no new media downloader.
- Imported historical outcomes are history, not evidence of an Orchid-owned process. Update observation projection to honor import origin, including when a known source completion timestamp exists.
- Import a settled source boundary. Verify the installed protocol's actual fork behavior; if it cannot select a completed boundary, require the source to finish before import. A status read from another app-server process is not proof that the desktop has no active turn.
- Validate link syntax, home, folder, profile, and decodable history before creating the fork. Revalidate selected profile continuity when committing the import. Source metadata and configuration are data, not instructions to the importer.

## Failure boundary

Codex fork creation and SQLite commit cannot be one transaction. Keep an import request ID stable across dialog retries, with a small durable receipt recording an acknowledged fork before materialization. A completed request returns its existing Orchid session; a retry with a known fork reuses that fork. This is import-specific bookkeeping, not a background job system.

Do not automatically repeat a fork request whose response was lost: report the uncertain outcome. A crash between provider success and receipt persistence remains a possible orphan-fork boundary; do not claim exactly-once behavior or delete source conversations to compensate. Failure while writing Orchid history rolls back the entire usable-session transaction, leaving the receipt available for retry.

## Implementation sequence and validation

1. Verify the installed executable/protocol and capture disposable thread/read and fork fixtures. Current local CLI is 0.144.0; current online documentation may describe newer fields. Confirm history ordering, user input/steering, timestamp availability, attachments, fork context, and incomplete-turn behavior before finalizing DTOs. No real user conversation is forked as a planning probe.
2. Extract the bounded RPC client and pure item converter; keep discovery and live-event regression tests passing. Add history adapter fixtures and parsing tests.
3. Add preparation reuse, import service, native binding helper, and transactional persistence. Test rollback, stable retry identity, home continuity, full reload, source order, and absence of launch/workflow side effects.
4. Add transport/client/modal and standalone composition. Test invalid links, preview, errors, one import while pending, cancel before import, successful selection, and rendered historical messages.
5. Run relevant Rust/frontend tests and builds. In disposable local state, import a known conversation, restart Orchid, verify history/configuration, and send one prompt that depends on source context. Confirm it resumes the fork and the source does not acquire the new turn. Exercise the modal visibly, including keyboard focus and error recovery.

No broad Agent Session controller, navigation, native-profile, or process-supervisor rewrite is included. Remove only the superseded inline RPC lifetime and item-conversion code after their existing consumers use the extractions. Coordinate action placement with the separate repository-navigation plan rather than implementing that plan here.

## Execution record

Implemented the ownership boundaries above, including the missing-folder correction: the service passes an absent folder through the same allocator as normal session creation. The modal has no folder chooser. The standalone import action places the result in Unfiled, while retaining an existing source directory when available.

The default-session preparation extraction stayed in `application/configuration.rs`; it reuses the existing `prepare_session_with_id` without changes to `creation.rs`. `sessionClipboard.ts` also consumes imported content so copying a session retains steering messages in order. `ImportedTurnContent.tsx` and `importedTranscript.ts` own historical display and provenance, with existing transcript anchors retained for excerpts. Import tables ship in active schema version 51.

Validation:

- Production frontend build and Rust compile passed. Existing large-bundle and Rust dead-code warnings remain.
- Agent Sessions and composition frontend suite: 104 tests passed. A subsequent focused transcript/excerpt run passed all 15 tests after tightening summary-view detail handling.
- Rust Agent Sessions: 77 passed; storage and migrations: 41 passed; app-server adapter: 10 passed. The opt-in installed-CLI test runs separately.
- Installed Codex CLI 0.144.0 with a disposable home and local fixture provider: native fork retained history; Orchid's real import adapter materialized the session and native-home binding; reopening the application/database retained the exact history and pinned configuration; retry returned the same session; a normal profiled Orchid follow-up completed on the fork. The outgoing provider request contained the source context marker, and the source retained its original turn count. No user conversation was imported by these tests.
- The isolated fixture uses read-only capabilities: its fresh home has no Windows workspace-write sandbox setup. The test first confirmed that the normal runtime rejects that mismatch before starting a turn; no production permission behavior was changed.
- Browser review of the real standalone screen with fixture clients covered invalid-link recovery, preview, automatic empty-folder notice, successful selection/history rendering, autofocus, modal isolation, Escape, and focus restoration. This was a browser review, not a packaged Tauri/WebView session.

Reproduce the installed-CLI check by building the Rust test binary with `cargo test --profile test-fast --lib import`, then setting `CODEX_APP_SERVER_CONTRACT_PROGRAM` to the installed Codex executable and `ORCHID_IMPORT_TEST_BINARY` to that binary and running `node --test --test-name-pattern="independent history fork" scripts/codex-app-server-contract.node-test.mjs`.

The original conversation remains independent. Imports read only the selected local Codex home; unavailable attachments have visible placeholders with original payloads retained. The initial pass did not perform packaged desktop validation or a hosted-model run. The subsequent native development-app and authenticated continuation checks are recorded in `codex-import-native-validation.md`. The documented orphan-fork boundary after an uncertain provider response remains.
