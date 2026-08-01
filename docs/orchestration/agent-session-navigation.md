# Product-aware Agent Session navigation

Status: implemented recorded navigation and application contracts. This record does not claim user
acceptance, merge, release, or a complete production relationship source.

## Ownership map

| Relationship                    | Durable authority                                                                                                                 | Navigation use                                                                                                                                      |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| True Session ownership          | The Agent Session record owns its ID, title, availability, runtime binding, invocations, and events.                              | One canonical Session leaf and content surface. Every route loads that same ID through the injected `AgentSessionClient`.                           |
| Orchestration containment       | Epic contains Sprints. Sprint Plan revisions contain typed Work Slice planning-point membership and scoped Work Units.            | Epic, Sprint, accomplished/current planning-point, and Work Unit folders. This containment does not make the folder a Session owner.                |
| Typed Session relevance         | `AgentSessionReference` associates one provider-neutral Session with a typed target and semantic role.                            | Places a Session beside its one exact target and supplies product navigation. It is an association, not ownership.                                  |
| Planning-draft binding          | The native planning-draft/session association owns the pre-initiation link.                                                       | The Session remains directly under Independent Sessions and links to the draft.                                                                     |
| Runtime parent/child provenance | Runtime external-context IDs are provider bindings. `actorAgentSessionRefId` is causal provenance for a fact.                     | Not used for hierarchy. Neither field establishes a Session parent, child, or product owner.                                                        |
| Merely related or absent        | Multiple typed targets are legitimate relevance. `targetKind: other` and no reference provide no first-class product containment. | One leaf at the shared typed Epic when one exists; otherwise one leaf under Independent Sessions with explicit related-view actions when available. |

Titles, transcripts, current routes, Harness names, visual layout, and component state are never
relationship authority.

## Implemented hierarchy and interaction

- **Epics** and **Independent Sessions** are titled separators, not folders.
- each Epic is a folder. Direct Epic Sessions appear before Sprint folders.
- each Sprint is a folder. Direct Sprint Sessions appear before its Work Slice planning-point folders.
- planning-point folders use the recorded accomplished/current scope title. Their direct Planner
  Sessions precede Work Unit folders; no generic Planning or Execution level is invented.
- Work Unit Handler and Work Unit Implementer references sit under the Work Unit resolved through
  the execution's fixed scope and planning-point membership. Review and correction turns remain on
  the Handler; no separate Reviewer role or Session is fabricated.
- planning drafts, provider-neutral Sessions, unassociated references, and multi-Epic Sessions are
  direct children of Independent Sessions. There is no Unassigned folder.
- one same-Epic multi-target Session appears once at the shared Epic containment level. Its header
  lists every typed destination instead of selecting an owner.
- selection and expansion are application-owned while switching surfaces. A selected Session stays
  selected when an ancestor collapses; the collapsed folder exposes that it contains the selection
  and can recover it when reopened.
- tree focus follows Up/Down, Left/Right, Home/End, Enter, Space, and roving-tabindex behavior across
  both titled sections.
- the navigation/content boundary is pointer- and keyboard-resizable. At the compact breakpoint it
  stacks vertically while retaining the same Session selection.
- standalone Agent Sessions renders only the Session as its content surface. A single typed product
  destination gets a direct action, multiple destinations get an explicit chooser, and none gets no
  action.

## Identity and status

The tree accepts a Session-keyed Agent identity, Harness role, and visual token. When the
application does not supply that binding, it shows a neutral Agent mark and the recorded semantic
role; it does not manufacture an Agent or Harness. Processing, completion, failure, cancellation,
and interruption come from the latest durable invocation summary.

## Recorded and production limits

The recorded composition demonstrates Epic, Sprint, Work Slice planning point, Work Unit, planning-draft,
multi-target, and independent grouping. Production records currently do not supply every Runner,
Planner, Handler, or Implementer Session reference, so absent folders remain absent.

The application has no durable canonical-owner marker for a genuinely multi-target Session.
Shared-Epic or Independent placement is therefore explicitly containment, not inferred ownership.
Durable expansion persistence across restart, hierarchy levels beyond the current typed target
kinds, and a production Harness identity source remain deferred.

## Reconciled detail contract

The Agent Sessions route and accepted Sprint/Epic detail route now share typed product-location and
open-Session callbacks through `App`, `OrchestrationSection`, `EpicDetail`, `SprintWorkspace`, and
the shared Session panel. The detail route retains its flow, lifecycle, Documents/file review, and
resizer contracts; standalone navigation does not reconstruct them.
