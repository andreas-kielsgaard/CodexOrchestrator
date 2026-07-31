/** Development-only recorded fixture. It never proves execution, persistence, or runtime support. */
import type { RecordedPlanWorkflowV1 } from '../../application/orchestrations/recordedPlanWorkflow';
import type { OrchestrationSectionView } from '../../features/orchestrations';

export const recordedSprintEvaluationProjection = {
  fixtureKind: 'recorded_theoretical',
  executionSupported: false,
} as const;

export const recordedPlanWorkflow: RecordedPlanWorkflowV1 = {
  version: 'plan-workflow/v1',
  workSlicePlanningPointId: 'sprint-planner-activity-recorded-1',
  scopeSummary: 'Recorded Work Unit flow; no execution is performed.',
  fixtureKind: 'recorded_theoretical',
  actors: [
    { id: 'sprint-runner', kind: 'sprint_runner', label: 'Sprint Runner' },
    { id: 'repository', kind: 'repository', label: 'Repository' },
    { id: 'work-slice-planner', kind: 'work_slice_planner', label: 'Work Slice Planner' },
    {
      id: 'work-unit-handler',
      kind: 'work_unit_handler',
      label: 'Work Unit Handler',
      workUnitId: 'work-unit-recorded-1',
    },
    {
      id: 'work-unit-implementer',
      kind: 'work_unit_implementer',
      label: 'Work Unit Implementer',
      workUnitId: 'work-unit-recorded-1',
    },
  ],
  sharedStart: [
    {
      id: 'sprint-started',
      actorId: 'sprint-runner',
      kind: 'sprint_started',
      phase: 'sprint_start',
      title: 'Sprint started',
      summary: 'Recorded Sprint start only.',
    },
    {
      id: 'repository-reevaluated',
      actorId: 'repository',
      kind: 'repository_reevaluated',
      phase: 'repository_assessment',
      title: 'Reality reevaluated',
      summary: 'Recorded branch and repository assessment.',
    },
    {
      id: 'ready-work',
      actorId: 'work-slice-planner',
      kind: 'ready_work_determined',
      phase: 'scope',
      title: 'Ready work planned',
      summary: 'Recorded temporal planning point.',
    },
  ],
  workUnitLanes: [
    {
      id: 'work-unit-recorded-1',
      workUnitId: 'work-unit-recorded-1',
      title: 'Recorded Work Unit',
      handlerActorId: 'work-unit-handler',
      implementerActorId: 'work-unit-implementer',
      steps: [
        {
          id: 'handler-created',
          actorId: 'work-slice-planner',
          kind: 'handler_created',
          phase: 'work_unit_start',
          title: 'Handler created',
          summary: 'Recorded Handler relationship.',
        },
        {
          id: 'implementer-created',
          actorId: 'work-unit-handler',
          kind: 'implementer_created',
          phase: 'implementer_start',
          title: 'Implementer created',
          summary: 'Recorded Implementer relationship.',
        },
        {
          id: 'implementer-return',
          actorId: 'work-unit-implementer',
          kind: 'implementer_return',
          phase: 'first_return',
          title: 'Returned',
          summary: 'Recorded return.',
        },
        {
          id: 'review',
          actorId: 'work-unit-handler',
          kind: 'handler_review',
          phase: 'first_review',
          title: 'Reviewed',
          summary: 'Recorded review.',
        },
        {
          id: 'settled',
          actorId: 'work-unit-handler',
          kind: 'work_unit_settled',
          phase: 'settled',
          title: 'Settled',
          summary: 'Recorded settlement.',
        },
      ],
    },
  ],
  sharedCompletion: [
    {
      id: 'completed',
      actorId: 'work-slice-planner',
      kind: 'work_slice_planner_completed',
      phase: 'planning_complete',
      title: 'Complete',
      summary: 'Recorded completion.',
    },
    {
      id: 'outcome',
      actorId: 'sprint-runner',
      kind: 'sprint_runner_outcome',
      phase: 'sprint_return',
      title: 'Sprint outcome',
      summary: 'Recorded outcome.',
    },
  ],
  interactions: [
    {
      id: 'return-review',
      kind: 'return',
      fromStepId: 'implementer-return',
      toStepId: 'review',
    },
  ],
};

export const disposableRecordedOrchestrationView = {
  epics: [
    {
      id: 'epic-recorded-1',
      name: 'Recorded Epic',
      goal: 'A non-executing recorded Epic for interface development.',
      movement: { kind: 'available', items: [] },
      state: 'paused',
      readyWork: [],
      humanInput: null,
      plan: {
        items: [
          {
            id: 'sprint-recorded-1',
            name: 'Recorded Sprint',
            purpose: 'Recorded-only presentation.',
            status: 'not_started',
          },
        ],
      },
      continuation: { automaticEnabled: false, eligible: false, status: 'not_ready' },
    },
  ],
} satisfies OrchestrationSectionView;
