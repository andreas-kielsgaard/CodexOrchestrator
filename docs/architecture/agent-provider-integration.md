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
- Each provider has one desktop `register` function (`codex::register`, `claude::register`). It adds the provider's parts to `ProviderRegistrations` in `active_app/execution_configuration.rs`:
  - runtime (`AgentRuntime`);
  - configuration (`ProviderConfigurationSource`);
  - launch preparation (`ProviderLaunchPreparation`);
  - continuation (`ProviderContinuationPort`), optional.
- `ExecutionEndpoints` is the router consumers ask. It maps an instance's provider and device to that provider's parts, locally or over SSH.
- Frontend provider features live in `src/features/agentProviders/<provider>/`. Native clients live in `src/infrastructure/agentProviders/<provider>/`.
- The provider's display descriptor is registered in `src/application/agentProviders/descriptors.ts`.
- The provider's route settings component, if it has native options, is registered in `src/features/agentProviders/ProviderRouteSettings.tsx`.
- The provider's guidance for its own runtime failures, if any, is registered in `src/features/agentProviders/RuntimeFailureGuidance.tsx`.
- The provider's setup screen is a Technical Settings section.

The parts are registered separately, in one generic provider map each. A provider-specific exception belongs in a focused provider module such as `options`, `continuation`, `configuration`, or `setups`, not in a shared provider object.

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

Configuration discovery implements `ProviderConfigurationSource`. A registered configuration is identified by `ProviderConfigurationRef { provider, configuration_id }`. The source:

- builds runtime profile snapshots with that reference, and reports provider defaults as `provider_options`;
- reports models and skills for the composer and the model catalogue;
- lists the provider's setups on the device as `ProviderSetup` (folder, executable, sign-in state). Technical Settings lists every provider's setups together, and Capability Profile routes are added from them.

A Capability Profile has at most one route per provider per device, and a model ID belongs to one route per device. The route for a prompt follows from its device and model.

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

Three fields are written only by the provider itself:

- `environment`, filled by the provider's own launch preparation, for example its native folder.
- `executable`, the setup's CLI, also filled by launch preparation.
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

- A provider request opens as a typed `RuntimeRequest`: an approval with choices, questions, or an external action. A question may offer options, several selections, a typed answer and secret input; the UI renders whichever the provider offers.
- Interaction responses are `RuntimeInteractionResponse`. The provider maps opaque product choice IDs and answer maps to its native wire format.
- Continuation is optional and registered through `ProviderContinuationPort`. Its payload is provider-tagged and opaque to device transport.
- The remote host (`crates/orchid-engine/src/host.rs`) is provider-neutral. Each provider implements `HostProvider` (`host/providers.rs`); only Codex is registered on hosts today. Remote configuration-bound commands carry provider identity. During SSH connection setup, the desktop checks `HOST_PROTOCOL_VERSION` (currently 2) before issuing provider commands.

## Sessions, instances and conversations

- An **Agent Session** is an Orchid conversation: its history, transcript and settings, stored in Orchid's database on the main device. It is not tied to one device or provider.
- An **instance** is the provider, device, configuration and harness that the session's invocations run on. While an invocation is active, everything for it goes to that instance: steering, request answers and cancellation. It cannot change device, provider or harness partway through.
- A different target selected between invocations gives the **same session** a new instance. The target change is applied when the next prompt is prepared; no new Agent Session is created. A device move copies the worktree state and transfers the provider's native conversation to the new device through `ProviderContinuationPort`. A provider without that port restarts its conversation from the session log instead.
- A session has at most one native conversation per provider. Native conversation IDs only save work: the session log is the record.
  - The current provider's conversation is on the session's runtime binding. The other providers' conversations are parked in `agent_session_parked_conversations`.
  - Changing provider parks the current conversation. A provider new to the session starts a conversation that receives the session's history, and the session's first initial prompt prefix, in one `orchid_provider_handoff` prefix. A returning provider resumes its parked conversation and receives only the turns it missed.
  - The first initial prompt prefix is kept in `agent_session_initial_prompt_prefixes` for this.
- A native conversation record is written by one Orchid session only. Importing a Codex app conversation uses Codex's thread fork to create a new Orchid-owned native conversation, so Orchid never writes to the Codex app's own record. Forking is a Codex capability; the import is the only feature that uses it today.

## Current Codex implementation

Codex is registered at the composition root. Its app-server adapter, options, native profile administration, configuration discovery, history, and continuation code are confined to the Codex provider directories.

- Codex personality is encoded through `CodexNativeOptions`.
- Launch intent is translated in `providers/codex/app_server/configuration.rs`, which both the app-server runtime and the test-only CLI runtime use. Managed servers are sent as thread configuration, with bearers in the process environment. The other intents are process-level values.
- Stored Codex-era references and personality settings were converted once by `runtime/providers/codex/legacy_migration.rs` (schema v59).
- The shared UI never constructs a Codex approval response.

## Current Claude implementation

Claude Code is driven through its CLI in stream-json mode, one `claude -p` process per invocation. The engine module (`crates/orchid-engine/src/providers/claude/`) holds the runtime, and the desktop module (`src-tauri/src/runtime/providers/claude/`) holds setups, configuration and launch preparation.

- **Setups** are a Claude configuration folder and the CLI that uses it, stored in `claude_setups`. The default folder (`CLAUDE_CONFIG_DIR`, otherwise `~/.claude`) registers itself once it exists. Other folders are passed to Claude as `CLAUDE_CONFIG_DIR`; the default folder is left unnamed, because naming it moves Claude's global settings file into it. Readiness comes from `claude auth status`, and signing in stays with `claude auth login`.
- **Configuration**: models and effort levels come from Claude's `initialize` report. Skills come from `<folder>/skills` and `<project>/.claude/skills`. Claude offers full access only.
- **Launch intent** (`providers/claude/launch.rs`):
  - full access is `--permission-mode bypassPermissions`, and unattended approval is `--permission-prompts none`;
  - managed MCP servers go in `--mcp-config`, with each bearer read from an environment variable in the header, and their tools are pre-approved with `--allowedTools`. Claude has no per-server tool filter, so a managed server's other tools remain visible; the server authorizes its own tools;
  - native MCP suppression is `--strict-mcp-config`, and ignoring user rules is `--setting-sources ""`;
  - trusted workspace and sandbox network access need nothing in print mode.
- **Conversation**: a new conversation starts with an Orchid-chosen `--session-id`, and a later one resumes with `--resume`. Claude has no continuation port, so a conversation that moves device restarts from the session log.
- **Steering** writes another user message. `--replay-user-messages` echoes each message when Claude takes it in, so the invocation finishes at the result after the last one.
- **Requests**: permission prompts (`can_use_tool`) open approvals with Allow once, Always allow (when Claude suggests a rule) and Decline. `AskUserQuestion` opens questions, and the answers return in the tool input under each question's text.
- **Cancel** sends `interrupt`, then stops the process through the supervisor after a short grace period.
- Invoked skills are not delivered natively; the prompt names them, and `read_skill` serves Orchid's pinned skills.
- Adapter tests run against recordings from Claude Code 2.1.282 in `providers/claude/fixtures/`. `live_claude_code` drives the installed, signed-in CLI when `ORCHID_CLAUDE_LIVE=true` (`cargo test -p orchid-engine live_claude_code -- --ignored`).

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
