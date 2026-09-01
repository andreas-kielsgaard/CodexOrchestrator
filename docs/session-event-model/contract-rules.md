# Contract and boundary rules

## Posture

Assume agents and contributors intend to follow the design. Make the correct route obvious, and add
technical enforcement where a violation would silently corrupt runtime truth. Do not attempt to
make every organizational preference impossible to violate.

These are low-cost defaults, not a demand for maximum isolation. A focused implementation may use a
different mechanism when it makes the same ownership and authority boundary easier to understand.

## Dependency direction

- Workflow may consume public Execution Configuration vocabulary and Session Event definition
  types.
- Session Events do not know about Workflow or Agent Sessions.
- Agent Sessions implement Session Event directory and dispatch ports.
- Execution Configuration does not depend on Workflow or Agent Sessions.
- Identity does not determine runtime capability or operational behavior.
- UI features consume application contracts, not repositories or raw Tauri commands.
- Infrastructure is selected in the application composition root.

## Strong technical boundaries

| Boundary              | Minimum enforcement                                                                    |
| --------------------- | -------------------------------------------------------------------------------------- |
| Cross-domain identity | Typed IDs and `ReferenceIdentity`; never display-name addressing.                      |
| Session creation      | One validated, coherent operation for Session, address and pinned profile persistence. |
| Profile resolution    | Pure resolver with explicit Runtime, Capability and Node inputs.                       |
| Session Profile       | Immutable resolved value persisted with the Session.                                   |
| Workflow compilation  | Authoring types enter; generic Session Event definitions leave.                        |
| Session Event effects | Ports for directory lookup, creation and invocation dispatch.                          |
| External data         | Validate Tauri DTOs, persisted JSON, MCP input and provider responses at entry.        |
| Direct user options   | Dedicated invocation request; never a Session Profile mutation.                        |
| Persistence           | Domain-owned schemas and repository mapping.                                           |
| Frontend transport    | Explicit mapping between backend DTOs and feature view models.                         |

Validation belongs at meaningful entry points. Validated values may generally be trusted inside the
owning module.

## Conventions rather than machinery

- Put behavior in the directory of the domain that owns it.
- Do not add new behavior to legacy Harness or Role modules.
- Do not introduce another universal effective-configuration object.
- Keep generic UI components free of domain imports and domain language.
- Use direct pure functions for transformations.
- Introduce a port for a cross-domain call, external effect or real replaceable integration, not
  for every function or class.
- Keep read projections separate from mutation authority.
- Query canonical records rather than reconstructing state from adjacent domains.

These rules need concise documentation and a few import restrictions, not a dependency-injection
framework or custom architecture compiler.

## Public module surface

Each domain should make its public route obvious through its module facade or frontend `index.ts`.
Internal repository mappings and helpers remain private or `pub(crate)` where practical. Consumers
should not reach through a module facade to reuse internal storage types.

A module contract should state:

1. what it owns;
2. what it accepts;
3. what it returns or records;
4. which invariants it guarantees;
5. which external effects it may perform; and
6. what it explicitly does not own.

Example:

> Workflow owns editable recipes and compilation. It consumes Capability and Node Profile
> vocabulary and emits Session Event definitions. It does not create, find or message Agent
> Sessions.

## Boundary tests

Prefer a small suite proving important relationships:

- Node defaults fit within node exposure and capability/runtime ceilings.
- Workflow compilation preserves prompt-source order.
- Session creation pins the resolved profile.
- later messages omit the initial prompt.
- direct-user options do not mutate the Session Profile.
- Session Events operate through ports without Workflow imports.
- Workflow compilation does not require Agent Session infrastructure.
- event deliveries retain group, source and target provenance.

## Avoid initially

- interface per class
- dependency-injection framework
- universal repository abstraction
- generic policy evaluation
- plugin registry for hypothetical providers
- schema-generated editors
- cross-domain event bus for ordinary calls
- repeated defensive validation at every internal method

The target should be difficult to misunderstand, not difficult to violate by force.
