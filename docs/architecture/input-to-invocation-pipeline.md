# Input-to-invocation pipeline

This document describes the implemented ownership boundaries. An Orchid Session is the canonical
conversation. A provider session is a route-scoped cache that may cover only part of that
conversation.

## Flow

1. A composer, Session Event, Workflow, OTP-mediated feature, or application feature creates
   semantic invocation content and destination intent.
2. Agent Sessions accepts the invocation durably and delegates delivery through
   `InvocationDeliveryService`.
3. Delivery planning resolves the selected device and the single Capability Profile route that
   exposes the selected model. Provider identity is a consequence of this resolution; it is not a
   composer option.
4. The continuity planner selects only an exact provider-session binding: provider configuration,
   device, and endpoint must all match. It computes missed context from Orchid history.
5. The Harness Engine prepares the authorized per-invocation MCP and skill exposure against the
   resolved policy and route.
6. The provider registration prepares its native launch data. The provider adapter renders
   `InvocationContent` into its native request.
7. Agent Sessions prepares the runtime invocation, commits its immutable execution snapshot and
   binding, then releases the prompt once.
8. Runtime events update the canonical Session record. Steering and interaction responses resolve
   the runtime from the invocation, never from a newly selected model or current Session route.

## Ownership

### Callers

Callers own why a prompt exists and the provenance of each product reference. They provide one
primary query plus ordered context parts. Workflow and Session Events retain logical addressing,
fan-out, deterministic delivery IDs, create-on-missing policy, and durable source references. OTP
packages retain curated `node`, `sessions`, and `emit` handles; they do not receive provider
runtimes or repositories.

### Agent Sessions delivery

`src-tauri/src/agent_sessions/application/delivery/` owns the application entry point, semantic
delivery contract, exact-route continuity decision, and canonical context assembly. Prepared
delivery additionally owns recoverable acceptance, route/workspace planning, Harness preparation,
provider preparation, atomic commit, one-time release, cancellation, and restart classification.

The immutable invocation execution snapshot records the exact target, working directory, Session
policy, resolved selections, exposed MCP names/tools, invoked skill IDs, and Harness version.
Secrets such as MCP bearer tokens are not persisted.

### Execution Configuration

Capability Profiles are design-time route policy. A selected model must resolve to exactly one
route on the selected device. Missing and ambiguous mappings are errors; there is no default-route
fallback. Provider setups supply neutral catalogues and readiness but do not select a route.

### Execution Targets

Execution Targets own device readiness, endpoint routing, workspace materialization, and worktree
transitions. They do not transfer provider-native sessions. Changing a device affects only a later
invocation.

### Harness Engine

The Harness Engine owns Session-policy binding, provisioners, managed MCP selection/proxying,
skill-reader mediation, and per-invocation exposure. A route change reprovisions the binding; stale
provider-specific exposure is retired.

### Providers and runtime engine

Each provider has one application registration containing its configuration source, runtime, and
local launch preparer. Product-side setup/discovery lives under
`src-tauri/src/runtime/providers/<provider>/`; native protocol and process behavior lives under
`crates/orchid-engine/src/providers/<provider>/`.

The engine contract carries structured invocation content and neutral product intent. Codex and
Claude own their respective rendering, process configuration, native session handle, event
normalization, and interaction encoding. Remote host configuration is a provider-scoped settings
envelope. Remote Claude remains explicitly unsupported until a remote setup is implemented and
tested.

### Frontend

The composer lists the selected Capability Profile's models for the selected device. It does not
offer provider composition as a user choice. Application provider descriptors and feature-layer
provider UI registrations are the explicit composition points for setup screens and native route
settings.

## Continuity scenarios

| Later invocation | Native action | Orchid context |
| --- | --- | --- |
| Exact current provider configuration and device | Resume current handle | No missed-history handoff |
| Never-used provider/configuration/device | Start a new native session | Initial instructions and all canonical earlier turns |
| Previously used exact route | Resume that route's cached handle | Turns after its durable context cursor |
| Same provider on another configuration or device | Use that exact route's own binding, or start | Missed or full context as above |
| Steering or request response during an active invocation | Address the invocation's frozen runtime | No route resolution |

A stale provider cursor falls back to canonical history rather than suppressing context. Only
launch-accepted invocations may advance the cursor used when caching the current provider session.
Failed preparation never advances it.

## Remaining internal migration seams

Application-owned orchestration still supplies some first-turn instructions through the existing
`InitialPromptPrefix` input and persistence table. Agent Sessions consumes that value into a typed
`InitialInstructions` context part before the runtime boundary, so providers do not receive the
legacy wrapper. Replacing its storage with a general durable semantic-input record remains a
separate data migration.

Synchronous application delivery and recoverable prepared delivery both enter
`InvocationDeliveryService`, but their internal sequencing currently remains in
`application/invocation.rs` and `application/preparation/`. Moving those implementations under the
delivery directory is a structural follow-up; callers cannot bypass the delivery facade.

## Non-capabilities

The system does not steer a Codex invocation into Claude, move a running invocation to another
device, transplant native provider state, expose provider choice as prompt composition, or give OTP
general access to delivery internals. These are boundary violations rather than unsupported route
variants.
