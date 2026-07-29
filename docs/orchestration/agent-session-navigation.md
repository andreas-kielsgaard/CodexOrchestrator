# Product-aware Agent Session navigation

Status: implemented recorded navigation and application contracts. This record does not claim user
acceptance, merge, release, or a complete production relationship source.

## Ownership map

| Relationship                 | Durable authority                                                                                                                                        | Navigation use                                                                                                                                       |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Agent Session lifecycle      | Agent Session record owns its ID, title, availability, runtime binding, invocations, and events.                                                         | One Session leaf. Every view loads the same ID through the injected `AgentSessionClient`.                                                            |
| Product containment          | Epic owns Sprints. A Sprint owns its logical Plan; revisions contain scoped Work Units and Planner Activities.                                           | Epic -> Sprint -> Planning/Execution folders.                                                                                                        |
| Session relevance            | `AgentSessionReference` associates one provider-neutral Session with a typed target and semantic role. It is not an ownership record.                    | Places a singly related Session beside its target and labels the recorded role.                                                                      |
| Active Epic planning draft   | The native planning-draft/session association owns the pre-initiation link.                                                                              | Independent Sessions -> Epic planning drafts. After durable initiation, the same association projects an Epic Plan Builder reference under the Epic. |
| Runtime/provenance           | Runtime external-context IDs remain provider bindings. `actorAgentSessionRefId` is causal provenance for a fact. Neither is a Session parent/child edge. | Not used for hierarchy. No parent/child navigation is inferred.                                                                                      |
| Other or absent relationship | `targetKind: other` is recorded relevance without a first-class destination; an unreferenced Session has no orchestration relationship.                  | Separate independent folders.                                                                                                                        |

Titles, transcripts, current routes, Harness names, visual layout, and component state are never
relationship authority.

## Implemented projection

- Epic-level Runner and Plan Builder references sit directly under their Epic.
- Sprint-level Runner references sit directly under their Sprint.
- Sprint Planner and Work Unit planner references sit under the exact recorded Planner Activity.
- handler, worker, and reviewer references to a Work Unit execution sit under the Work Unit resolved
  through that execution's fixed scope and Planner Activity membership.
- a Session with more than one legitimate destination appears once under **Multiple related views**.
  Its Session header exposes every typed destination in an explicit chooser.
- an active planning draft, other recorded relevance, and unreferenced Session remain independently
  discoverable. No Epic is inferred for them.
- tree selection and expansion are application-owned while switching surfaces. Keyboard behavior
  follows tree conventions: Up/Down, Left/Right, Home/End, and roving focus.
- orchestration panels can open their exact Session in standalone Agent Sessions. The standalone
  pane shows only the Session content and a direct typed return action, or a chooser when needed.

## Identity and status

The tree accepts an optional session-keyed Agent identity, Harness role, and visual token. When no
such application binding is supplied, it shows a neutral Agent mark plus the recorded semantic
role; it does not manufacture an Agent name or Harness binding. Invocation summaries now carry the
latest durable invocation status so processing, completed, failed, canceled, and interrupted states
are not guessed from inactivity.

## Recorded and production limits

The recorded development composition demonstrates Epic, Sprint, Planner Activity, Work Unit,
reviewer, and independent grouping. Product native-query composition now projects the durable
initiated Plan Builder association to its Epic. Current production records do not yet supply Runner,
Sprint Planner, or Work Unit Session references, so those production folders remain absent until
their typed facts exist.

The separate Harness Management exploration defines a session-owned identity/binding read model,
but it is not present on this branch. This work keeps identity input optional and session-keyed for
later consolidation; it does not copy, select, or reconstruct a Harness. Durable Session tree
expansion persistence across application restart, a durable canonical owner marker for genuinely
multi-target Sessions, and any hierarchy levels beyond the recorded target kinds are deferred.

## Consolidation boundary

The active Sprint/Epic detail redesign overlaps `App`, `OrchestrationSection`, `EpicDetail`,
`SharedAgentSessionPanel`, `SprintWorkspace`, `WorkUnitDetailWorkspace`, their tests, and shared
panel styling. It remains untouched. The navigation projection, typed product locations, standalone
tree, and session-keyed identity input are separable. Consolidation should re-thread the typed
location and open-Session callbacks through the redesigned detail components without replacing its
layout, then repeat interaction and responsive evidence.
