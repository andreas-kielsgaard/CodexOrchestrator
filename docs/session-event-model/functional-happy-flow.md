# Minimal functional happy flow

The first target is one complete backend path. It proves the replacement boundaries without
building speculative extension or hardening systems.

The operations below define the proof that should remain possible. Their internal subdivision is a
recommendation: agents may combine or separate services when more local context makes the ownership
clearer without coupling the domains.

## Minimum implementations

| Element                  | Minimum behavior                                                                                                             |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| Runtime Profile          | Read the globally selected Codex profile. Return explicit inherited/mock capability catalogues where control is unavailable. |
| Capability Profile       | Persist ID, name, revision and allowed capabilities. Support list, read, create and update.                                  |
| Node Profile             | Persist directly in a Workflow node. Validate capability subsets and pinned defaults.                                        |
| Copy node state          | Deep-copy one embedded Node Profile to another node, then edit independently.                                                |
| Workflow connection      | Persist one concrete trigger, ordered prompt sources and target-selection definition.                                        |
| Workflow compiler        | Compile the active authoring definition into Session Event definitions and return direct validation errors.                  |
| Session Event ingress    | Accept a definition plus occurrence input, materialize it, resolve targets and dispatch deliveries.                          |
| Session addressing       | Support exact Session and logical Workflow-instance/node addressing plus accepted selectors.                                 |
| Session creation         | Resolve and pin the Session Profile, create/address the Session and deliver the initial prompt coherently.                   |
| Existing Session message | Address the target and deliver ordered event prompt sources without the initial prompt.                                      |
| Direct user message      | Apply optional model/reasoning to one invocation without mutating Session state.                                             |
| Event records            | Store one group and target-specific deliveries with source, prompt and outcome provenance.                                   |
| Identity assignment      | Carry an optional identity reference through Session creation without affecting capability resolution.                       |
| Referenced content       | Resolve only the concrete expected-file/reference source needed by current Workflow use.                                     |

## End-to-end proof

1. Observe the global Runtime Profile.
2. Create a Capability Profile.
3. Create a Workflow node with a Capability Profile reference and embedded Node Profile.
4. Add a connection with trigger, ordered prompt sources and target selection.
5. Activate and compile the Workflow.
6. Materialize a creation event.
7. Resolve and persist an immutable Session Profile.
8. Create and logically address the Agent Session.
9. Deliver the concatenated creation prompt, including the Node initial prompt.
10. Materialize another event and address the existing Session.
11. Deliver its prompt without repeating the initial prompt.
12. Query the Session Profile, event group and delivery records.
13. Send a direct user message with optional model/reasoning choices.
14. Confirm that the Session Profile did not change.

## Required invariants

- Node exposure is a subset of the Capability Profile and supported Runtime Profile.
- Pinned defaults belong to the Node exposure.
- Prompt contributions preserve their configured order.
- Initial prompt contributions are used only when a Session is created.
- Session creation does not leave an unaddressed Session when profile/address persistence fails.
- Persisted Session Profile truth is not reconstructed from current profile state.
- Workflow delivery uses Node defaults; direct user options remain invocation-local.
- Event group and deliveries retain source and target relationships.
- Cross-domain references use complete typed identities rather than display names.

## Deferred

- provider/API-key and multiple Codex-profile management
- reusable Node Profile entities
- Capability Profile history/comparison UI
- general trigger, prompt or target provider registration
- Workflow scripting
- dynamic Session reconfiguration
- retries, repair dashboards and generalized idempotency systems beyond required coherent creation
- custom runtime projections and canonical run graph design
- generic policy engine

Minimal means narrow concrete adapters behind the intended contracts. It does not mean duplicating
the domain model in temporary legacy types.
