# 0006: Sprint 6 Plan Builder harness and initiation confirmation

Status: accepted basis for WU-S6-01 implementation. Supersedes the one-tool catalog in 0004 and
the best-estimate MCP approval-mode wording in 0003.

## Decision

- Keep Agent Session provider- and role-neutral. Specialized orchestration services select one
  versioned Conversation Harness profile and assemble its runtime options, context, and MCP
  exposure.
- The Epic Plan Builder runs with Codex `read-only`, noninteractive approval escalation disabled,
  and one required child-scoped MCP server. That server exposes only
  `submit_epic_plan_proposal` and `request_epic_initiation`; both derive draft, role, session,
  invocation, authorization, and replay scope from the registered managed session. Discussion has
  no semantic effect.
- The Bootstrap Generator profile is `workspace-write` only when its future specialized service
  uses an application-prepared working root. The Epic Runner profile is `read-only`. This unit does
  not create either session or claim their later semantic tools exist. Model and reasoning remain
  inherited because this unit has no accepted role-specific override.
- One application-owned initiation coordinator serves button and agent requests. It publishes a
  typed request, accepts only an explicit user confirmation or rejection, and invokes the existing
  semantic initiation command only after confirmation. Its events distinguish `requested`,
  `user_confirmed` or `user_rejected`, `applied`, `persisted`, and `projected`. Agent rejection is
  returned to the waiting MCP call; button rejection has no agent callback.
- Request registration, replay identity, and initial publication are atomic; failed publication
  leaves no reusable invisible request. Every resolution records one terminal result and wakes all
  waiters. If initiation persists before a later notification or projection failure, the terminal
  result retains that durable identity for reconciliation and replay never reapplies the effect.
- The existing direct initiation transport is not an authorization boundary. WU-S6-03 will connect
  the button and popup to the coordinator request/resolution commands; no production path may
  treat a popup opening or tool receipt as confirmation.

## Runtime evidence and residual boundaries

Installed `codex-cli 0.144.0` exposes `--sandbox read-only|workspace-write|danger-full-access`,
`--model`, `--cd`, strict `-c` configuration, and Streamable HTTP MCP bearer configuration. The
current Codex manual documents required MCP startup, tool allow-lists, per-server approval mode,
and OS-enforced local sandboxing. WU-S6-01 wires only those options and fails closed when requested
sandbox support is unknown or unsupported.

`read-only` constrains model-generated filesystem commands; it does not authorize product effects.
Product effects remain limited by the child MCP allow-list and server-side semantic authorization.
The Bootstrap Generator's prepared-root containment is not proven until its specialized service
selects that root. No harness profile, prompt, Agent Session terminal state, or `initiated` draft
status proves materials complete, an Epic Runner launched, or a Sprint started.
