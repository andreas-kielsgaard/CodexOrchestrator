# 0003: managed Codex invocation and proof

Status: accepted basis for Sprint 5 implementation.

## Managed-invocation seam

- Preserve Agent Session identity and `AgentRuntime` as provider-neutral. Do not add an Epic ID,
  draft ID, MCP endpoint, token, tools, or role to either contract.
- Add an orchestration-owned managed-invocation assembly at native composition. It resolves a
  capability profile for an `EpicPlanningDraftId`, builds child-scoped Codex configuration and
  environment, starts/stops the MCP server, and delegates the resulting generic process launch to
  the existing Codex runtime/process supervisor.
- For the installed CLI, inject only supported `-c key=value` configuration:
  `mcp_servers.<generated-name>.url`, `.bearer_token_env_var`, `.enabled_tools`,
  `.default_tools_approval_mode="prompt"`, `.startup_timeout_sec=10`, and
  `.tool_timeout_sec=60`. The generated bearer variable receives its value only in the child
  environment. Do not alter global or project `config.toml`. The approval-mode value is a G1
  best-estimate configuration input; its installed-client noninteractive behavior is unproven until
  a later authorized live gate.
- The reversal boundary is this assembly and its `CodexInvocationConfigurator` port. A future SDK,
  app-server, stdio, or non-Codex provider replaces that adapter without changing Agent Session
  identity, orchestration persistence, tool handlers, or TypeScript presentation contracts.

## Provenance and evidence

Every authorized, validated, applied, persisted command/tool effect records immutable provenance
available at effect time: source kind, recorded time, causal IDs, actor/session reference when known,
managed-invocation/profile/config-version identity, Codex CLI version, prompt template version,
submitted prompt digest and retention classification, tool name, validated argument digest,
idempotency key, and recorded event/result IDs. Prompt text or tool arguments that contain secrets
are redacted before durable logging; a digest is not a claim that content was retained. Later
invocation terminal evidence is a separate observed record with causal references when available;
its absence at effect time neither blocks a valid tool effect nor implies a terminal outcome. Tool
handlers request application commands; they never append events directly.

## Codex capability observation

On 2026-07-15, local `codex --version` returned `codex-cli 0.144.0`. Help established `exec`,
`exec resume`, `-c`, model and sandbox options, and `mcp add --url` with
`--bearer-token-env-var`; `codex mcp list` established only currently configured servers. No model
prompt was run.

| Status                                      | Evidence and required behavior                                                                                                                                                                                                                                                          |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Supported CLI surface                       | `exec`/`exec resume` expose `--json`; `exec` exposes model/sandbox; `-c` accepts TOML configuration; Streamable HTTP MCP accepts a bearer-token environment variable.                                                                                                                   |
| Deterministically assembled product surface | `RuntimeLaunchExtension` is opt-in on one `RuntimeInvocationRequest`; `ManagedPlanBuilderService` owns the server/injection lifecycle and `send_managed_plan_builder_message` is the role-specific command. Argument, environment, lifecycle, and direct MCP tests cover that assembly. |
| Unknown                                     | Installed-client noninteractive behavior, model tool selection, a paid/provider run, and real Codex-client/server interaction remain unobserved. Do not promote them from help output.                                                                                                  |

The current runtime's help/version probe is supported capability evidence, fresh for 30 minutes when
observed and one minute when unavailable; explicit refresh/invalidation is already available.
Configuration edits or executable changes must invalidate/refresh this cache. A persistent MCP
configuration requires client restart/reload under Codex documentation; the selected `-c` values are
child-scoped and apply only to that invocation.

Deterministic protocol success proves server/handler conformance and recorded completion only. It
does not prove Codex model tool selection, prompt delivery, paid/provider behavior, or a live user
flow. Those require separately authorized live proof; G3 includes the required user UI/UX review.

Sources: [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp.md) and
[Codex configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference).

## G1-D4 finding

Implementation established that an opt-in `RuntimeLaunchExtension` plus an
orchestration-owned managed command was necessary. A global runtime wrapper was rejected: it
would inject role-specific MCP configuration into ordinary Agent Session sends and blur their
provider-neutral boundary. With Codex CLI `0.144.0`, the child arguments and bearer-only child
environment are deterministically assembled and tested. That is not evidence that the installed
client accepts the configuration in a noninteractive run or that a model uses either tool; the
[real-flow gate](../sprint-5-extension-and-gates.md#real-flow-gate) remains required.
