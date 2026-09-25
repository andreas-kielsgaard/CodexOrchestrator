# Agent provider integration boundary

Orchid owns:

- session and invocation lifecycle;
- capability-profile policy;
- workspace and device routing;
- managed skills and tools;
- persistence and presentation.

A provider owns:

- native process and protocol behavior;
- native configuration and native options;
- event translation and interaction encoding;
- optional continuation and history behavior.

A provider receives product intent and translates it. It reports an intent it cannot honor instead of substituting another behavior.

## Registration points

- Shared implementations live in `crates/orchid-engine/src/providers/<provider>/`.
- Desktop implementations live in `src-tauri/src/runtime/providers/<provider>/`.
- Desktop composition registers runtime, configuration, and continuation implementations separately in `active_app/execution_configuration.rs`.
- Frontend provider features live in `src/features/agentProviders/<provider>/`. Native clients live in `src/infrastructure/agentProviders/<provider>/`.
- The provider's display descriptor is registered in `src/application/agentProviders/descriptors.ts`.
- The provider's route settings component, if it has native options, is registered in `src/features/agentProviders/ProviderRouteSettings.tsx`.

The runtime, configuration, and continuation registries are intentionally separate. A provider-specific exception belongs in a focused provider module such as `options`, `continuation`, `configuration`, or `profiles`, not in a shared provider object.

## Required behavior

### Runtime

A provider runtime implements the existing `AgentRuntime` operations:

- prepare and deliver;
- start and resume;
- interaction;
- steering;
- cancellation;
- shutdown.

Unknown providers fail explicitly; there is no Codex fallback.

### Configuration

Configuration discovery implements `ProviderConfigurationSource`. A registered configuration is identified by `ProviderConfigurationRef { provider, configuration_id }`. The source builds runtime profile snapshots with that reference, and reports provider defaults as `provider_options`.

### Launch intent

`RuntimeLaunchExtension` carries intent only. The provider translates:

- **Managed MCP servers**: name, URL, optional bearer, tool list, required flag. Managed tools never prompt for approval.
- **Approval**: inherit, or unattended.
- **Trusted workspace**: the application authorized this isolated working directory.
- **Sandbox network access**: currently requested only by the Work Unit Implementer reporting continuation.
- **Native MCP suppression** (`native_mcp_enabled: Some(false)`).
- **Reasoning mode.**
- **Ignoring user rules.**
- **Initial prompt prefix.**
- **Pinned skills and invoked skills.**

Two fields are written only by the provider itself:

- `environment`, filled by the provider's own launch preparation.
- `provider_options`, the provider-tagged native settings envelope. Shared code routes it without decoding it.

### Skills

Skills are model independent:

- Orchid resolves its own `$name` mention syntax into `invoked_skill_ids`.
- `read_skill` serves every pinned skill.
- A provider may deliver invoked skills in its native form. Codex attaches the skills its own catalogue discovered.
- Product skill roots are offered with every provider; a provider contributes only its natively discovered skills.

### Events

The provider emits normalized events. Every tool item carries `NormalizedToolActivity`, with a neutral `kind`, a phase, and a stable item ID. The transcript pairs start and completion rows from these fields.

Orchid's own control vocabulary is emitted through `RuntimeControlRecord`:

- turn active;
- request opened, unsupported, or answered;
- steering;
- working directory resolved;
- process exit.

Raw payloads are diagnostic evidence only. A Rust source guard (`provider_boundary_tests.rs`) and an ESLint rule keep product code from deciding on them.

### Interactions, continuation and remote hosts

- Interaction responses are `RuntimeInteractionResponse`. The provider maps opaque product choice IDs and answer maps to its native wire format.
- Continuation is optional and registered through `ProviderContinuationPort`. Its payload is provider-tagged and opaque to device transport. Cross-provider continuation is rejected.
- Remote configuration-bound commands carry provider identity. During SSH connection setup, the desktop checks `HOST_PROTOCOL_VERSION` (currently 2) before issuing provider commands.

## Sessions, instances and conversations

- An **Agent Session** is an Orchid conversation: its history, transcript and settings, stored in Orchid's database on the main device. It is not tied to one device or provider.
- An **instance** is the provider, device, configuration and harness that the session's invocations run on. While an invocation is active, everything for it goes to that instance: steering, request answers and cancellation. It cannot change device, provider or harness partway through.
- A different target selected between invocations gives the **same session** a new instance. The target change is applied when the next prompt is prepared; no new Agent Session is created. A device move copies the worktree state and transfers the provider's native conversation to the new device through `ProviderContinuationPort`. Continuation across providers is rejected. What a provider change does to the conversation is decided with the second provider.
- A native conversation record is written by one Orchid session only. Importing a Codex app conversation uses Codex's thread fork to create a new Orchid-owned native conversation, so Orchid never writes to the Codex app's own record. Forking is a Codex capability; the import is the only feature that uses it today.

## Current Codex implementation

Codex is registered at the composition root. Its app-server adapter, options, native profile administration, configuration discovery, history, and continuation code are confined to the Codex provider directories.

- Codex personality is encoded through `CodexNativeOptions`.
- Launch intent is translated in `providers/codex/app_server/configuration.rs`, which both the app-server runtime and the test-only CLI runtime use. Managed servers are sent as thread configuration, with bearers in the process environment. The other intents are process-level values.
- Stored Codex-era references and personality settings were converted once by `runtime/providers/codex/legacy_migration.rs` (schema v59).
- The shared UI never constructs a Codex approval response.

## Validation

Provider work should add:

- adapter tests;
- registry dispatch tests with a minimal test provider;
- shared contract serialization tests;
- frontend interaction tests;
- engine tests;
- Rust application tests;
- a frontend production build.

A new provider must not require shared session code to parse its native events or response payloads.
