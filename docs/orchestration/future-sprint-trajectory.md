# Future Sprint trajectory

Status: current directional roadmap. Sprints 1 through 4 are accepted and closed. Every later
Sprint requires its own accepted handoff and authorization.

The active hierarchy is Orchestration capability -> Epic -> Sprint -> Work Unit. See
`terminology.md`. Sprint 2 does not implement any roadmap item below.

## Completed foundation

- Sprint 1: product data and controller integration.
- Sprint 2: Epic/Sprint terminology clean break.
- Sprint 3: provider-neutral Agent Session runtime boundary with Codex CLI as the concrete adapter.
- Sprint 4: conversation-primary Epic Plan Builder foundation with a neutral proposed-plan source.

## Remaining Epic projection

1. Durable orchestration state and MCP integration.
2. Material generation and Epic initiation.
3. Serial Sprint/Work Unit execution.
4. Multi-unit adaptive execution.
5. Concurrency, automatic continuation, and recovery.
6. Scaffolding retirement and complete validation.

## Agent integration carry-forward

Product-to-agent access and agent-to-product tools are separate product areas. Sprint 3 established
the former under the Agent Session runtime boundary; later MCP work must not fold tool exposure or
product commands into `AgentRuntime`.

- Epic Plan Builder may describe required roles, tools, skills, authority, and context without
  selecting transport implementations.
- Durable state and MCP integration owns the semantic tool catalog, exposure policy, server-side
  authorization, application command handling, and the tool-extension procedure. Accepted commands
  produce Orchestration Events; agents do not append events directly.
- Material generation and initiation turn accepted requirements into inspectable capability
  profiles for created Agent Sessions.
- Serial execution provides the first controlled proof that a responsibility receives the intended
  tools, skills, authority, and context. Later adaptive and concurrent execution must preserve that
  isolation across revisions, roles, and recovery.
- Scaffolding retirement removes obsolete prompt-driven or unreachable provider/tool paths only
  after their active replacements are proven.

Provider and service discovery should stay integration-owned and expose semantic capability
evidence with provenance, freshness, truthful unknown/unavailable states, cached reuse, and explicit
refresh or invalidation. Tool visibility controls context size; application authorization remains
the safety boundary.

## Retained constraints

- Agent Session remains independently usable and provider-neutral at its application boundary.
- Recorded evaluation and product use share components but use different adapters.
- Product controllers expose only authorized Agent Control commands; verification hooks remain
  test-only.
- Agent Control -> Agent Session -> application MCP handling -> Orchestration Event -> UI read model
  remains the intended authority path. Agent prose is not authoritative state.
- Requested, eligible, observed, integrated, reviewed, and responsibility-accepted facts remain
  distinct.
- Epic-level continuation targets the next Sprint Planner. Sprint-level continuation targets the
  next ready Work Unit planner.
- New work does not import legacy task/run contracts or add behavior to the quarantined Rust root.
- Presentation contracts do not become persistence or transition contracts by convenience.

## Still deferred after Sprint 4

Sprint 4 supplies no orchestration persistence, MCP/event persistence, semantic tool exposure,
authorization, transition execution, prompt delivery, automatic continuation, material generation,
initiation, native artifact effects, or live provider proof. Its proposed-plan source is a neutral
presentation boundary, not durable Epic/Sprint/Work Unit identity or agent-prose authority. It does
not implement alternate providers, descendant-process ownership, file upload, or a later Sprint.
