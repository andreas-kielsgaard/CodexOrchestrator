/**
 * Recorded development facts for the product composition boundary. These are application-owned
 * evidence, not a connector, runtime, persistence record, or a projection of the old fixture.
 */
import {
  AGENT_CONTROL_CONTRACTS_V1,
  ARTIFACT_ACCESS_CONTRACTS_V1,
  ORCHESTRATION_EVENTS_V1,
  ORCHESTRATION_ROLE_REPORTS_V1,
  type ProductReadCompositionInputV1,
} from '../../application/orchestrations';

const epicId = 'epic-codex-runner-workspace';
const controlSprintId = 'sprint-control-surface';
const reviewSprintId = 'sprint-parallel-review';
const provenanceId = 'recorded-development';
const ecs2eAcceptanceProvenanceId = 'recorded-ecs2e-accepted-path';
const source = () => ({
  status: 'available' as const,
  sourceKind: 'application_interpretation' as const,
  sourceReferences: [provenanceId],
});
const scoped = (revision: string, unit: string) => `${revision}:${unit}`;

const revisionUnits = {
  'ECS-R1': ['WU-ECS1', 'WU-ECS2', 'WU-ECS3'],
  'ECS-R2': ['WU-ECS1', 'WU-ECS2A', 'WU-ECS2B', 'WU-ECS2C', 'WU-ECS2D', 'WU-ECS3'],
  'ECS-R3': ['WU-ECS1', 'WU-ECS2A', 'WU-ECS2B', 'WU-ECS2C', 'WU-ECS2E', 'WU-ECS2D', 'WU-ECS3'],
  'ECS-R4': ['WU-ECS1', 'WU-ECS2A', 'WU-ECS2B', 'WU-ECS2C', 'WU-ECS2E', 'WU-ECS2D', 'WU-ECS3'],
} as const;
const dependencyIds: Readonly<Record<string, readonly string[]>> = {
  'WU-ECS2A': [],
  'WU-ECS2B': ['WU-ECS2A'],
  'WU-ECS2C': ['WU-ECS2A', 'WU-ECS2B'],
  'WU-ECS2D': ['WU-ECS2A', 'WU-ECS2E'],
  'WU-ECS2E': ['WU-ECS2C'],
  'WU-ECS3': [],
};
const workUnitText: Readonly<Record<string, readonly [string, string, string]>> = {
  'WU-ECS1': [
    'Minimal Sprint surface',
    'Mount the Sprint supervision surface.',
    'Accepted UI discovery Work Unit.',
  ],
  'WU-ECS2': [
    'Bounded refinement and hardening',
    'Original refinement after G1.',
    'Superseded and never launched.',
  ],
  'WU-ECS2A': [
    'Reusable Sprint plan projection',
    'Establish the neutral read boundary.',
    'Accepted after parent review.',
  ],
  'WU-ECS2B': [
    'Shared detail workspace',
    'Add the shared detail workspace.',
    'Accepted shared presentation shell.',
  ],
  'WU-ECS2C': [
    'Plan flow map and revision history',
    'Add revision history and flow.',
    'Accepted corrected semantic substrate.',
  ],
  'WU-ECS2D': [
    'Concerns and Documents',
    'Add concerns and Documents surfaces.',
    'Accepted recorded detail surface.',
  ],
  'WU-ECS2E': [
    'Plan and Work Unit detail surfaces',
    'Add Plan and Work Unit detail surfaces.',
    'Accepted integrated detail surface.',
  ],
  'WU-ECS3': [
    'Consolidation and state-contract handoff',
    'Prepare the state-contract handoff.',
    'Accepted after re-review.',
  ],
};
const executionUnits = [
  'WU-ECS1',
  'WU-ECS2A',
  'WU-ECS2B',
  'WU-ECS2C',
  'WU-ECS2E',
  'WU-ECS2D',
  'WU-ECS3',
];
const reviewWorkUnitText: Readonly<Record<string, readonly [string, string, string]>> = {
  'WU-RD1': [
    'Model review relationships',
    'Represent objectives, Sprint Runner concerns, and graph links.',
    'Recorded completed relationship slice.',
  ],
  'WU-RD2': [
    'Build reusable split surfaces',
    'Create shared horizontal and vertical resizing.',
    'Recorded processing interaction slice.',
  ],
  'WU-RD3': [
    'Connect lifecycle navigation',
    'Connect lifecycle entries to recorded Agent Session turns.',
    'Recorded returned slice awaiting review.',
  ],
  'WU-RD4': [
    'Refine responsive flow',
    'Fit the mixed-state graph at narrow and wide sizes.',
    'Planned after the split surface is ready.',
  ],
  'WU-RD5': [
    'Normalize document review',
    'Open complete Documents with Sprint-start comparison.',
    'Planned after the relationship boundary.',
  ],
  'WU-RD6': [
    'Converge review evidence',
    'Bring later divergent review evidence back into the plan.',
    'Introduced by the second recorded plan revision.',
  ],
};
const reviewRevisionUnits = {
  'RD-R1': ['WU-RD1', 'WU-RD2', 'WU-RD3', 'WU-RD4', 'WU-RD5'],
  'RD-R2': ['WU-RD1', 'WU-RD2', 'WU-RD3', 'WU-RD4', 'WU-RD5', 'WU-RD6'],
} as const;
const reviewDependencies: Readonly<Record<string, readonly string[]>> = {
  'WU-RD1': [],
  'WU-RD2': ['WU-RD1'],
  'WU-RD3': ['WU-RD1'],
  'WU-RD4': ['WU-RD2', 'WU-RD3'],
  'WU-RD5': ['WU-RD1'],
  'WU-RD6': ['WU-RD3', 'WU-RD5'],
};
const reviewExecutionUnits = ['WU-RD1', 'WU-RD2', 'WU-RD3'] as const;

const sprintDefinitions = [
  [
    'sprint-preparation',
    'Preparation Canvas Recovery and Foundation Acceptance',
    'Recover the accepted canvas and establish the neutral Agent Session foundation.',
    'The preparation canvas and reusable Agent Session foundation were accepted.',
    'completed',
  ],
  [
    'sprint-orientation',
    'Orchestration Orientation Discovery',
    'Establish an orchestration-first workspace direction through bounded discovery.',
    'The orchestration overview and contained detail direction were accepted.',
    'completed',
  ],
  [
    controlSprintId,
    'Sprint Control Surface Discovery',
    'Determine the minimum in-app surface needed to understand and supervise one started Sprint.',
    'Recorded development facts only; no runtime work is implied.',
    'completed',
  ],
  [
    reviewSprintId,
    'Sprint and Epic Detail Review',
    'Evaluate the redesigned Sprint context, mixed-state flow, Documents, and Work Unit lifecycle.',
    'Recorded review composition only; it does not represent live orchestration.',
    'in_progress',
  ],
  [
    'sprint-planner-work-unit',
    'Planner and Work Unit Interaction Discovery',
    'Explore planner and Work Unit interactions after the Sprint surface is understood.',
    'This Sprint has not started.',
    'not_started',
  ],
  [
    'sprint-plan-builder',
    'Epic Plan Builder',
    'Explore how a user constructs and revises an epic plan.',
    'This Sprint has not started.',
    'not_started',
  ],
] as const;

const simpleSprints = sprintDefinitions.filter(
  ([id]) => ![controlSprintId, reviewSprintId].includes(id),
);
const simplePlans = simpleSprints.map(([id]) => ({ sprintPlanId: `plan-${id}`, sprintId: id }));
const simpleRevisions = simpleSprints.map(([id]) => ({
  sprintPlanRevisionId: `${id}-r1`,
  sprintPlanId: `plan-${id}`,
  revision: 1,
}));

function managedObjective({
  objectiveId,
  sprintId,
  title,
  concernId,
  workUnitIds,
  handlerRefId,
}: {
  objectiveId: string;
  sprintId: string;
  title: string;
  concernId: string;
  workUnitIds: readonly string[];
  handlerRefId?: string;
}) {
  return {
    objectiveId,
    epicId,
    sprintId,
    title,
    proposalInputProvenanceId: provenanceId,
    sprintRunnerAgentSessionRefId: 'session-ref-parallel-review',
    state: 'concretized' as const,
    oversight: {
      status: 'accepted' as const,
      epicRunnerAgentSessionRefId: 'session-ref-epic-runner',
      provenanceId,
    },
    associations: [
      { kind: 'epic' as const, targetId: epicId },
      { kind: 'sprint' as const, targetId: sprintId },
      { kind: 'concern' as const, targetId: concernId },
      { kind: 'work_slice_planning_point' as const, targetId: 'planner-rd-r2-relationships' },
      ...workUnitIds.map((targetId) => ({ kind: 'work_unit' as const, targetId })),
      ...(handlerRefId ? [{ kind: 'work_unit_handler' as const, targetId: handlerRefId }] : []),
    ],
    source: source(),
  };
}

function proposedObjective({
  objectiveId,
  sprintId,
  title,
  concernId,
}: {
  objectiveId: string;
  sprintId: string;
  title: string;
  concernId: string;
}) {
  return {
    objectiveId,
    epicId,
    sprintId,
    title,
    proposalInputProvenanceId: provenanceId,
    state: 'proposed' as const,
    oversight: { status: 'pending' as const },
    associations: [
      { kind: 'epic' as const, targetId: epicId },
      { kind: 'sprint' as const, targetId: sprintId },
      { kind: 'concern' as const, targetId: concernId },
    ],
    source: source(),
  };
}

/** Complete, ordered canonical input used by the recorded development client. */
export const recordedProductReadCompositionInput = {
  events: {
    version: ORCHESTRATION_EVENTS_V1,
    epics: [{ epicId }],
    sprints: sprintDefinitions.map(([sprintId]) => ({ sprintId, epicId })),
    sprintPlans: [
      ...simplePlans,
      { sprintPlanId: 'plan-control-surface', sprintId: controlSprintId },
      { sprintPlanId: 'plan-parallel-review', sprintId: reviewSprintId },
    ],
    sprintPlanRevisions: [
      ...simpleRevisions,
      { sprintPlanRevisionId: 'ECS-R1', sprintPlanId: 'plan-control-surface', revision: 1 },
      {
        sprintPlanRevisionId: 'ECS-R2',
        sprintPlanId: 'plan-control-surface',
        revision: 2,
        supersedesSprintPlanRevisionId: 'ECS-R1',
      },
      {
        sprintPlanRevisionId: 'ECS-R3',
        sprintPlanId: 'plan-control-surface',
        revision: 3,
        supersedesSprintPlanRevisionId: 'ECS-R2',
      },
      {
        sprintPlanRevisionId: 'ECS-R4',
        sprintPlanId: 'plan-control-surface',
        revision: 4,
        supersedesSprintPlanRevisionId: 'ECS-R3',
      },
      {
        sprintPlanRevisionId: 'RD-R1',
        sprintPlanId: 'plan-parallel-review',
        revision: 1,
      },
      {
        sprintPlanRevisionId: 'RD-R2',
        sprintPlanId: 'plan-parallel-review',
        revision: 2,
        supersedesSprintPlanRevisionId: 'RD-R1',
      },
    ],
    workUnits: [...Object.keys(workUnitText), ...Object.keys(reviewWorkUnitText)].map(
      (workUnitId) => ({ workUnitId }),
    ),
    workUnitScopes: [
      ...Object.entries(revisionUnits).flatMap(([revision, units]) =>
        units.map((workUnitId) => ({
          workUnitScopeId: scoped(revision, workUnitId),
          sprintPlanRevisionId: revision,
          workUnitId,
          dependsOnWorkUnitScopeIds: (dependencyIds[workUnitId] ?? [])
            .filter((dependency) => (units as readonly string[]).includes(dependency))
            .map((dependency) => scoped(revision, dependency)),
          gateIds: revision === 'ECS-R4' && workUnitId === 'WU-ECS3' ? ['G2-R4'] : [],
        })),
      ),
      ...Object.entries(reviewRevisionUnits).flatMap(([revision, units]) =>
        units.map((workUnitId) => ({
          workUnitScopeId: scoped(revision, workUnitId),
          sprintPlanRevisionId: revision,
          workUnitId,
          dependsOnWorkUnitScopeIds: (reviewDependencies[workUnitId] ?? [])
            .filter((dependency) => (units as readonly string[]).includes(dependency))
            .map((dependency) => scoped(revision, dependency)),
          gateIds: [],
        })),
      ),
    ],
    workSlicePlanningPoints: [
      ...(['ECS-R1', 'ECS-R2', 'ECS-R3'] as const).map((revision) => ({
        workSlicePlanningPointId: `planner-${revision.toLowerCase()}`,
        sprintPlanId: 'plan-control-surface',
        assessedSprintPlanRevisionIds: [revision],
      })),
      ...['foundation', 'integration', 'convergence'].map((group) => ({
        workSlicePlanningPointId: `planner-r4-${group}`,
        sprintPlanId: 'plan-control-surface',
        assessedSprintPlanRevisionIds: ['ECS-R4'],
      })),
      {
        workSlicePlanningPointId: 'planner-rd-r1',
        sprintPlanId: 'plan-parallel-review',
        assessedSprintPlanRevisionIds: ['RD-R1'],
      },
      ...['relationships', 'convergence'].map((group) => ({
        workSlicePlanningPointId: `planner-rd-r2-${group}`,
        sprintPlanId: 'plan-parallel-review',
        assessedSprintPlanRevisionIds: ['RD-R2'],
      })),
    ],
    workUnitExecutions: [
      ...executionUnits.map((workUnitId) => ({
        workUnitExecutionId: `execution-${workUnitId}`,
        workUnitId,
        fixedWorkUnitScopeId: scoped('ECS-R4', workUnitId),
      })),
      ...reviewExecutionUnits.map((workUnitId) => ({
        workUnitExecutionId: `execution-${workUnitId}`,
        workUnitId,
        fixedWorkUnitScopeId: scoped('RD-R2', workUnitId),
      })),
    ],
    attempts: [
      ...executionUnits
        .filter((workUnitId) => workUnitId !== 'WU-ECS2E')
        .map((workUnitId) => ({
          attemptId: `attempt-${workUnitId}`,
          workUnitExecutionId: `execution-${workUnitId}`,
          fixedWorkUnitScopeId: scoped('ECS-R4', workUnitId),
        })),
      {
        attemptId: 'WU-ECS2E-attempt-1',
        workUnitExecutionId: 'execution-WU-ECS2E',
        fixedWorkUnitScopeId: scoped('ECS-R4', 'WU-ECS2E'),
      },
      {
        attemptId: 'WU-ECS2E-attempt-2',
        workUnitExecutionId: 'execution-WU-ECS2E',
        fixedWorkUnitScopeId: scoped('ECS-R4', 'WU-ECS2E'),
      },
      {
        attemptId: 'WU-RD1-attempt-1',
        workUnitExecutionId: 'execution-WU-RD1',
        fixedWorkUnitScopeId: scoped('RD-R2', 'WU-RD1'),
      },
      {
        attemptId: 'WU-RD1-attempt-2',
        workUnitExecutionId: 'execution-WU-RD1',
        fixedWorkUnitScopeId: scoped('RD-R2', 'WU-RD1'),
      },
      ...(['WU-RD2', 'WU-RD3'] as const).map((workUnitId) => ({
        attemptId: `attempt-${workUnitId}`,
        workUnitExecutionId: `execution-${workUnitId}`,
        fixedWorkUnitScopeId: scoped('RD-R2', workUnitId),
      })),
    ],
    agentSessions: [
      { agentSessionId: 'recorded-epic-runner-manual-continuation-ready' },
      { agentSessionId: 'recorded-sprint-control-surface-discovery' },
      { agentSessionId: 'recorded-session-planner-r4-integration' },
      ...executionUnits.map((workUnitId) => ({ agentSessionId: `recorded-session-${workUnitId}` })),
      { agentSessionId: 'recorded-implementer-WU-ECS2E' },
      { agentSessionId: 'recorded-sprint-parallel-review' },
      { agentSessionId: 'recorded-planner-rd-r2' },
      { agentSessionId: 'recorded-handler-WU-RD1' },
      { agentSessionId: 'recorded-implementer-WU-RD1' },
      ...(['WU-RD2', 'WU-RD3'] as const).map((workUnitId) => ({
        agentSessionId: `recorded-implementer-${workUnitId}`,
      })),
    ],
    agentSessionReferences: [
      {
        agentSessionRefId: 'session-ref-epic-runner',
        agentSessionId: 'recorded-epic-runner-manual-continuation-ready',
        targetKind: 'epic',
        targetId: epicId,
        semanticRole: 'epic_runner',
      },
      {
        agentSessionRefId: 'session-ref-sprint',
        agentSessionId: 'recorded-sprint-control-surface-discovery',
        targetKind: 'sprint',
        targetId: controlSprintId,
        semanticRole: 'sprint_runner',
      },
      {
        agentSessionRefId: 'session-ref-planner-r4-integration',
        agentSessionId: 'recorded-session-planner-r4-integration',
        targetKind: 'work_slice_planning_point',
        targetId: 'planner-r4-integration',
        semanticRole: 'work_slice_planner',
      },
      ...executionUnits.map((workUnitId) => ({
        agentSessionRefId: `session-ref-${workUnitId}`,
        agentSessionId: `recorded-session-${workUnitId}`,
        targetKind: 'work_unit_execution',
        targetId: `execution-${workUnitId}`,
        semanticRole:
          workUnitId === 'WU-ECS2E'
            ? ('work_unit_handler' as const)
            : ('work_unit_implementer' as const),
      })),
      {
        agentSessionRefId: 'session-ref-implementer-WU-ECS2E',
        agentSessionId: 'recorded-implementer-WU-ECS2E',
        targetKind: 'work_unit_execution',
        targetId: 'execution-WU-ECS2E',
        semanticRole: 'work_unit_implementer',
      },
      {
        agentSessionRefId: 'session-ref-parallel-review',
        agentSessionId: 'recorded-sprint-parallel-review',
        targetKind: 'sprint',
        targetId: reviewSprintId,
        semanticRole: 'sprint_runner',
      },
      {
        agentSessionRefId: 'session-ref-planner-rd-r2',
        agentSessionId: 'recorded-planner-rd-r2',
        targetKind: 'work_slice_planning_point',
        targetId: 'planner-rd-r2-relationships',
        semanticRole: 'work_slice_planner',
      },
      {
        agentSessionRefId: 'session-ref-handler-WU-RD1',
        agentSessionId: 'recorded-handler-WU-RD1',
        targetKind: 'work_unit_execution',
        targetId: 'execution-WU-RD1',
        semanticRole: 'work_unit_handler',
      },
      {
        agentSessionRefId: 'session-ref-implementer-WU-RD1',
        agentSessionId: 'recorded-implementer-WU-RD1',
        targetKind: 'work_unit_execution',
        targetId: 'execution-WU-RD1',
        semanticRole: 'work_unit_implementer',
      },
      ...(['WU-RD2', 'WU-RD3'] as const).map((workUnitId) => ({
        agentSessionRefId: `session-ref-implementer-${workUnitId}`,
        agentSessionId: `recorded-implementer-${workUnitId}`,
        targetKind: 'work_unit_execution' as const,
        targetId: `execution-${workUnitId}`,
        semanticRole: 'work_unit_implementer' as const,
      })),
    ],
    gates: ['ECS-R1', 'ECS-R2', 'ECS-R3', 'ECS-R4'].map((revision) => ({
      gateId: `G2-${revision.slice(-2)}`,
      sprintPlanRevisionId: revision,
    })),
    gateCriteriaRevisions: ['ECS-R1', 'ECS-R2', 'ECS-R3', 'ECS-R4'].map((revision) => ({
      gateCriteriaRevisionId: `criteria-${revision}`,
      gateId: `G2-${revision.slice(-2)}`,
      revision: 1,
    })),
    feedbackRecords: [],
    policyEligibilityFacts: [],
    executionRequests: [...executionUnits, ...reviewExecutionUnits].map((workUnitId) => ({
      executionRequestId: `request-${workUnitId}`,
      workUnitExecutionId: `execution-${workUnitId}`,
      provenanceId,
    })),
    observedLaunches: [
      ...executionUnits
        .filter((workUnitId) => workUnitId !== 'WU-ECS2E')
        .map((workUnitId) => ({
          observedLaunchId: `launch-${workUnitId}`,
          executionRequestId: `request-${workUnitId}`,
          workUnitExecutionId: `execution-${workUnitId}`,
          attemptId: `attempt-${workUnitId}`,
          provenanceId,
        })),
      {
        observedLaunchId: 'launch-WU-ECS2E-attempt-1',
        executionRequestId: 'request-WU-ECS2E',
        workUnitExecutionId: 'execution-WU-ECS2E',
        attemptId: 'WU-ECS2E-attempt-1',
        provenanceId,
      },
      {
        observedLaunchId: 'launch-WU-ECS2E-attempt-2',
        executionRequestId: 'request-WU-ECS2E',
        workUnitExecutionId: 'execution-WU-ECS2E',
        attemptId: 'WU-ECS2E-attempt-2',
        provenanceId,
      },
      {
        observedLaunchId: 'launch-WU-RD1-attempt-1',
        executionRequestId: 'request-WU-RD1',
        workUnitExecutionId: 'execution-WU-RD1',
        attemptId: 'WU-RD1-attempt-1',
        provenanceId,
      },
      {
        observedLaunchId: 'launch-WU-RD1-attempt-2',
        executionRequestId: 'request-WU-RD1',
        workUnitExecutionId: 'execution-WU-RD1',
        attemptId: 'WU-RD1-attempt-2',
        provenanceId,
      },
      ...(['WU-RD2', 'WU-RD3'] as const).map((workUnitId) => ({
        observedLaunchId: `launch-${workUnitId}`,
        executionRequestId: `request-${workUnitId}`,
        workUnitExecutionId: `execution-${workUnitId}`,
        attemptId: `attempt-${workUnitId}`,
        provenanceId,
      })),
    ],
    observedReturns: [
      ...executionUnits
        .filter((workUnitId) => workUnitId !== 'WU-ECS2E')
        .map((workUnitId) => ({
          observedReturnId: `return-${workUnitId}`,
          observedLaunchId: `launch-${workUnitId}`,
          attemptId: `attempt-${workUnitId}`,
          provenanceId,
        })),
      {
        observedReturnId: 'return-WU-ECS2E-attempt-1',
        observedLaunchId: 'launch-WU-ECS2E-attempt-1',
        attemptId: 'WU-ECS2E-attempt-1',
        provenanceId,
      },
      {
        observedReturnId: 'return-WU-ECS2E-attempt-2',
        observedLaunchId: 'launch-WU-ECS2E-attempt-2',
        attemptId: 'WU-ECS2E-attempt-2',
        provenanceId,
      },
      {
        observedReturnId: 'return-WU-RD1-attempt-1',
        observedLaunchId: 'launch-WU-RD1-attempt-1',
        attemptId: 'WU-RD1-attempt-1',
        provenanceId,
      },
      {
        observedReturnId: 'return-WU-RD1-attempt-2',
        observedLaunchId: 'launch-WU-RD1-attempt-2',
        attemptId: 'WU-RD1-attempt-2',
        provenanceId,
      },
      {
        observedReturnId: 'return-WU-RD3',
        observedLaunchId: 'launch-WU-RD3',
        attemptId: 'attempt-WU-RD3',
        provenanceId,
      },
    ],
    reviews: [
      ...executionUnits
        .filter((workUnitId) => workUnitId !== 'WU-ECS2E')
        .map((workUnitId) => ({
          reviewId: `review-${workUnitId}`,
          subjectKind: 'work_unit_execution',
          subjectId: `execution-${workUnitId}`,
          outcome: 'accepted',
          provenanceId,
        })),
      {
        reviewId: 'review-WU-ECS2E-attempt-1',
        subjectKind: 'attempt',
        subjectId: 'WU-ECS2E-attempt-1',
        outcome: 'needs_correction',
        provenanceId,
      },
      {
        reviewId: 'review-WU-ECS2E-attempt-2',
        subjectKind: 'attempt',
        subjectId: 'WU-ECS2E-attempt-2',
        outcome: 'accepted',
        provenanceId,
      },
      {
        reviewId: 'review-WU-RD1-attempt-1',
        subjectKind: 'attempt',
        subjectId: 'WU-RD1-attempt-1',
        outcome: 'needs_correction',
        provenanceId,
      },
      {
        reviewId: 'review-WU-RD1-attempt-2',
        subjectKind: 'attempt',
        subjectId: 'WU-RD1-attempt-2',
        outcome: 'accepted',
        provenanceId,
      },
    ],
    observedIntegrations: [
      ...executionUnits
        .filter((workUnitId) => workUnitId !== 'WU-ECS2E')
        .map((workUnitId) => ({
          observedIntegrationId: `integration-${workUnitId}`,
          workUnitExecutionId: `execution-${workUnitId}`,
          provenanceId,
        })),
      {
        observedIntegrationId: 'integration-WU-ECS2E',
        workUnitExecutionId: 'execution-WU-ECS2E',
        provenanceId: ecs2eAcceptanceProvenanceId,
      },
      {
        observedIntegrationId: 'integration-WU-RD1',
        workUnitExecutionId: 'execution-WU-RD1',
        provenanceId,
      },
    ],
    observedCompletions: [
      ...executionUnits
        .filter((workUnitId) => workUnitId !== 'WU-ECS2E')
        .map((workUnitId) => ({
          observedCompletionId: `completion-${workUnitId}`,
          subjectKind: 'work_unit_execution',
          subjectId: `execution-${workUnitId}`,
          responsibilityAccepted: true,
          provenanceId,
        })),
      {
        observedCompletionId: 'completion-WU-ECS2E',
        subjectKind: 'work_unit_execution',
        subjectId: 'execution-WU-ECS2E',
        responsibilityAccepted: true,
        provenanceId: ecs2eAcceptanceProvenanceId,
      },
      {
        observedCompletionId: 'completion-sprint-control-surface',
        subjectKind: 'sprint',
        subjectId: controlSprintId,
        responsibilityAccepted: true,
        provenanceId,
      },
      {
        observedCompletionId: 'completion-WU-RD1',
        subjectKind: 'work_unit_execution',
        subjectId: 'execution-WU-RD1',
        responsibilityAccepted: true,
        provenanceId,
      },
    ],
    continuationRequests: [],
    observedContinuations: [],
    observedHandoffs: [],
    internalArtifacts: [
      { artifactId: 'artifact-ecs-r1', provenanceId },
      { artifactId: 'artifact-g1', provenanceId },
      { artifactId: 'artifact-ecs2e-review', provenanceId },
      { artifactId: 'artifact-file-review', provenanceId },
      { artifactId: 'artifact-rd-review', provenanceId },
    ],
    documentReferences: [
      { documentRefId: 'doc-ecs-r1', artifactIds: ['artifact-ecs-r1'], provenanceId },
      { documentRefId: 'doc-g1', artifactIds: ['artifact-g1'], provenanceId },
      { documentRefId: 'doc-ecs2e-review', artifactIds: ['artifact-ecs2e-review'], provenanceId },
      { documentRefId: 'doc-file-review', artifactIds: ['artifact-file-review'], provenanceId },
      { documentRefId: 'doc-rd-review', artifactIds: ['artifact-rd-review'], provenanceId },
    ],
    provenance: [
      {
        provenanceId,
        sourceKind: 'application',
        recordedAt: '2026-07-15T09:00:00.000Z',
        causalFactIds: [],
      },
      {
        provenanceId: ecs2eAcceptanceProvenanceId,
        sourceKind: 'application',
        recordedAt: '2026-07-15T09:31:00.000Z',
        causalFactIds: ['review-WU-ECS2E-attempt-2'],
      },
    ],
  },
  agentControl: {
    version: AGENT_CONTROL_CONTRACTS_V1,
    promptProvenance: [],
    continuationPolicies: [
      {
        continuationPolicyId: 'recorded-sprint-policy',
        level: 'sprint',
        sprintId: controlSprintId,
        autoFlowEnabled: false,
      },
      {
        continuationPolicyId: 'recorded-epic-policy',
        level: 'epic',
        epicId,
        autoFlowEnabled: false,
      },
    ],
    continuationEligibilityEvaluations: [],
    commands: [],
    results: [],
  },
  artifactAccess: {
    version: ARTIFACT_ACCESS_CONTRACTS_V1,
    artifacts: [
      {
        artifactId: 'artifact-ecs-r1',
        kind: 'epic_plan',
        provenanceReference: provenanceId,
      },
      { artifactId: 'artifact-g1', kind: 'review_material', provenanceReference: provenanceId },
      {
        artifactId: 'artifact-ecs2e-review',
        kind: 'review_material',
        provenanceReference: provenanceId,
      },
      {
        artifactId: 'artifact-file-review',
        kind: 'changed_file_manifest',
        provenanceReference: provenanceId,
      },
      {
        artifactId: 'artifact-rd-review',
        kind: 'review_material',
        provenanceReference: provenanceId,
      },
    ],
    changedFileReferences: [
      {
        changedFileReferenceId: 'document-ecs-r1',
        displayName: 'documents/original-ecs-r1-plan.md',
        changeKind: 'modified',
      },
      {
        changedFileReferenceId: 'document-g1',
        displayName: 'documents/g1-feedback.md',
        changeKind: 'modified',
      },
      {
        changedFileReferenceId: 'document-ecs2e-review',
        displayName: 'documents/ecs2e-review.md',
        changeKind: 'modified',
      },
      {
        changedFileReferenceId: 'changed-file-review-doc',
        displayName: 'docs/orchestration/file-diff-viewer-exploration.md',
        changeKind: 'modified',
      },
      {
        changedFileReferenceId: 'document-rd-review',
        displayName: 'documents/sprint-detail-review-evidence.md',
        changeKind: 'modified',
      },
    ],
    documents: [
      {
        documentRefId: 'doc-ecs-r1',
        classification: 'other',
        title: 'Original ECS-R1 plan',
        summary: 'Recorded original plan.',
        artifactIds: ['artifact-ecs-r1'],
        changedFileReferenceIds: ['document-ecs-r1'],
        provenanceReference: provenanceId,
      },
      {
        documentRefId: 'doc-g1',
        classification: 'review_material',
        title: 'G1 feedback and ECS-R2 replan',
        summary: 'Recorded G1 feedback.',
        artifactIds: ['artifact-g1'],
        changedFileReferenceIds: ['document-g1'],
        provenanceReference: provenanceId,
      },
      {
        documentRefId: 'doc-ecs2e-review',
        classification: 'review_material',
        title: 'WU-ECS2E corrected visual review',
        summary: 'Recorded accepted review.',
        artifactIds: ['artifact-ecs2e-review'],
        changedFileReferenceIds: ['document-ecs2e-review'],
        provenanceReference: provenanceId,
      },
      {
        documentRefId: 'doc-file-review',
        classification: 'changed_files',
        title: 'Application-owned file review',
        summary: 'Recorded application-owned review material',
        artifactIds: ['artifact-file-review'],
        changedFileReferenceIds: ['changed-file-review-doc'],
        provenanceReference: provenanceId,
      },
      {
        documentRefId: 'doc-rd-review',
        classification: 'review_material',
        title: 'Sprint detail review evidence',
        summary: 'Recorded evidence for the in-progress Sprint review composition.',
        artifactIds: ['artifact-rd-review'],
        changedFileReferenceIds: ['document-rd-review'],
        provenanceReference: provenanceId,
      },
    ],
    requests: [],
    results: [],
  },
  roleReports: {
    version: ORCHESTRATION_ROLE_REPORTS_V1,
    reports: [
      {
        reportId: 'report-rd-r2-sprint-plan',
        toolName: 'record_sprint_plan',
        agentRole: 'sprint_runner',
        agentSessionRefId: 'session-ref-parallel-review',
        sprintId: reviewSprintId,
        sprintPlanRevisionId: 'RD-R2',
        managedObjectiveIds: [
          'objective-review-relationships',
          'objective-review-flow',
          'objective-review-evidence',
        ],
        concernIds: [
          'concern-review-relationships',
          'concern-review-lifecycle',
          'concern-review-documents',
        ],
        refinementSummary:
          'The Sprint Runner concretized the Epic proposal against the recorded repository assessment.',
        provenanceId,
      },
      {
        reportId: 'report-rd-r2-epic-oversight',
        toolName: 'record_sprint_oversight',
        agentRole: 'epic_runner',
        agentSessionRefId: 'session-ref-epic-runner',
        sprintId: reviewSprintId,
        sprintPlanRevisionId: 'RD-R2',
        decision: 'accepted',
        summary: 'The Epic Runner accepted the concretized Sprint plan for recorded review.',
        provenanceId,
      },
      {
        reportId: 'report-rd-r2-relationships-plan',
        toolName: 'record_work_slice_plan',
        agentRole: 'work_slice_planner',
        agentSessionRefId: 'session-ref-planner-rd-r2',
        workSlicePlanningPointId: 'planner-rd-r2-relationships',
        sprintPlanRevisionId: 'RD-R2',
        analysisItems: [
          {
            analysisItemId: 'analysis-rd-contracts',
            text: 'Stabilize typed relationships before lifecycle and document convergence.',
            linkedWorkUnitScopeIds: ['WU-RD1', 'WU-RD3', 'WU-RD5'].map((unit) =>
              scoped('RD-R2', unit),
            ),
          },
          {
            analysisItemId: 'analysis-rd-parallel',
            text: 'Run split-surface work in parallel with the relationship foundation.',
            linkedWorkUnitScopeIds: ['WU-RD1', 'WU-RD2'].map((unit) => scoped('RD-R2', unit)),
          },
          {
            analysisItemId: 'analysis-rd-join',
            text: 'Join independently completed prerequisites without implying a merged output where none exists.',
            linkedWorkUnitScopeIds: ['WU-RD2', 'WU-RD3', 'WU-RD4'].map((unit) =>
              scoped('RD-R2', unit),
            ),
          },
        ],
        workUnitScopeIds: ['WU-RD1', 'WU-RD2', 'WU-RD3', 'WU-RD4', 'WU-RD5'].map((unit) =>
          scoped('RD-R2', unit),
        ),
        dependencies: [
          {
            dependencyId: 'dependency-rd1-rd3-output',
            fromWorkUnitScopeId: scoped('RD-R2', 'WU-RD1'),
            toWorkUnitScopeId: scoped('RD-R2', 'WU-RD3'),
            kind: 'functional_output',
            label: 'typed lifecycle contract',
          },
          {
            dependencyId: 'dependency-rd1-rd2-workspace',
            fromWorkUnitScopeId: scoped('RD-R2', 'WU-RD1'),
            toWorkUnitScopeId: scoped('RD-R2', 'WU-RD2'),
            kind: 'shared_resource_exclusion',
            label: 'shared detail files',
            sharedResourceKey: 'sprint-detail-workspace',
          },
          {
            dependencyId: 'dependency-rd2-rd3-rd4-prerequisites',
            inputWorkUnitScopeIds: ['WU-RD2', 'WU-RD3'].map((unit) => scoped('RD-R2', unit)),
            toWorkUnitScopeId: scoped('RD-R2', 'WU-RD4'),
            kind: 'merge_join',
            label: 'responsive prerequisites',
            joinSemantics: 'independent_prerequisites',
          },
          {
            dependencyId: 'dependency-rd1-rd5-merge',
            fromWorkUnitScopeId: scoped('RD-R2', 'WU-RD1'),
            toWorkUnitScopeId: scoped('RD-R2', 'WU-RD5'),
            kind: 'merge_join',
            label: 'merged relationship result',
            joinSemantics: 'merged_result',
          },
        ],
        provenanceId,
      },
      {
        reportId: 'report-rd1-handler-current',
        toolName: 'report_handler_activity',
        agentRole: 'work_unit_handler',
        agentSessionRefId: 'session-ref-handler-WU-RD1',
        workUnitExecutionId: 'execution-WU-RD1',
        activity: 'approved',
        summary: 'Approved and integrated the corrected relationship model.',
        lifecycleEntryId: 'rd1-completion',
        provenanceId,
      },
      {
        reportId: 'report-rd1-worker-current',
        toolName: 'report_worker_activity',
        agentRole: 'work_unit_implementer',
        agentSessionRefId: 'session-ref-implementer-WU-RD1',
        workUnitExecutionId: 'execution-WU-RD1',
        activity: 'completed',
        summary: 'Completed the corrected typed relationship implementation.',
        outcome: 'Accepted relationship model',
        lifecycleEntryId: 'rd1-renewed-work',
        provenanceId,
      },
      {
        reportId: 'report-rd2-worker-current',
        toolName: 'report_worker_activity',
        agentRole: 'work_unit_implementer',
        agentSessionRefId: 'session-ref-implementer-WU-RD2',
        workUnitExecutionId: 'execution-WU-RD2',
        activity: 'implementing',
        summary: 'Implementing the draggable split surfaces.',
        lifecycleEntryId: 'rd2-work',
        provenanceId,
      },
      {
        reportId: 'report-rd3-worker-current',
        toolName: 'report_worker_activity',
        agentRole: 'work_unit_implementer',
        agentSessionRefId: 'session-ref-implementer-WU-RD3',
        workUnitExecutionId: 'execution-WU-RD3',
        activity: 'returned',
        summary: 'Returned lifecycle navigation for Handler review.',
        outcome: 'Awaiting typed review',
        lifecycleEntryId: 'rd3-work',
        provenanceId,
      },
    ],
  },
  referenceIndex: {
    epics: [
      {
        epicId,
        title: 'Codex Epic Runner workspace development',
        goal: 'Develop Codex Epic Runner from a neutral Agent Session foundation into an orchestration-first workspace.',
        source: source(),
      },
    ],
    epicOverviews: [
      {
        epicId,
        currentMovement: {
          source: source(),
          value: { kind: 'executing_work', processingCount: 0, reviewingCount: 0 },
        },
        state: { source: source(), value: 'ready_to_continue' },
      },
    ],
    sprints: sprintDefinitions.map(([sprintId, title, summary, details, lifecycle]) => ({
      sprintId,
      title,
      summary,
      details,
      source: source(),
      lifecycle: { source: source(), value: lifecycle },
      planningState:
        lifecycle === 'not_started'
          ? { source: source(), value: { kind: 'pre_start_forecast' as const } }
          : sprintId === controlSprintId || sprintId === reviewSprintId
            ? {
                source: source(),
                value: {
                  kind: 'started_plan' as const,
                  currentWorkSlicePlanningPointId:
                    sprintId === controlSprintId
                      ? 'planner-r4-integration'
                      : 'planner-rd-r2-relationships',
                  repositoryAssessmentSummary:
                    'Recorded branch and repository state was reevaluated at Sprint start.',
                  reevaluatedAt: '2026-07-15T09:00:00.000Z',
                },
              }
            : {
                source: {
                  status: 'unavailable' as const,
                  reason: 'No recorded Sprint-start repository assessment is available.',
                },
              },
    })),
    sprintPlanRevisions: [
      ...simpleRevisions.map(({ sprintPlanRevisionId }) => ({
        sprintPlanRevisionId,
        summary: 'Recorded initial plan',
        source: source(),
      })),
      ...['ECS-R1', 'ECS-R2', 'ECS-R3', 'ECS-R4'].map((sprintPlanRevisionId) => ({
        sprintPlanRevisionId,
        summary: (
          {
            'ECS-R1': 'Original Sprint surface discovery plan.',
            'ECS-R2': 'Accepted G1 replan.',
            'ECS-R3': 'Conversation-driven overview correction.',
            'ECS-R4': 'Integrated Plan ownership and detail evaluation.',
          } as Record<string, string>
        )[sprintPlanRevisionId],
        source: source(),
      })),
      {
        sprintPlanRevisionId: 'RD-R1',
        summary: 'Initial detail review forecast.',
        source: source(),
      },
      {
        sprintPlanRevisionId: 'RD-R2',
        summary: 'Parallel review plan with later convergence evidence.',
        source: source(),
      },
    ],
    workSlicePlanningPoints: [
      ...(['ECS-R1', 'ECS-R2', 'ECS-R3'] as const).map((revision) => ({
        workSlicePlanningPointId: `planner-${revision.toLowerCase()}`,
        title: `Planning point ${revision}`,
        purpose: 'Recorded Plan assessment.',
        source: source(),
      })),
      {
        workSlicePlanningPointId: 'planner-r4-foundation',
        title: 'Foundation and correction',
        purpose: 'Accepted semantic and layout substrate.',
        source: source(),
      },
      {
        workSlicePlanningPointId: 'planner-r4-integration',
        title: 'Integrated detail surfaces',
        purpose: 'Own the recorded integrated detail surfaces.',
        source: source(),
      },
      {
        workSlicePlanningPointId: 'planner-r4-convergence',
        title: 'Convergence handoff',
        purpose: 'Hold consolidation until review acceptance.',
        source: source(),
      },
      {
        workSlicePlanningPointId: 'planner-rd-r1',
        title: 'Initial review forecast',
        purpose: 'Forecast the first relationship and interaction slices.',
        source: source(),
      },
      {
        workSlicePlanningPointId: 'planner-rd-r2-relationships',
        title: 'Relationship foundation',
        purpose:
          'Run the typed relationship, interaction, lifecycle, and document lanes in parallel.',
        source: source(),
      },
      {
        workSlicePlanningPointId: 'planner-rd-r2-convergence',
        title: 'Later convergence',
        purpose: 'Converge the later responsive and evidence work.',
        source: source(),
      },
    ],
    workUnits: [...Object.entries(workUnitText), ...Object.entries(reviewWorkUnitText)].map(
      ([workUnitId, [title, summary, details]]) => ({
        workUnitId,
        title,
        summary,
        details,
        source: source(),
      }),
    ),
    gates: ['ECS-R1', 'ECS-R2', 'ECS-R3', 'ECS-R4'].map((revision) => ({
      gateId: `G2-${revision.slice(-2)}`,
      title: 'G2',
      summary: 'Recorded user evaluation gate.',
      source: source(),
    })),
    concerns: [
      {
        concernId: 'concern-sprint-surface',
        sprintId: controlSprintId,
        title: 'Sprint control surface',
        summary: 'A reusable semantic view of one started Sprint.',
        details: 'Complete after the recorded required Work Units were accepted.',
        requiredWorkUnitIds: ['WU-ECS1', 'WU-ECS2A', 'WU-ECS2B'],
        stateAuthority: { kind: 'derived_from_required_work_units' },
        source: source(),
      },
      {
        concernId: 'concern-sprint-flow',
        sprintId: controlSprintId,
        title: 'Flow and detail navigation',
        summary: 'Plan history and Work Unit detail stay semantically connected.',
        details: 'Recorded against the flow and detail Work Units.',
        requiredWorkUnitIds: ['WU-ECS2C', 'WU-ECS2E'],
        stateAuthority: { kind: 'derived_from_required_work_units' },
        source: source(),
      },
      {
        concernId: 'concern-sprint-records',
        sprintId: controlSprintId,
        title: 'Concern and Document records',
        summary: 'Recorded concern and Document surfaces remain truthful.',
        details: 'Recorded against the information and handoff Work Units.',
        requiredWorkUnitIds: ['WU-ECS2D', 'WU-ECS3'],
        stateAuthority: { kind: 'derived_from_required_work_units' },
        source: source(),
      },
      {
        concernId: 'concern-review-relationships',
        sprintId: reviewSprintId,
        title: 'Context and relationship clarity',
        summary: 'Objectives and Sprint Runner concerns remain visible and linked.',
        details: 'Requires the relationship model and split-surface work.',
        requiredWorkUnitIds: ['WU-RD1', 'WU-RD2'],
        stateAuthority: { kind: 'derived_from_required_work_units' },
        source: source(),
      },
      {
        concernId: 'concern-review-lifecycle',
        sprintId: reviewSprintId,
        title: 'Lifecycle and responsive interaction',
        summary: 'Lifecycle turns and the responsive graph remain reviewable.',
        details: 'Requires lifecycle navigation and responsive refinement.',
        requiredWorkUnitIds: ['WU-RD3', 'WU-RD4'],
        stateAuthority: { kind: 'derived_from_required_work_units' },
        source: source(),
      },
      {
        concernId: 'concern-review-documents',
        sprintId: reviewSprintId,
        title: 'Document and convergence evidence',
        summary: 'Documents open completely and later evidence remains explicit.',
        details: 'Requires Document normalization and convergence evidence.',
        requiredWorkUnitIds: ['WU-RD5', 'WU-RD6'],
        stateAuthority: { kind: 'derived_from_required_work_units' },
        source: source(),
      },
      {
        concernId: 'concern-planner-interaction-forecast',
        sprintId: 'sprint-planner-work-unit',
        title: 'Planning and parallel implementation relationships',
        summary: 'Forecast how temporal planning may organize parallel implementation concerns.',
        details: 'Pre-start concern only; no Work Units or dependencies are yet planned facts.',
        requiredWorkUnitIds: [],
        stateAuthority: { kind: 'derived_from_required_work_units' },
        source: source(),
      },
      {
        concernId: 'concern-plan-builder-forecast',
        sprintId: 'sprint-plan-builder',
        title: 'Epic planning interaction',
        summary: 'Forecast the concern of constructing and revising an Epic plan.',
        details: 'Pre-start concern only; no Work Units or dependencies are yet planned facts.',
        requiredWorkUnitIds: [],
        stateAuthority: { kind: 'derived_from_required_work_units' },
        source: source(),
      },
    ],
    agentSessions: [
      {
        agentSessionId: 'recorded-epic-runner-manual-continuation-ready',
        title: 'Orientation discovery handler',
        source: source(),
      },
      {
        agentSessionId: 'recorded-sprint-control-surface-discovery',
        title: 'Sprint control surface discovery',
        source: source(),
      },
      {
        agentSessionId: 'recorded-session-planner-r4-integration',
        title: 'Recorded planner R4 integration',
        source: source(),
      },
      ...executionUnits.map((workUnitId) => ({
        agentSessionId: `recorded-session-${workUnitId}`,
        title:
          workUnitId === 'WU-ECS2E'
            ? 'Recorded WU-ECS2E Work Unit Handler'
            : `${workUnitText[workUnitId][0]} Work Unit Implementer`,
        source: source(),
      })),
      {
        agentSessionId: 'recorded-implementer-WU-ECS2E',
        title: 'Recorded WU-ECS2E Work Unit Implementer',
        source: source(),
      },
      {
        agentSessionId: 'recorded-sprint-parallel-review',
        title: 'Recorded parallel review Sprint',
        identity: {
          agentName: 'Soren',
          visualIdentity: { token: 'SR', accentColor: '#376f8d' },
        },
        source: source(),
      },
      {
        agentSessionId: 'recorded-planner-rd-r2',
        title: 'Recorded review Work Slice Planner',
        identity: {
          agentName: 'Priya',
          visualIdentity: { token: 'P', accentColor: '#7562a9' },
        },
        source: source(),
      },
      {
        agentSessionId: 'recorded-handler-WU-RD1',
        title: 'Relationship Work Unit Handler',
        identity: {
          agentName: 'Rowan',
          visualIdentity: { token: 'R', accentColor: '#9a5e35' },
        },
        source: source(),
      },
      {
        agentSessionId: 'recorded-implementer-WU-RD1',
        title: 'Relationship Work Unit Implementer',
        identity: {
          agentName: 'Mina',
          visualIdentity: { token: 'M', accentColor: '#2d8067' },
        },
        source: source(),
      },
      ...(['WU-RD2', 'WU-RD3'] as const).map((workUnitId, index) => ({
        agentSessionId: `recorded-implementer-${workUnitId}`,
        title: `${reviewWorkUnitText[workUnitId][0]} Work Unit Implementer`,
        identity: {
          agentName: index === 0 ? 'Theo' : 'June',
          visualIdentity: {
            token: index === 0 ? 'T' : 'J',
            accentColor: index === 0 ? '#3b719f' : '#8a5e9d',
          },
        },
        source: source(),
      })),
    ],
    artifactOwnership: [
      { artifactId: 'artifact-ecs-r1', sprintId: controlSprintId, source: source() },
      { artifactId: 'artifact-g1', sprintId: controlSprintId, source: source() },
      { artifactId: 'artifact-ecs2e-review', sprintId: controlSprintId, source: source() },
      { artifactId: 'artifact-file-review', sprintId: controlSprintId, source: source() },
      { artifactId: 'artifact-rd-review', sprintId: reviewSprintId, source: source() },
    ],
    documentOwnership: [
      { documentRefId: 'doc-ecs-r1', sprintId: controlSprintId, source: source() },
      { documentRefId: 'doc-g1', sprintId: controlSprintId, source: source() },
      { documentRefId: 'doc-ecs2e-review', sprintId: controlSprintId, source: source() },
      { documentRefId: 'doc-file-review', sprintId: controlSprintId, source: source() },
      { documentRefId: 'doc-rd-review', sprintId: reviewSprintId, source: source() },
    ],
    sprintWorkspacePresentation: {
      workSlicePlanningPointMembership: [
        ...(['ECS-R1', 'ECS-R2', 'ECS-R3'] as const).map((revision) => ({
          workSlicePlanningPointId: `planner-${revision.toLowerCase()}`,
          sprintPlanRevisionId: revision,
          workUnitScopeIds: revisionUnits[revision].map((unit) => scoped(revision, unit)),
          source: source(),
        })),
        {
          workSlicePlanningPointId: 'planner-r4-foundation',
          sprintPlanRevisionId: 'ECS-R4',
          workUnitScopeIds: ['WU-ECS1', 'WU-ECS2A', 'WU-ECS2B', 'WU-ECS2C'].map((unit) =>
            scoped('ECS-R4', unit),
          ),
          source: source(),
        },
        {
          workSlicePlanningPointId: 'planner-r4-integration',
          sprintPlanRevisionId: 'ECS-R4',
          workUnitScopeIds: ['WU-ECS2E', 'WU-ECS2D'].map((unit) => scoped('ECS-R4', unit)),
          source: source(),
        },
        {
          workSlicePlanningPointId: 'planner-r4-convergence',
          sprintPlanRevisionId: 'ECS-R4',
          workUnitScopeIds: [scoped('ECS-R4', 'WU-ECS3')],
          source: source(),
        },
        {
          workSlicePlanningPointId: 'planner-rd-r1',
          sprintPlanRevisionId: 'RD-R1',
          workUnitScopeIds: reviewRevisionUnits['RD-R1'].map((unit) => scoped('RD-R1', unit)),
          source: source(),
        },
        {
          workSlicePlanningPointId: 'planner-rd-r2-relationships',
          sprintPlanRevisionId: 'RD-R2',
          workUnitScopeIds: ['WU-RD1', 'WU-RD2', 'WU-RD3', 'WU-RD4', 'WU-RD5'].map((unit) =>
            scoped('RD-R2', unit),
          ),
          source: source(),
        },
        {
          workSlicePlanningPointId: 'planner-rd-r2-convergence',
          sprintPlanRevisionId: 'RD-R2',
          workUnitScopeIds: [scoped('RD-R2', 'WU-RD6')],
          source: source(),
        },
      ],
      gates: ['ECS-R1', 'ECS-R2', 'ECS-R3', 'ECS-R4'].map((revision) => ({
        gateId: `G2-${revision.slice(-2)}`,
        role: { kind: 'accepted_review_marker' },
        source: source(),
      })),
      documents: [
        {
          documentRefId: 'doc-rd-review',
          displayOrder: 0,
          recordedAt: { source: source(), value: '2026-07-28T13:00:00.000Z' },
          displayCategory: { source: source(), value: 'review' },
          sprintPlanRevisionIds: ['RD-R2'],
          workSlicePlanningPointIds: ['planner-rd-r2-convergence'],
          workUnitScopeIds: [scoped('RD-R2', 'WU-RD6')],
        },
        {
          documentRefId: 'doc-file-review',
          displayOrder: 0,
          recordedAt: { source: source(), value: '2026-07-17T05:00:00.000Z' },
          displayCategory: { source: source(), value: 'changed files' },
          sprintPlanRevisionIds: ['ECS-R4'],
          workSlicePlanningPointIds: ['planner-r4-integration'],
          workUnitScopeIds: [scoped('ECS-R4', 'WU-ECS2E')],
        },
        {
          documentRefId: 'doc-ecs-r1',
          displayOrder: 1,
          recordedAt: { source: source(), value: '2026-07-10T09:00:00.000Z' },
          displayCategory: { source: source(), value: 'plan' },
          sprintPlanRevisionIds: ['ECS-R1'],
          workSlicePlanningPointIds: ['planner-ecs-r1'],
          workUnitScopeIds: [],
        },
        {
          documentRefId: 'doc-g1',
          displayOrder: 2,
          recordedAt: { source: source(), value: '2026-07-13T09:00:00.000Z' },
          displayCategory: { source: source(), value: 'decision' },
          sprintPlanRevisionIds: ['ECS-R2'],
          workSlicePlanningPointIds: ['planner-ecs-r2'],
          workUnitScopeIds: [],
        },
        {
          documentRefId: 'doc-ecs2e-review',
          displayOrder: 3,
          recordedAt: { source: source(), value: '2026-07-14T19:31:00.000Z' },
          displayCategory: { source: source(), value: 'review' },
          sprintPlanRevisionIds: ['ECS-R4'],
          workSlicePlanningPointIds: ['planner-r4-integration'],
          workUnitScopeIds: [scoped('ECS-R4', 'WU-ECS2E')],
        },
      ],
      managedObjectives: [
        managedObjective({
          objectiveId: 'objective-review-relationships',
          sprintId: reviewSprintId,
          title: 'Make typed product relationships directly reviewable.',
          concernId: 'concern-review-relationships',
          workUnitIds: ['WU-RD1', 'WU-RD2'],
          handlerRefId: 'session-ref-handler-WU-RD1',
        }),
        managedObjective({
          objectiveId: 'objective-review-flow',
          sprintId: reviewSprintId,
          title: 'Make the parallel mixed-state workflow directly reviewable.',
          concernId: 'concern-review-lifecycle',
          workUnitIds: ['WU-RD3', 'WU-RD4'],
        }),
        managedObjective({
          objectiveId: 'objective-review-evidence',
          sprintId: reviewSprintId,
          title: 'Keep lifecycle and document evidence connected to managed work.',
          concernId: 'concern-review-documents',
          workUnitIds: ['WU-RD5', 'WU-RD6'],
        }),
        proposedObjective({
          objectiveId: 'objective-prestart-planner-interaction',
          sprintId: 'sprint-planner-work-unit',
          title: 'Clarify temporal planning and parallel implementation concerns.',
          concernId: 'concern-planner-interaction-forecast',
        }),
        proposedObjective({
          objectiveId: 'objective-prestart-plan-builder',
          sprintId: 'sprint-plan-builder',
          title: 'Clarify the Epic planning interaction before Sprint start.',
          concernId: 'concern-plan-builder-forecast',
        }),
      ],
      forecastTasks: [
        {
          forecastTaskId: 'forecast-planner-interaction-relationships',
          sprintId: 'sprint-planner-work-unit',
          title: 'Explore planner-to-parallel-work relationship boundaries.',
          objectiveIds: ['objective-prestart-planner-interaction'],
          concernIds: ['concern-planner-interaction-forecast'],
          source: source(),
        },
        {
          forecastTaskId: 'forecast-plan-builder-conversation',
          sprintId: 'sprint-plan-builder',
          title: 'Explore proposal and revision interaction.',
          objectiveIds: ['objective-prestart-plan-builder'],
          concernIds: ['concern-plan-builder-forecast'],
          source: source(),
        },
      ],
      workUnitLifecycle: [
        {
          entryId: 'ecs2e-planning',
          sprintId: controlSprintId,
          workUnitId: 'WU-ECS2E',
          sequence: 0,
          kind: 'planning',
          title: 'Plan Work Unit',
          summary: 'The Work Slice Planner recorded the Plan and Work Unit detail scope.',
          agentSessionId: 'recorded-session-planner-r4-integration',
          agentRole: 'work_slice_planner',
          invocationId: 'recorded-planner-r4-integration-scope',
          source: source(),
        },
        {
          entryId: 'ecs2e-first-return',
          sprintId: controlSprintId,
          workUnitId: 'WU-ECS2E',
          sequence: 1,
          kind: 'work',
          title: 'First return',
          summary: 'The Work Unit Implementer returned the first detail-surface implementation.',
          agentSessionId: 'recorded-implementer-WU-ECS2E',
          agentRole: 'work_unit_implementer',
          invocationId: 'recorded-implementer-WU-ECS2E-first-return',
          source: source(),
        },
        {
          entryId: 'ecs2e-review',
          sprintId: controlSprintId,
          workUnitId: 'WU-ECS2E',
          sequence: 2,
          kind: 'review',
          title: 'Review',
          summary: 'The Work Unit Handler reviewed the return and requested a correction.',
          agentSessionId: 'recorded-session-WU-ECS2E',
          agentRole: 'work_unit_handler',
          invocationId: 'recorded-handler-WU-ECS2E-first-review',
          source: source(),
        },
        {
          entryId: 'ecs2e-reprompt',
          sprintId: controlSprintId,
          workUnitId: 'WU-ECS2E',
          sequence: 3,
          kind: 'reprompt',
          title: 'Reprompt',
          summary: 'The Work Unit Handler recorded the bounded correction request.',
          agentSessionId: 'recorded-session-WU-ECS2E',
          agentRole: 'work_unit_handler',
          invocationId: 'recorded-handler-WU-ECS2E-reprompt',
          source: source(),
        },
        {
          entryId: 'ecs2e-renewed-work',
          sprintId: controlSprintId,
          workUnitId: 'WU-ECS2E',
          sequence: 4,
          kind: 'renewed_work',
          title: 'Renewed work',
          summary:
            'The Work Unit Implementer returned the corrected detail-surface implementation.',
          agentSessionId: 'recorded-implementer-WU-ECS2E',
          agentRole: 'work_unit_implementer',
          invocationId: 'recorded-implementer-WU-ECS2E-second-return',
          source: source(),
        },
        {
          entryId: 'ecs2e-acceptance',
          sprintId: controlSprintId,
          workUnitId: 'WU-ECS2E',
          sequence: 5,
          kind: 'review',
          title: 'Acceptance',
          summary: 'The Work Unit Handler accepted the corrected result.',
          agentSessionId: 'recorded-session-WU-ECS2E',
          agentRole: 'work_unit_handler',
          invocationId: 'recorded-handler-WU-ECS2E-acceptance',
          source: source(),
        },
        {
          entryId: 'rd1-planning',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD1',
          sequence: 0,
          kind: 'planning',
          title: 'Plan Work Unit',
          summary: 'The Work Slice Planner recorded the relationship-model scope.',
          agentSessionId: 'recorded-planner-rd-r2',
          agentRole: 'work_slice_planner',
          invocationId: 'recorded-planner-rd-r2-scope',
          source: source(),
        },
        {
          entryId: 'rd1-launch',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD1',
          sequence: 1,
          kind: 'launch',
          title: 'Launch',
          summary: 'The Work Unit Handler recorded the Work Unit Implementer launch.',
          agentSessionId: 'recorded-handler-WU-RD1',
          agentRole: 'work_unit_handler',
          invocationId: 'recorded-handler-WU-RD1-launch',
          source: source(),
        },
        {
          entryId: 'rd1-work',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD1',
          sequence: 2,
          kind: 'work',
          title: 'First work',
          summary: 'The Work Unit Implementer returned the initial relationship model.',
          agentSessionId: 'recorded-implementer-WU-RD1',
          agentRole: 'work_unit_implementer',
          invocationId: 'recorded-implementer-WU-RD1-first-work',
          source: source(),
        },
        {
          entryId: 'rd1-review-1',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD1',
          sequence: 3,
          kind: 'review',
          title: 'Review',
          summary: 'The Work Unit Handler requested a focused correction.',
          agentSessionId: 'recorded-handler-WU-RD1',
          agentRole: 'work_unit_handler',
          invocationId: 'recorded-handler-WU-RD1-first-review',
          source: source(),
        },
        {
          entryId: 'rd1-reprompt',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD1',
          sequence: 4,
          kind: 'reprompt',
          title: 'Reprompt',
          summary: 'The handler recorded the bounded correction request.',
          agentSessionId: 'recorded-handler-WU-RD1',
          agentRole: 'work_unit_handler',
          invocationId: 'recorded-handler-WU-RD1-reprompt',
          source: source(),
        },
        {
          entryId: 'rd1-renewed-work',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD1',
          sequence: 5,
          kind: 'renewed_work',
          title: 'Renewed work',
          summary: 'The Work Unit Implementer returned the corrected relationship model.',
          agentSessionId: 'recorded-implementer-WU-RD1',
          agentRole: 'work_unit_implementer',
          invocationId: 'recorded-implementer-WU-RD1-renewed-work',
          source: source(),
        },
        {
          entryId: 'rd1-review-2',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD1',
          sequence: 6,
          kind: 'review',
          title: 'Review',
          summary: 'The Work Unit Handler accepted the corrected result.',
          agentSessionId: 'recorded-handler-WU-RD1',
          agentRole: 'work_unit_handler',
          invocationId: 'recorded-handler-WU-RD1-second-review',
          source: source(),
        },
        {
          entryId: 'rd1-merge',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD1',
          sequence: 7,
          kind: 'merge',
          title: 'Merge',
          summary: 'The handler recorded integration into the review checkpoint.',
          agentSessionId: 'recorded-handler-WU-RD1',
          agentRole: 'work_unit_handler',
          invocationId: 'recorded-handler-WU-RD1-merge',
          source: source(),
        },
        {
          entryId: 'rd1-completion',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD1',
          sequence: 8,
          kind: 'completion',
          title: 'Completion',
          summary: 'Completion was recorded for this Work Unit.',
          agentSessionId: 'recorded-handler-WU-RD1',
          agentRole: 'work_unit_handler',
          invocationId: 'recorded-handler-WU-RD1-completion',
          source: source(),
        },
        {
          entryId: 'rd2-work',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD2',
          sequence: 0,
          kind: 'work',
          title: 'Implementation',
          summary: 'The Work Unit Implementer is building the draggable split surfaces.',
          agentSessionId: 'recorded-implementer-WU-RD2',
          agentRole: 'work_unit_implementer',
          invocationId: 'recorded-implementer-WU-RD2-work',
          source: source(),
        },
        {
          entryId: 'rd3-work',
          sprintId: reviewSprintId,
          workUnitId: 'WU-RD3',
          sequence: 0,
          kind: 'work',
          title: 'Returned work',
          summary: 'The Work Unit Implementer returned the lifecycle navigation change.',
          agentSessionId: 'recorded-implementer-WU-RD3',
          agentRole: 'work_unit_implementer',
          invocationId: 'recorded-implementer-WU-RD3-work',
          source: source(),
        },
      ],
      narratives: [
        {
          sprintId: controlSprintId,
          direction: {
            source: source(),
            value: 'Recorded development composition; no durable connector is available.',
          },
          progress: {
            source: source(),
            value:
              'Acceptance and integration are recorded independently from requests and observations.',
          },
        },
        {
          sprintId: reviewSprintId,
          direction: {
            source: source(),
            value: 'Recorded mixed-state review composition; no live runner is attached.',
          },
          progress: {
            source: source(),
            value: 'One Work Unit is completed, two are processing, and later work is planned.',
          },
        },
      ],
    },
  },
} as unknown as ProductReadCompositionInputV1;
