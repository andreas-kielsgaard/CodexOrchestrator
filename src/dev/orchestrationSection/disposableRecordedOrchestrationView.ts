/** Development-only recorded fixture. It never proves execution, persistence, or runtime support. */
import type { RecordedPlanWorkflowV1 } from '../../application/orchestrations/recordedPlanWorkflow';
import type { OrchestrationSectionView } from '../../features/orchestrations';

export const recordedSprintEvaluationProjection = {
  fixtureKind: 'recorded_theoretical',
  executionSupported: false,
} as const;

export const recordedPlanWorkflow: RecordedPlanWorkflowV1 = {
  version: 'plan-workflow/v1',
  sprintPlannerActivityId: 'sprint-planner-activity-recorded-1',
  scopeSummary: 'Recorded Work Unit flow; no execution is performed.',
  fixtureKind: 'recorded_theoretical',
  actors: [
    { id: 'sprint-planner', kind: 'planner', label: 'Sprint Planner' },
    {
      id: 'work-unit-handler',
      kind: 'work_unit_initiator',
      label: 'Work Unit Handler',
      workUnitId: 'work-unit-recorded-1',
    },
    {
      id: 'work-unit-worker',
      kind: 'worker',
      label: 'Work Unit Worker',
      workUnitId: 'work-unit-recorded-1',
    },
  ],
  sharedStart: [
    {
      id: 'ready',
      actorId: 'sprint-planner',
      kind: 'ready_scope',
      phase: 'ready',
      title: 'Ready',
      summary: 'Recorded scope only.',
    },
  ],
  workUnitLanes: [
    {
      id: 'work-unit-recorded-1',
      workUnitId: 'work-unit-recorded-1',
      title: 'Recorded Work Unit',
      initiatorActorId: 'work-unit-handler',
      workerActorId: 'work-unit-worker',
      steps: [
        {
          id: 'worker-return',
          actorId: 'work-unit-worker',
          kind: 'worker_return',
          phase: 'first_return',
          title: 'Returned',
          summary: 'Recorded return.',
        },
        {
          id: 'review',
          actorId: 'work-unit-handler',
          kind: 'initiator_review',
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
      actorId: 'sprint-planner',
      kind: 'planner_completed',
      phase: 'planner_complete',
      title: 'Complete',
      summary: 'Recorded completion.',
    },
    {
      id: 'outcome',
      actorId: 'sprint-planner',
      kind: 'sprint_outcome',
      phase: 'sprint_return',
      title: 'Sprint outcome',
      summary: 'Recorded outcome.',
    },
  ],
  interactions: [
    { id: 'return-review', kind: 'return', fromStepId: 'worker-return', toStepId: 'review' },
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
