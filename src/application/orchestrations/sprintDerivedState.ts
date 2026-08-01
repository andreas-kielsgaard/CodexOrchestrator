import type {
  SprintExecutionSnapshotV1,
  SprintRunnerPlanV1,
} from './sprintControlSurfaceCompatibility';
import type {
  ConcernState,
  WorkUnitExecutionState,
  WorkUnitPresentationState,
} from './sprintReadModels';

export function deriveWorkUnitPresentation(
  unit: SprintRunnerPlanV1['workUnits'][number],
  execution: WorkUnitExecutionState,
  states: ReadonlyMap<string, SprintExecutionSnapshotV1['workUnits'][number]>,
): WorkUnitPresentationState {
  if (execution === 'accepted') return 'completed';
  if (['deferred', 'blocked', 'under_review', 'working'].includes(execution))
    return execution as WorkUnitPresentationState;
  if (
    unit.dependencies.some(
      ({ kind, workUnitId }) =>
        ['hard', 'gated'].includes(kind) &&
        workUnitId !== undefined &&
        states.get(workUnitId)?.state !== 'accepted',
    )
  )
    return 'waiting_for_dependencies';
  return execution === 'projected' ? 'not_started' : 'working';
}

export function deriveConcernState(
  concern: SprintRunnerPlanV1['concerns'][number],
  decision: SprintExecutionSnapshotV1['concernDecisions'][number] | undefined,
  states: ReadonlyMap<string, WorkUnitPresentationState>,
): ConcernState {
  if (decision?.kind === 'deferred') return 'deferred';
  if (decision?.kind === 'accepted') return 'completed';
  const required = concern.requiredWorkUnitIds.map((id) => states.get(id) ?? 'not_started');
  if (required.includes('blocked')) return 'blocked';
  if (required.includes('working')) return 'working';
  if (required.includes('under_review')) return 'under_review';
  if (required.length > 0 && required.every((state) => state === 'completed')) return 'completed';
  if (
    required.includes('waiting_for_dependencies') ||
    (required.some((state) => state !== 'not_started') && required.includes('not_started'))
  )
    return 'waiting_for_dependencies';
  return 'not_started';
}
