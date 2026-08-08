# Workflow Engine concept record

Status: conceptual exploration only. This record does not describe implemented behavior or authorize
implementation. It separates the user's conceptual basis and decisions from proposals that remain
open.

## Conceptual basis from the user

Build a moddable agentic workflow engine, analogous to composing components in the Warcraft 3
custom-game engine.

- The mapping surface contains nodes representing agent-role instances.
- A role is defined through the Harness surface, including its instructions, skills, MCP tools, and
  other Harness configuration.
- Roles and their instances participate in a recipe connected through triggers and tools.
- Triggers may include an agent finishing or an MCP call finishing.
- A result may contain one or more files of an expected format. A later node may consume a fixed
  file set, such as a handoff, to construct its prompt.
- Application operations may prompt an agent, prompt a user, run a script, or perform other future
  actions.
- The engine runs directly from the live recipe. Activating an edited element updates that recipe.
- A trigger captures the then-current recipe when it fires. Already-instantiated actions and active
  Agent Sessions are not interrupted by later recipe changes.

## Current understanding from the discussion

This is an assistant synthesis and can be corrected as the concept develops.

- The first version is primarily a visual definition and execution surface, not a workflow-proof or
  policy-enforcement system.
- The recipe designer owns semantic correctness. The engine should prevent an incoherent graph from
  becoming live, but it should otherwise permit weak prompts, missing runtime files, unsuitable
  payloads, and other poor recipe choices to fail naturally.
- A connection is initially sender-oriented. It observes or accepts something from the sender,
  exposes values through a connection-type-specific output interface, constructs an action input,
  and activates the target.
- Connection contracts are initially practical payload shapes rather than negotiated sender and
  receiver schemas.
- Files and text blocks are the first useful payload primitives. More elaborate contracts should be
  added only after concrete needs emerge.
- Live editing is important, but changes affect future trigger firings rather than interrupting work
  already in progress.
- The initial design should provide a small set of composable primitives and avoid infrastructure
  added only for speculative future functionality.

## Decision register

### Roles and instances

- Roles support one level of inheritance. Chained inheritance is deferred.
- A role instance can disconnect from inheritance and become a one-off definition.
- A role instance can be saved as a new role. The new role contains the instance's complete
  effective configuration.
- Editing a role changes its inheriting nodes except where a node has a local value for that
  property.
- Every Harness property can be changed locally on an instance.
- Inheritance and local configuration are indicated property by property.
- A new node can start from an existing role or from scratch.
- Role editing is available from the instance-configuration surface but remains a distinct action.

### Recipe editing and activation

- Every configurable element has its own Activate control.
- A bulk activation surface lists edited elements and supports selecting any subset or all edits.
- Draft edits may temporarily contain structural errors, but an activation is blocked if its
  selected changes would make the live recipe structurally invalid.
- An activation affects future action instantiations only.
- An action is instantiated, and captures the current recipe configuration, when its trigger fires.
- Connections target specific role-instance nodes, not roles or groups.
- Deleting a node deletes its outgoing connections.
- Connections entering a deleted node remain as dangling draft elements. They can be dragged to a
  replacement or different target, or deleted before activation.
- A node deletion can be activated only after all of its dangling dependent connections have been
  reconnected or deleted.

### Session and context policy

- Context policy has node defaults and connection-specific overrides.
- The default context policy is to continue the session.
- Each activation selects a continued or fresh session.
- A continued session cannot inherit context from another session. Its existing context may
  optionally be compressed.
- A fresh session may start without inherited context or inherit context from a selected source.
- Inherited context may optionally be compressed.
- Compression is independent of the fresh/continued choice and always occurs before the triggering
  prompt is delivered.
- Initial context behavior may vary by trigger. Examples include continuing after a child reports
  back, continuing when answering a parent query in the same scope, and starting fresh when a parent
  establishes a new activity scope.

### Workflow position

- A workflow currently requires exactly one starting point.
- Parent and child are abstract flow-relative categories.
- If seniority is needed, it can be derived from the earliest step at which a node is reachable from
  the starting point.
- Nodes first reachable at the same step have equal seniority.
- Seniority is reserved, but no consequential use has been accepted.
- Multiple entry nodes and external entry triggers are potential future concepts.

### Scope discipline

- Potential future functionality must be recorded without being prematurely supported when that
  support would add complexity to the initial build.

### Validation

- A structurally invalid workflow cannot be activated.
- A draft may remain structurally invalid while the user reconnects or removes dangling elements.
- An individual or bulk activation validates the complete resulting live recipe, not only the
  selected elements.
- Known structural errors include a dangling connection and the absence of exactly one starting
  point.
- The initial engine focuses on defining recipes, not proving that they are useful or robust.
- The recipe designer may activate semantically poor configurations. The engine does not initially
  verify every payload assumption, prompt, expected file, or runtime outcome.

## Proposals to explore

These are assistant proposals, not decisions.

### Event, operation, and artifact vocabulary

Consider separating:

- events, such as an agent lifecycle observation, MCP completion, or artifact publication;
- conditions evaluated after an event;
- application operations, such as prompting an agent or user or running a script; and
- typed artifacts whose payload may contain one or several files.

This would make a connection an event-condition-operation rule while keeping its visual form a
simple arrow.

"Agent finished" may need more precise events. Runtime termination, provider-terminal observation,
MCP completion, artifact production, and application acceptance can occur independently.

### Connection contracts

The initial contract is sender-defined and intentionally simple. The sender is assumed to provide
something the receiver can use. More expressive receiver or shared contracts are deferred.

The sender satisfies the connection when its trigger results in target activation. This is a simple
behavioral boundary for the first version, not a durable delivery or semantic-acceptance guarantee.

### Trigger, output, and action shape

A connection contains:

1. a trigger source;
2. a routine that constructs an output through the trigger source's output interface; and
3. a resulting action that consumes the constructed output.

Initial trigger-source categories are:

- a hook observing agent-session behavior;
- an MCP call made by the sender; and
- an actively monitored deterministic routine.

Each hook, MCP call, or deterministic routine exposes an output interface whose values can be used
to construct the resulting prompt or another action input. Payload-construction options depend on
the connection type.

Initial examples:

- An MCP call accepts a list of files and a prompt from the sender. Its connection output exposes
  the file list and text block for construction of the resulting action.
- A turn-finished hook expects a configured file to exist after the sender session finishes its
  turn. Its connection output exposes the file location, a connection-defined description text
  block, and a fixed prompt text block for construction of the resulting action.

### Structural validity still to define

Dangling connections and an invalid starting-point count are structural errors. It remains open
whether any other state makes the graph impossible to activate. Unavailable triggers, missing
tools, incompatible payloads, absent expected files, and unusable prompts should initially be
treated as recipe or runtime failures rather than exhaustively verified before activation.

### Potential uses of seniority

Seniority would become consequential only if the engine used it for behavior such as default context
direction, escalation, authority, permission inheritance, conflict resolution, or visual layout.
None of those uses is currently assumed or accepted. Reachability depth may be useful only as a
descriptive graph property.

## Open questions

- How are MCP triggers named and exposed to the sender?
- How is an expected file location configured for a turn-finished hook?
- How does a connection map its trigger output fields into a target prompt or other action input?
- What qualifies as an actively monitored deterministic routine in the first version?
- Should any engine policy eventually consume reserved seniority?
