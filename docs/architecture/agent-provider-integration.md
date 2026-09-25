# Agent provider integration boundary

Orchid owns session and invocation lifecycle, capability-profile policy, workspace/device routing, managed skills and tools, persistence, and presentation. A provider owns native process/protocol behavior, native configuration, native options, event translation, interaction encoding, and optional continuation/history behavior.

## Registration points

- Shared implementations live in `crates/orchid-engine/src/providers/<provider>/`.
- Desktop implementations live in `src-tauri/src/runtime/providers/<provider>/`.
- Desktop composition registers runtime, configuration, and continuation implementations separately in `active_app/execution_configuration.rs`.
- Frontend provider features live in `src/features/agentProviders/<provider>/`; native clients live in `src/infrastructure/agentProviders/<provider>/`.

The runtime, configuration, and continuation registries are intentionally separate. A provider-specific exception belongs in a focused provider module such as `options`, `continuation`, `configuration`, or `profiles`, not in a shared provider object.

## Required behavior

A provider runtime implements the existing `AgentRuntime` prepare/deliver, start/resume, interaction, steering, cancellation, and shutdown operations. Configuration discovery implements `SelectedRuntimeProfileSource`. Unknown providers fail explicitly; there is no Codex fallback.

Provider-native settings use `ProviderNativeOptions`. Shared code persists and routes the provider-tagged envelope without decoding it. Interaction responses are `RuntimeInteractionResponse`; the provider maps opaque product choice IDs and answer maps to its native wire format.

Continuation is optional and registered through `ProviderContinuationPort`. Its payload is provider-tagged and opaque to device transport. Cross-provider continuation is rejected.

Remote configuration-bound commands carry provider identity. The desktop checks `HOST_PROTOCOL_VERSION` during SSH connection setup before issuing provider commands.

## Session identity

An already-bound ordinary session is not retargeted when provider, device, configuration, capability-profile revision, or workspace selection changes. Prompt acceptance creates a destination session and leaves the source session and history intact. A provider may transfer native continuation only through its continuation port.

## Current Codex implementation

Codex is registered at the composition root. Its app-server adapter, options, native profile administration, configuration discovery, history, and continuation code are confined to the Codex provider directories. Codex personality is encoded through `CodexNativeOptions`. The shared UI never constructs a Codex approval response.

## Validation

Provider work should add adapter tests, registry dispatch tests with a minimal test provider, shared contract serialization tests, frontend interaction tests, engine tests, Rust application tests, and a frontend production build. A new provider must not require shared session code to parse its native events or response payloads.
