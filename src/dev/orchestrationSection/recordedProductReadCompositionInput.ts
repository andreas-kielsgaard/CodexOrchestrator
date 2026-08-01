/**
 * Recorded development facts for the product composition boundary. These are application-owned
 * evidence, not a connector, runtime, persistence record, or a projection of the old fixture.
 */
import {
  AGENT_CONTROL_CONTRACTS_V1,
  ARTIFACT_ACCESS_CONTRACTS_V1,
  ORCHESTRATION_EVENTS_V1,
  type ProductReadCompositionInputV1,
} from '../../application/orchestrations';

const epicId = 'epic-codex-runner-workspace';
const controlSprintId = 'sprint-control-surface';
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

const simpleSprints = sprintDefinitions.filter(([id]) => id !== controlSprintId);
const simplePlans = simpleSprints.map(([id]) => ({ sprintPlanId: `plan-${id}`, sprintId: id }));
const simpleRevisions = simpleSprints.map(([id]) => ({
  sprintPlanRevisionId: `${id}-r1`,
  sprintPlanId: `plan-${id}`,
  revision: 1,
}));

/** Complete, ordered canonical input used by the recorded development client. */
export const recordedProductReadCompositionInput = {
  events: {
    version: ORCHESTRATION_EVENTS_V1,
    epics: [{ epicId }],
    sprints: sprintDefinitions.map(([sprintId]) => ({ sprintId, epicId })),
    sprintPlans: [
      ...simplePlans,
      { sprintPlanId: 'plan-control-surface', sprintId: controlSprintId },
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
    ],
    workUnits: Object.keys(workUnitText).map((workUnitId) => ({ workUnitId })),
    workUnitScopes: Object.entries(revisionUnits).flatMap(([revision, units]) =>
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
    sprintPlannerActivities: [
      ...(['ECS-R1', 'ECS-R2', 'ECS-R3'] as const).map((revision) => ({
        sprintPlannerActivityId: `planner-${revision.toLowerCase()}`,
        sprintPlanId: 'plan-control-surface',
        assessedSprintPlanRevisionIds: [revision],
      })),
      ...['foundation', 'integration', 'convergence'].map((group) => ({
        sprintPlannerActivityId: `planner-r4-${group}`,
        sprintPlanId: 'plan-control-surface',
        assessedSprintPlanRevisionIds: ['ECS-R4'],
      })),
    ],
    workUnitExecutions: executionUnits.map((workUnitId) => ({
      workUnitExecutionId: `execution-${workUnitId}`,
      workUnitId,
      fixedWorkUnitScopeId: scoped('ECS-R4', workUnitId),
    })),
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
    ],
    agentSessions: [
      { agentSessionId: 'recorded-epic-runner-manual-continuation-ready' },
      { agentSessionId: 'recorded-sprint-control-surface-discovery' },
      { agentSessionId: 'recorded-session-planner-r4-integration' },
      { agentSessionId: 'recorded-session-reviewer-WU-ECS2E' },
      ...executionUnits.map((workUnitId) => ({ agentSessionId: `recorded-session-${workUnitId}` })),
    ],
    agentSessionReferences: [
      {
        agentSessionRefId: 'session-ref-epic-runner',
        agentSessionId: 'recorded-epic-runner-manual-continuation-ready',
        targetKind: 'epic',
        targetId: epicId,
        semanticRole: 'epic',
      },
      {
        agentSessionRefId: 'session-ref-sprint',
        agentSessionId: 'recorded-sprint-control-surface-discovery',
        targetKind: 'sprint',
        targetId: controlSprintId,
        semanticRole: 'sprint',
      },
      {
        agentSessionRefId: 'session-ref-planner-r4-integration',
        agentSessionId: 'recorded-session-planner-r4-integration',
        targetKind: 'sprint_planner_activity',
        targetId: 'planner-r4-integration',
        semanticRole: 'work_slice_planner',
      },
      ...executionUnits.map((workUnitId) => ({
        agentSessionRefId: `session-ref-${workUnitId}`,
        agentSessionId: `recorded-session-${workUnitId}`,
        targetKind: 'work_unit_execution',
        targetId: `execution-${workUnitId}`,
        semanticRole: 'work_unit_implementer',
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
    executionRequests: executionUnits.map((workUnitId) => ({
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
    ],
    continuationRequests: [],
    observedContinuations: [],
    observedHandoffs: [],
    internalArtifacts: [
      { artifactId: 'artifact-ecs-r1', provenanceId },
      { artifactId: 'artifact-g1', provenanceId },
      { artifactId: 'artifact-ecs2e-review', provenanceId },
    ],
    documentReferences: [
      { documentRefId: 'doc-ecs-r1', artifactIds: ['artifact-ecs-r1'], provenanceId },
      { documentRefId: 'doc-g1', artifactIds: ['artifact-g1'], provenanceId },
      { documentRefId: 'doc-ecs2e-review', artifactIds: ['artifact-ecs2e-review'], provenanceId },
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
    ],
    changedFileReferences: [],
    documents: [
      {
        documentRefId: 'doc-ecs-r1',
        classification: 'other',
        title: 'Original ECS-R1 plan',
        summary: 'Recorded original plan.',
        artifactIds: ['artifact-ecs-r1'],
        changedFileReferenceIds: [],
        provenanceReference: provenanceId,
      },
      {
        documentRefId: 'doc-g1',
        classification: 'review_material',
        title: 'G1 feedback and ECS-R2 replan',
        summary: 'Recorded G1 feedback.',
        artifactIds: ['artifact-g1'],
        changedFileReferenceIds: [],
        provenanceReference: provenanceId,
      },
      {
        documentRefId: 'doc-ecs2e-review',
        classification: 'review_material',
        title: 'WU-ECS2E corrected visual review',
        summary: 'Recorded accepted review.',
        artifactIds: ['artifact-ecs2e-review'],
        changedFileReferenceIds: [],
        provenanceReference: provenanceId,
      },
    ],
    requests: [],
    results: [],
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
    ],
    plannerActivities: [
      ...(['ECS-R1', 'ECS-R2', 'ECS-R3'] as const).map((revision) => ({
        sprintPlannerActivityId: `planner-${revision.toLowerCase()}`,
        title: `Planner activity ${revision}`,
        purpose: 'Recorded Plan assessment.',
        source: source(),
      })),
      {
        sprintPlannerActivityId: 'planner-r4-foundation',
        title: 'Foundation and correction',
        purpose: 'Accepted semantic and layout substrate.',
        source: source(),
      },
      {
        sprintPlannerActivityId: 'planner-r4-integration',
        title: 'Integrated detail surfaces',
        purpose: 'Own the recorded integrated detail surfaces.',
        source: source(),
      },
      {
        sprintPlannerActivityId: 'planner-r4-convergence',
        title: 'Convergence handoff',
        purpose: 'Hold consolidation until review acceptance.',
        source: source(),
      },
    ],
    workUnits: Object.entries(workUnitText).map(([workUnitId, [title, summary, details]]) => ({
      workUnitId,
      title,
      summary,
      details,
      source: source(),
    })),
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
        requiredWorkUnitIds: executionUnits,
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
      {
        agentSessionId: 'recorded-session-reviewer-WU-ECS2E',
        title: 'Recorded reviewer WU-ECS2E',
        source: source(),
      },
      ...executionUnits.map((workUnitId) => ({
        agentSessionId: `recorded-session-${workUnitId}`,
        title: `${workUnitText[workUnitId][0]} worker`,
        source: source(),
      })),
    ],
    artifactOwnership: [
      { artifactId: 'artifact-ecs-r1', sprintId: controlSprintId, source: source() },
      { artifactId: 'artifact-g1', sprintId: controlSprintId, source: source() },
      { artifactId: 'artifact-ecs2e-review', sprintId: controlSprintId, source: source() },
    ],
    documentOwnership: [
      { documentRefId: 'doc-ecs-r1', sprintId: controlSprintId, source: source() },
      { documentRefId: 'doc-g1', sprintId: controlSprintId, source: source() },
      { documentRefId: 'doc-ecs2e-review', sprintId: controlSprintId, source: source() },
    ],
    sprintWorkspacePresentation: {
      plannerActivityMembership: [
        ...(['ECS-R1', 'ECS-R2', 'ECS-R3'] as const).map((revision) => ({
          sprintPlannerActivityId: `planner-${revision.toLowerCase()}`,
          sprintPlanRevisionId: revision,
          workUnitScopeIds: revisionUnits[revision].map((unit) => scoped(revision, unit)),
          source: source(),
        })),
        {
          sprintPlannerActivityId: 'planner-r4-foundation',
          sprintPlanRevisionId: 'ECS-R4',
          workUnitScopeIds: ['WU-ECS1', 'WU-ECS2A', 'WU-ECS2B', 'WU-ECS2C'].map((unit) =>
            scoped('ECS-R4', unit),
          ),
          source: source(),
        },
        {
          sprintPlannerActivityId: 'planner-r4-integration',
          sprintPlanRevisionId: 'ECS-R4',
          workUnitScopeIds: ['WU-ECS2E', 'WU-ECS2D'].map((unit) => scoped('ECS-R4', unit)),
          source: source(),
        },
        {
          sprintPlannerActivityId: 'planner-r4-convergence',
          sprintPlanRevisionId: 'ECS-R4',
          workUnitScopeIds: [scoped('ECS-R4', 'WU-ECS3')],
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
          documentRefId: 'doc-ecs-r1',
          displayOrder: 0,
          recordedAt: { source: source(), value: '2026-07-10T09:00:00.000Z' },
          displayCategory: { source: source(), value: 'plan' },
          sprintPlanRevisionIds: ['ECS-R1'],
          sprintPlannerActivityIds: ['planner-ecs-r1'],
          workUnitScopeIds: [],
        },
        {
          documentRefId: 'doc-g1',
          displayOrder: 1,
          recordedAt: { source: source(), value: '2026-07-13T09:00:00.000Z' },
          displayCategory: { source: source(), value: 'decision' },
          sprintPlanRevisionIds: ['ECS-R2'],
          sprintPlannerActivityIds: ['planner-ecs-r2'],
          workUnitScopeIds: [],
        },
        {
          documentRefId: 'doc-ecs2e-review',
          displayOrder: 2,
          recordedAt: { source: source(), value: '2026-07-14T19:31:00.000Z' },
          displayCategory: { source: source(), value: 'review' },
          sprintPlanRevisionIds: ['ECS-R4'],
          sprintPlannerActivityIds: ['planner-r4-integration'],
          workUnitScopeIds: [scoped('ECS-R4', 'WU-ECS2E')],
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
      ],
    },
  },
} as unknown as ProductReadCompositionInputV1;
