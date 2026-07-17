/** Coherent facade for provider-neutral Sprint reads and recorded discovery compatibility. */
import type {
  SprintControlSurfaceProjection,
  SprintExecutionSnapshotV1,
  SprintPlannerOutputV1,
} from './sprintControlSurfaceCompatibility';
import {
  decodeSprintExecutionSnapshotV1,
  decodeSprintPlannerOutputV1,
} from './sprintControlSurfaceDecoder';
import { assembleSprintControlSurface } from './sprintReadModelAssembly';

export * from './sprintControlSurfaceCompatibility';
export * from './sprintReadModels';
export {
  decodeSprintExecutionSnapshotV1,
  decodeSprintPlannerOutputV1,
} from './sprintControlSurfaceDecoder';
export { deriveConcernState } from './sprintDerivedState';
export { projectSprintRelationshipGraph } from './sprintRelationshipGraph';

export function projectSprintControlSurface(
  plannerOutput: SprintPlannerOutputV1,
  executionSnapshot: SprintExecutionSnapshotV1,
  selectedPlanRevisionId = executionSnapshot.activePlanRevisionId,
): SprintControlSurfaceProjection {
  const planner = decodeSprintPlannerOutputV1(plannerOutput);
  const snapshot = decodeSprintExecutionSnapshotV1(executionSnapshot, planner);
  return assembleSprintControlSurface(planner, snapshot, selectedPlanRevisionId);
}
