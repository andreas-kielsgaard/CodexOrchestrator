# Agent Session quick features

Typing `/` at the start of an otherwise empty message opens quick features. `/model` or `/reasoning` followed by Enter opens its choices and replaces the command with `/`. Type a choice and press Enter again. Choices come from the selected runtime; names such as `light` are offered only when the provider exposes them.

`/skills` opens the skill list. Skills are also searchable directly from the root menu. Selecting one inserts its provider-specific invocation text and leaves the draft ready for a request. It does not send a message.

Arrow keys navigate, Enter or Tab selects, Backspace at `/` returns to the parent, and Escape returns or closes. Shift+Enter remains a newline; IME confirmation does not send. Unmatched commands, discovery failures, and loading states do not submit. Escape closes the picker when the user intends to send literal slash-prefixed text. Paths containing another slash and ordinary prose remain message text.

Model and reasoning choices use the existing message-local selection state, shared with the configuration controls, and reset after acceptance. They do not change pinned Session defaults or an active turn. Changing models replaces an incompatible effort with the model's advertised default. Skills remain available for steering.

## Ownership

- `src/features/agentSessions/composerQuickActions.ts` defines local commands and choice actions. A command opens child choices or updates the draft/selection. Add future quick tools here with their required application callbacks.
- `useComposerQuickMenu.ts` owns navigation, filtering, keyboard handling, asynchronous discovery, and stale-context rejection. `ComposerQuickMenu.tsx` renders the accessible choice list above the shared composer.
- `src/application/agentSessions/quickFeatures.ts` is the serializable discovery contract. No provider request method or filesystem parser belongs in the composer.
- `src-tauri/src/agent_sessions/application/quick_features.rs` resolves the Session's working directory and inherited defaults. Existing Sessions must retain their attached runtime identity; a first-message preview uses the default Capability Profile without creating a Session.
- `src-tauri/src/execution_configuration/native_codex/quick_features.rs` projects Codex models, model-specific reasoning efforts, and enabled skills. It uses the existing environment reader and product skill roots. Other providers implement `SelectedRuntimeProfileSource::quick_features_at`; unsupported discovery returns an explicit error.

Discovery refreshes when the picker opens and supports retry. Send-time validation remains authoritative. The current Codex adapter inserts native `$name` mentions. Duplicate names from distinct paths are omitted with an explanation because a name-only invocation cannot identify the intended source.

## Provider differences

Skills, plugins, and MCP connections are separate concepts. A plugin can bundle skills, hooks, agents, servers, and provider-specific features. Sharing an MCP server configuration does not establish portability for those other components. [Claude Code plugin documentation](https://code.claude.com/docs/en/plugins).

Skill discovery roots, namespacing, metadata, explicit invocation syntax, and reload behavior vary. Codex supports textual `$name` invocation and recommends a typed skill input with a path when exact attachment is needed; Claude plugin skills use names such as `/plugin-name:skill-name`. Each adapter must own that translation. This implementation validates Codex textual invocation and does not claim cross-provider skill execution. [Codex app-server documentation](https://learn.chatgpt.com/docs/app-server), [Claude Code skills](https://code.claude.com/docs/en/skills).

An installed plugin or configured MCP server may still need authentication, host support, or permissions before its tools are callable. Future plugin controls should distinguish installed, enabled, connected, and callable states. MCP prompts are user-selected templates, while tool execution is a different operation; a future `/mcp` feature should specify which operation it exposes. [OpenAI plugin documentation](https://learn.chatgpt.com/docs/plugins), [MCP prompts specification](https://modelcontextprotocol.io/specification/2025-06-18/server/prompts).

## Validation

Component and screen tests cover the two-step flow, mouse and keyboard selection, IME/newlines, first-message choices, model-specific efforts, skills, discovery retry, stale responses, active turns, and literal slash input. Rust tests cover provider projection, duplicate/disabled skills, Session context, and runtime identity mismatch.

The installed Codex 0.144.0 contract test uses a local fixture provider and a disposable unauthenticated home. It verifies the catalog fields and that a textual skill mention loads the skill instructions into provider input. Browser review exercises the real Session screen against explicit fixture capabilities and checks the send receipt. These checks do not establish an authenticated model run, a relaunched native application, or another provider's plugin support.
