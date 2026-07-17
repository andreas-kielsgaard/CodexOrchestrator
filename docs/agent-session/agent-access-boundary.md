# Agent access and capability boundary

Status: Sprint 3 capability discovery and Codex production composition implemented.

Agent Session application behavior depends on the provider-neutral `AgentRuntime` port. Session
creation does not accept or select an adapter. The active application composition chooses Codex CLI
and constructs it under `runtime/codex`; Codex arguments, protocol events, executable discovery,
and processes do not cross into Agent Session application code.

The durable runtime binding contains only the external continuation context and observed runtime
version. Start versus resume depends on whether that external context exists. Local Agent Session
and invocation IDs are never continuation targets.

## Ownership and extension

The active provider-access structure is intentionally small:

- `agent_sessions/ports.rs` owns the provider-neutral runtime contract consumed by Agent Session;
- `runtime/capabilities.rs` owns reusable freshness and cache policy for provider-access evidence;
- `runtime/codex/` owns the concrete Codex CLI adapter and its discovery, protocol, and launch rules;
- `runtime/processes/` owns supervised direct-child process lifecycle; and
- `active_app.rs` selects and composes the supported adapter.

When a product need requires another access mode, first define the needed semantic capability. Add
it to the neutral contract only when current application behavior needs it, keep raw provider
details inside the concrete adapter, add deterministic contract and adapter tests, and then update
composition. Multiple simultaneously supported adapters will require an explicit selection,
binding, and routing decision; Sprint 3 does not pre-build that machinery.

## Capability discovery protocol

Capability discovery follows one focused protocol:

1. The concrete adapter probes its own surface and translates raw findings into semantic start and
   resume capabilities.
2. Every capability is `supported`, `unsupported`, or `unknown`. Discovery failure produces
   unknown capabilities with an explicit `unavailable` state; it never implies unsupported.
3. A snapshot records discovery provenance, runtime version when known, observation time, and the
   adapter-selected validity deadline.
4. Runtime infrastructure reuses a fresh cached snapshot. Callers may request refresh, and
   composition may invalidate the cache when executable/configuration identity changes.
5. Application behavior consumes semantic capabilities and effective options. It does not parse
   CLI help text, flags, versions, or provider protocol values.

The current Codex adapter owns an application-lifetime in-memory cache. It probes lazily on the
first capability-dependent preflight, never at startup. Observed version/help evidence is fresh for
30 minutes; a fully unavailable probe is cached for one minute. Explicit runtime refresh and
invalidation bypass those lifetimes, and concurrent first use is serialized into one discovery.
Persistence is intentionally deferred until a product need justifies cross-process freshness.

Structured JSONL is a required Codex protocol flag. Confirmed lack of JSON support fails preflight;
unknown evidence retains the existing honest behavior of attempting the required flag and allowing
the CLI launch result to decide. Requested model or sandbox options are applied only when support is
observed, omitted from effective options when unknown or unavailable, and rejected before launch
when confirmed unsupported. Start and resume use separate semantic capability surfaces. Preflight
is the only capability-dependent invocation step: after its effective options are persisted, launch
uses them directly and never rediscovers or reinterprets capability evidence for that invocation.

To extend the current adapter after a Codex update, add raw discovery in `runtime/codex`, translate
new evidence into the existing semantic snapshot (adding a semantic field only when product
behavior needs it), update provenance/freshness policy, and add deterministic probe/cache/argument
tests. This is the expansion pattern for later agent-access adapters without pre-building them now.

## Deferred Epic insight

Later service and MCP integration work should apply the same evidence rules: integration-owned
discovery, semantic snapshots with provenance and freshness, cached reuse, explicit refresh and
invalidation, and truthful unknown/unavailable results. Sprint 3 does not implement MCP discovery
or define capabilities for services other than Codex CLI. Carry this insight to the Epic Runner.

Agent-to-product tools are a complementary boundary, not an extension of `AgentRuntime`. Later MCP
work should own a semantic tool catalog, exposure policy, application authorization, and capability
profiles covering role, tools, skills, authority, and context. Tool visibility limits agent context;
server-side authorization remains the actual control boundary. MCP handlers should invoke product
application commands whose accepted effects become Orchestration Events rather than exposing event
storage or provider-runtime internals directly.
