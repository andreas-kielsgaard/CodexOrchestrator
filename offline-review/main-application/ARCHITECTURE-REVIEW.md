# Architecture review

## Responsibility flow

```text
Product surface
  → product-owned Conversation Harness
    → provider-neutral Agent Session application
      → AgentRuntime port
        → Codex CLI adapter

Agent product action
  → invocation-scoped MCP tool
    → server-derived role/session/draft authority
      → application command
        → Orchestration Event / durable fact
          → canonical read-model composition
            → UI projection
```

This direction deliberately prevents Agent Session, Codex CLI, orchestration roles, and UI
projection from collapsing into one abstraction.

## Strong choices worth preserving

### Agent Session neutrality

Session creation does not accept `codex_cli` or an orchestration role. The active composition chooses
the provider adapter. Durable runtime binding stores only continuation context and observed runtime
version.

### Product-owned role harnesses

Plan Builder, Bootstrap Generator, and Epic Runner behavior is configured outside Agent Session.
Harnesses currently define initial role context, skills, MCP exposure, sandbox, and related runtime
policy.

### Narrow semantic tools

The Plan Builder does not write product state through prose. It receives two tools. The Bootstrap
Generator receives one. Tool handlers derive authority from the registered invocation rather than
accepting Epic/session identifiers from the model.

### Durable facts before projection

The UI consumes strict, decoded read contracts. A transcript, button click, popup opening, or
running process does not silently become a product fact.

### Attempt-bound Bootstrap acceptance

Semantic completion and lifecycle success must belong to the same attempt. Retry attempts cannot
combine evidence, and one accepted inventory gates one Runner launch.

## Areas to challenge

### Large implementation modules

The accepted behavior is better separated conceptually than the original codebase, but several new
modules are still large:

- `src-tauri/src/orchestration/bootstrap_transition.rs`
- `src-tauri/src/orchestration/application.rs`
- `src-tauri/src/orchestration/repository.rs`
- `src/application/orchestrations/productReadModelComposer.ts`

Do not assume acceptance of behavior means their internal structure is final. Before adding many
more orchestration roles, review whether transition stages, repositories, identity derivation,
attempt policy, and composition can become smaller responsibility-focused modules.

### Legacy Rust root

`src-tauri/src/lib.rs` still retains the quarantined legacy task/run implementation. New
orchestration work must not call or extend it. Required legacy capabilities should be rewritten or
extracted into focused active modules.

### Process ownership

The active Agent Session supervisor owns direct children only. It has a polite shutdown window, then
terminates and reaps the child. It does not yet provide Windows process-tree ownership through a Job
Object.

### Test timing and formatting debt

Some frontend tests remain timing-sensitive under aggregate load despite passing in isolation or
serially. Repository-wide formatting also retains unrelated historical debt. These should remain
visible without making every Work Unit pay the entire cleanup cost.

### Provider expansion

Codex CLI is the only concrete adapter. The boundary supports future providers, but no alternate
provider should be pre-implemented until an actual capability requires it. Agent-to-product MCP
tools should not be folded into `AgentRuntime`.

## Questions for structural review

1. Does each module have one reason to change, or are transition stages accumulating around shared
   storage convenience?
2. Can a new orchestration role be added through a harness, specialized application service, and
   narrow tools without editing Agent Session internals?
3. Are durable facts stored once and projected, or duplicated between native query, transition
   query, fixtures, and view state?
4. Are retry, idempotency, and identity rules colocated with the transition that owns them?
5. Can recorded adapters be replaced by production connectors without changing the presentation
   component tree?
6. Are unsupported states explicit, or does absence silently look like success?

## Candidate next architectural movement

Finish the serial product-owned flow from Epic Runner to one Sprint before introducing broad
parallel scheduling. That movement should add application-owned actions for starting a Sprint and
receiving its result, with durable routing rather than prompt-based handoff.

Before or during that work, avoid extending the large transition modules by convenience. Extract or
create focused ownership boundaries when a new responsibility would otherwise enlarge them.
