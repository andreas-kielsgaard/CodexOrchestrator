# Workflow authoring concept

Status: working conceptual guide. Detailed target material is segmented under
[`docs/session-event-model/`](../session-event-model/README.md). This record summarizes how Workflow
authoring relates to that foundation; implementation details may be refined from targeted evidence.

## Purpose

Workflow is a product-owned authoring and presentation layer for managed Session Events. It should
make common flows visually editable without becoming a separate runtime model for Session creation,
addressing or messaging.

The visual recipe may be graph-shaped, but this does not predetermine the presentation of a running
Workflow instance. Runtime truth consists of Session, event, delivery and provenance records; later
views may project that truth as a graph, timeline, hierarchy or Workflow-specific surface.

## Nodes

A node represents an operational point in a Workflow recipe. Its first target shape contains:

- a node identity within the Workflow recipe;
- a Capability Profile reference;
- an embedded Node Profile;
- an optional Agent Identity reference; and
- canvas and presentation metadata owned by Workflow.

The Node Profile supplies an initial prompt, node-exposed capability subsets and pinned defaults.
It is not a reusable referenced entity in the first implementation. Copying a node's configuration
creates independent state that can be edited from there.

Generic Workflow Roles, role inheritance and save-as-role behavior are not part of this target. The
curated Epic/Sprint orchestration feature may retain its own role terminology without making Role a
general Workflow configuration object.

## Connections

A connection is an authoring abstraction for one Session Event definition. It describes:

1. a trigger binding;
2. ordered prompt sources; and
3. target Session selection, including missing-target behavior.

Initial prompt sources may include fixed connection text, occurrence/MCP parameters, referenced
content and application-provided input. A creation event also includes the destination node's
initial prompt. A later message to an existing Session does not.

Target selection can use a logical address derived from Workflow instance and node identity, with
supported filters such as created-by, newest, last addressed or currently running. It may address
the first match, all matches or create a Session when no match exists where the configuration
allows it.

The deeper application boundary uses generic `ReferenceIdentity` and `SessionLogicalAddress`
values. Workflow owns their mapping from its recipe and instance identities; Session Events should
not import Workflow domain types.

## Triggers

Triggers remain explicit in the authoring model because they help describe a connected flow. The
first implementation does not need a general trigger scheduler or provider registry. Existing MCP,
application, invocation and user-entry integrations may call the Session Event ingress operation
directly while recording the relevant trigger occurrence.

Later trigger, prompt or target providers can be added when a concrete use requires a new contract.
A script language and fully general extension mechanism are not initial objectives.

## Compilation and activation

Workflow authoring state compiles into Session Event definitions. Compilation is a useful boundary
for structural validation and for keeping Agent Sessions independent from Workflow semantics.

A practical initial activation path may:

- allow incomplete or invalid draft edits;
- validate the resulting active recipe;
- compile its supported nodes and connections; and
- use the compiled definitions for future occurrences.

The exact persistence strategy for compiled definitions may follow evidence from the existing
Workflow repository. The important behavior is that an occurrence uses a coherent definition and
an existing Session is not silently reconfigured when the authoring recipe changes.

## Session behavior

- Session configuration is resolved and pinned when the Session is created.
- Workflow-addressed messages use the attached Node Profile defaults.
- Direct user messages may select model and reasoning per invocation from the attached runtime
  capability.
- An implicitly displayed Session can accept ordinary user input without requiring a visible
  Workflow definition for that UI behavior.
- Event groups and deliveries preserve source, prompt and target provenance for future runtime
  projections.

## Initial validation posture

The engine should reject structurally incoherent definitions and invalid capability/default
relationships. It does not initially need to prove that prompts are useful, files will exist,
targets will always resolve or a Workflow will produce a desirable result. Those may fail plainly
at runtime and motivate later validation when concrete use demonstrates value.

## Deferred questions

- canonical presentation of a running Workflow instance;
- custom Workflow-specific runtime projections;
- user-provided trigger, prompt and target implementations;
- custom context compression and handoff routines;
- additional deterministic application operations;
- reusable configuration concepts that become justified by observed repetition.

See the [functional happy flow](../session-event-model/functional-happy-flow.md) and
[delivery sequence](../session-event-model/delivery-sequence.md) before implementing these concepts.
