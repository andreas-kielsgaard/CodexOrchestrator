/** Fixture-only workflow geometry. It is not an Sprint Plan or product execution contract. */
export const RECORDED_PLAN_WORKFLOW_V1 = 'plan-workflow/v1' as const;

export type PlanWorkflowActorKind =
  | 'sprint_runner'
  | 'work_slice_planner'
  | 'work_unit_handler'
  | 'work_unit_implementer'
  | 'repository';

export type PlanWorkflowStepKind =
  | 'sprint_started'
  | 'repository_reevaluated'
  | 'ready_work_determined'
  | 'handler_created'
  | 'implementer_created'
  | 'implementer_progress'
  | 'implementer_return'
  | 'handler_review'
  | 'correction_required'
  | 'implementer_reprompted'
  | 'handler_integration'
  | 'work_unit_settled'
  | 'work_slice_planner_completed'
  | 'sprint_runner_outcome';

export type PlanWorkflowPhase =
  | 'sprint_start'
  | 'repository_assessment'
  | 'scope'
  | 'work_unit_start'
  | 'implementer_start'
  | 'first_return'
  | 'first_review'
  | 'correction'
  | 'second_return'
  | 'second_review'
  | 'integration'
  | 'settled'
  | 'planning_complete'
  | 'sprint_return';

export interface PlanWorkflowStepV1 {
  readonly id: string;
  readonly actorId: string;
  readonly kind: PlanWorkflowStepKind;
  readonly phase: PlanWorkflowPhase;
  readonly title: string;
  readonly summary: string;
  readonly cycle?: number;
}

export interface RecordedPlanWorkflowV1 {
  readonly version: typeof RECORDED_PLAN_WORKFLOW_V1;
  readonly workSlicePlanningPointId: string;
  readonly scopeSummary: string;
  readonly fixtureKind: 'recorded_theoretical';
  readonly actors: readonly {
    readonly id: string;
    readonly kind: PlanWorkflowActorKind;
    readonly label: string;
    readonly workUnitId?: string;
  }[];
  readonly sharedStart: readonly PlanWorkflowStepV1[];
  readonly workUnitLanes: readonly {
    readonly id: string;
    readonly workUnitId: string;
    readonly title: string;
    readonly handlerActorId: string;
    readonly implementerActorId: string;
    readonly steps: readonly PlanWorkflowStepV1[];
  }[];
  readonly sharedCompletion: readonly PlanWorkflowStepV1[];
  readonly interactions: readonly {
    readonly id: string;
    readonly kind: 'sequence' | 'return' | 'correction_loop' | 'report';
    readonly fromStepId: string;
    readonly toStepId: string;
    readonly sameActorId?: string;
  }[];
}

export function decodeRecordedPlanWorkflowV1(value: unknown): RecordedPlanWorkflowV1 {
  if (!isRecord(value) || value.version !== RECORDED_PLAN_WORKFLOW_V1)
    fail('invalid Plan workflow version');
  string(value.workSlicePlanningPointId, 'workSlicePlanningPointId');
  string(value.scopeSummary, 'scopeSummary');
  if (value.fixtureKind !== 'recorded_theoretical') fail('invalid fixtureKind');
  const actors = array(value.actors, 'actors');
  const actorIds = uniqueIds(actors, 'actor');
  actors.forEach((candidate) => {
    if (!isRecord(candidate)) fail('invalid actor');
    string(candidate.label, 'actor label');
    if (
      ![
        'sprint_runner',
        'work_slice_planner',
        'work_unit_handler',
        'work_unit_implementer',
        'repository',
      ].includes(String(candidate.kind))
    )
      fail('invalid actor kind');
    if (candidate.workUnitId !== undefined) string(candidate.workUnitId, 'actor workUnitId');
  });
  const sharedStart = validateSteps(value.sharedStart, actorIds);
  const sharedCompletion = validateSteps(value.sharedCompletion, actorIds);
  const lanes = array(value.workUnitLanes, 'workUnitLanes');
  uniqueIds(lanes, 'Work Unit lane');
  const laneSteps = lanes.flatMap((candidate) => {
    if (!isRecord(candidate)) fail('invalid Work Unit lane');
    string(candidate.workUnitId, 'workUnitId');
    string(candidate.title, 'title');
    reference(candidate.handlerActorId, actorIds, 'handler actor');
    reference(candidate.implementerActorId, actorIds, 'implementer actor');
    return validateSteps(candidate.steps, actorIds);
  });
  const allSteps = [...sharedStart, ...laneSteps, ...sharedCompletion];
  const stepIds = uniqueIds(allSteps, 'workflow step');
  const interactions = array(value.interactions, 'interactions');
  uniqueIds(interactions, 'interaction');
  interactions.forEach((candidate) => {
    if (!isRecord(candidate)) fail('invalid interaction');
    if (!['sequence', 'return', 'correction_loop', 'report'].includes(String(candidate.kind)))
      fail('invalid interaction kind');
    reference(candidate.fromStepId, stepIds, 'interaction source');
    reference(candidate.toStepId, stepIds, 'interaction target');
    if (candidate.sameActorId !== undefined)
      reference(candidate.sameActorId, actorIds, 'interaction same actor');
  });
  return value as unknown as RecordedPlanWorkflowV1;
}

function validateSteps(value: unknown, actorIds: ReadonlySet<string>): PlanWorkflowStepV1[] {
  const steps = array(value, 'steps');
  steps.forEach((candidate) => {
    if (!isRecord(candidate)) fail('invalid workflow step');
    reference(candidate.actorId, actorIds, 'step actor');
    string(candidate.title, 'step title');
    string(candidate.summary, 'step summary');
    if (
      ![
        'sprint_started',
        'repository_reevaluated',
        'ready_work_determined',
        'handler_created',
        'implementer_created',
        'implementer_progress',
        'implementer_return',
        'handler_review',
        'correction_required',
        'implementer_reprompted',
        'handler_integration',
        'work_unit_settled',
        'work_slice_planner_completed',
        'sprint_runner_outcome',
      ].includes(String(candidate.kind))
    )
      fail('invalid workflow step kind');
    if (
      ![
        'sprint_start',
        'repository_assessment',
        'scope',
        'work_unit_start',
        'implementer_start',
        'first_return',
        'first_review',
        'correction',
        'second_return',
        'second_review',
        'integration',
        'settled',
        'planning_complete',
        'sprint_return',
      ].includes(String(candidate.phase))
    )
      fail('invalid workflow phase');
    if (
      candidate.cycle !== undefined &&
      (!Number.isInteger(candidate.cycle) || Number(candidate.cycle) < 1)
    )
      fail('invalid workflow cycle');
  });
  return steps as unknown as PlanWorkflowStepV1[];
}

function uniqueIds(values: readonly unknown[], label: string): Set<string> {
  const result = new Set<string>();
  values.forEach((candidate) => {
    if (!isRecord(candidate)) fail(`invalid ${label}`);
    const id = string(candidate.id, `${label} id`);
    if (result.has(id)) fail(`duplicate ${label} id`);
    result.add(id);
  });
  return result;
}
function reference(value: unknown, ids: ReadonlySet<string>, label: string): void {
  if (!ids.has(string(value, label))) fail(`unknown ${label}`);
}
function array(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}
function string(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim()) fail(`${label} must be a string`);
  return value;
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function fail(reason: string): never {
  throw new Error(`Invalid Plan workflow data: ${reason}`);
}
