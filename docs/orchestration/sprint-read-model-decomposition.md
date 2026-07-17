# Sprint read-model decomposition

Status: current decomposition record. `sprintControlSurface.ts` is the public facade.

- `sprintReadModels.ts`: provider-neutral application read contracts.
- `sprintControlSurfaceCompatibility.ts`: provisional recorded/discovery input and UI compatibility
  shapes.
- `sprintControlSurfaceDecoder.ts`: compatibility decoding and referential validation.
- `sprintDerivedState.ts`: pure presentation derivation.
- `sprintRelationshipGraph.ts`: semantic relationship projection.
- `sprintReadModelAssembly.ts`: final read-model and recorded presentation assembly.
- `recordedPlanWorkflow.ts`: fixture-only workflow geometry and validation.

The application read model has one independent Sprint Plan identity with revision links. Sprint
Planner Activities remain independent and may share a revision. Recorded UI cards use explicit
activity membership; no application projection creates Plan objects from activities.

Recorded compatibility facts do not establish product authority. They cannot prove an execution
request, fixed instantiated scope, provider action, persistence, continuation initiation, or
document opening. Epic and Sprint continuation remain separate, and completion is derived only from
explicit accepted responsibility state.
