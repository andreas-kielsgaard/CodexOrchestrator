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
- An activation affects future action instantiations only.
- An action is instantiated, and captures the current recipe configuration, when its trigger fires.
- Connections target specific role-instance nodes, not roles or groups.

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

- A workflow has a starting point.
- Parent and child are abstract flow-relative categories.
- If seniority is needed, it can be derived from the earliest step at which a node is reachable from
  the starting point.
- Nodes first reachable at the same step have equal seniority.
- No consequential use of seniority has been accepted.

### Validation

- If an activated recipe contains an error, activation should be blocked.
- What constitutes a structural error is unresolved. The current conceptual model does not yet make
  such errors evident to the user.

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

### Possible structural errors

The following are hypotheses based on assumptions that have not been accepted:

- A connection refers to a node that has been removed from the live recipe.
- A connection subscribes to a trigger or output that its source no longer exposes.
- An operation requires an input or MCP tool removed by another independently activated edit.
- A producer and consumer use incompatible enforced artifact formats.
- The configured starting point has been removed or made unavailable.
- A node inherits a role that has been removed without first detaching or replacing the node.

These problems exist only if the engine permits the referenced elements to be independently removed
or changed. Strong references and editing constraints could instead prevent invalid states from
being expressible. Compatibility warnings may still exist without constituting structural errors.

### Potential uses of seniority

Seniority would become consequential only if the engine used it for behavior such as default context
direction, escalation, authority, permission inheritance, conflict resolution, or visual layout.
None of those uses is currently assumed or accepted. Reachability depth may be useful only as a
descriptive graph property.

## Open questions

- Should invalid graph states be impossible to express, representable in a draft but blocked from
  activation, or allowed as inactive paths?
- Which edits count as removal: deletion, disabling, renaming, or changing a port contract?
- Are triggers and artifact formats typed contracts or flexible labels interpreted by operations?
- Does a workflow require exactly one starting node, at least one starting node, or explicit external
  entry triggers?
- Is seniority merely descriptive, or should any engine policy consume it?

