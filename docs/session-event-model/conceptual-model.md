# Conceptual model

The vocabulary and ownership distinctions are the stable direction. The diagram shows a useful
dependency shape rather than requiring a particular file layout or an interface for every arrow.

## Dependency shape

```text
Native runtime integration
        |
        | RuntimeProfileSnapshot
        v
Execution Configuration
  Capability Profile ---------+
                               | resolution at Session creation
  Workflow-owned Node Profile -+
                               v
                    immutable Session Profile

Workflow Authoring --compile--> Session Event Definitions
                                      |
                                      v
                              Session Event runtime
                         address / materialize / record
                                      |
                      SessionDirectory + Dispatcher
                                      |
                                      v
                                Agent Sessions

Agent Identity ---------------- presentation and assignment
Read projections <------------- runtime records from each domain
```

## Vocabulary and ownership

| Concept                  | Meaning                                                                     | Owner                                                    |
| ------------------------ | --------------------------------------------------------------------------- | -------------------------------------------------------- |
| Runtime Profile          | Observed capabilities and locked selections of the selected native runtime. | Runtime integration                                      |
| Capability Profile       | Reusable allowed technical capabilities selected from the Runtime Profile.  | Execution Configuration                                  |
| Node Profile             | Workflow-owned restrictions, defaults and initial prompt for one node.      | Execution Configuration vocabulary; Workflow persistence |
| Session Profile          | Resolved, immutable runtime configuration pinned at Session creation.       | Execution Configuration; persisted with Agent Session    |
| Workflow                 | Editable recipe of nodes and connections.                                   | Workflow                                                 |
| Session Event definition | Definition-time trigger, ordered prompt sources and target-selection logic. | Session Events                                           |
| Session Event occurrence | Runtime inputs supplied when a definition is triggered.                     | Session Events                                           |
| Event group              | One materialized fan-out operation and its shared provenance.               | Session Events                                           |
| Delivery                 | One target-specific dispatch attempt belonging to an event group.           | Session Events                                           |
| Logical Session address  | A conceptual address such as Workflow instance plus node identity.          | Session Events; mapped by Workflow                       |
| Reference identity       | Namespace, kind and identifier used across application boundaries.          | Session Events shared boundary                           |
| Agent Identity           | User-facing name, initials, color and shape for grouped agent work.         | Identity                                                 |

## Lifecycle distinctions

Definition-time configuration and runtime truth must remain separate:

1. A Runtime Profile is observed.
2. A Capability Profile selects an allowed ceiling.
3. A Workflow node embeds a Node Profile under that ceiling.
4. Workflow activation compiles nodes and connections into Session Event definitions.
5. An occurrence materializes one definition into a Session Event command.
6. If a target Session must be created, Capability and Node Profiles resolve into a Session
   Profile and that resolution is pinned.
7. Deliveries record which Sessions were resolved, what prompt contributions were used and what
   invocation resulted.

Later changes to Runtime, Capability or Node Profiles do not rewrite an existing Session Profile.

## Product decisions

- Node Profiles are not reusable managed entities yet. Copying creates independent embedded state.
- All Session configuration is pinned at creation.
- Node initial prompts contribute only to creation events.
- Later messages use connection/event prompt sources without repeating the initial prompt.
- Workflow-addressed messages use the attached Node Profile defaults.
- Direct user messages may choose model and reasoning per message from the Session's attached
  runtime exposure.
- Trigger types remain explicit even when the current application or MCP process performs the
  actual triggering.
- The first implementation supports the concrete trigger, prompt and target variants needed by the
  product. A provider registry or script language is deferred.
- The canonical run UI is not predetermined. Durable runtime records support later projections.

## Concepts deliberately absent

- Generic Workflow Role
- reusable Node Profile catalogue
- mutable Session Harness
- universal effective configuration object
- automatic Session reconfiguration after definition changes
- identity-derived runtime policy
