import { projectSprintWorkspacePresentation, type ProductSprintReadModelV1 } from './index';

describe('Sprint workspace presentation projector', () => {
  it('projects ordered active and historical revisions with stable explicit Planner groups', () => {
    const sprint = sprintReadModel();
    const first = projectSprintWorkspacePresentation(sprint);
    const second = projectSprintWorkspacePresentation({
      ...sprint,
      revisionViews: sprint.revisionViews.map((view) => ({
        ...view,
        workUnits: [...view.workUnits].reverse(),
        plannerActivityGroups: view.plannerActivityGroups.map((group) => ({
          ...group,
          workUnitScopeIds: [...group.workUnitScopeIds].reverse(),
        })),
      })),
    });

    expect(first.revisions.map((revision) => revision.sprintPlanRevisionId)).toEqual([
      'revision-1',
      'revision-2',
    ]);
    expect(first.activeSprintPlanRevisionId).toBe('revision-2');
    expect(first.selectedSprintPlanRevisionId).toBe('revision-1');
    expect(first.revisionViews).toEqual(second.revisionViews);
    expect(first.revisionViews).toMatchObject([
      {
        sprintPlanRevisionId: 'revision-1',
        plannerActivityGroups: [
          { sprintPlannerActivityId: 'activity-a', workUnitScopeIds: ['scope-1'] },
        ],
      },
      {
        sprintPlanRevisionId: 'revision-2',
        plannerActivityGroups: [
          { sprintPlannerActivityId: 'activity-b', workUnitScopeIds: ['scope-2a', 'scope-2b'] },
        ],
      },
    ]);
  });

  it('preserves truthful Work Unit, Document, Artifact, narrative, and continuation boundaries', () => {
    const presentation = projectSprintWorkspacePresentation(sprintReadModel());

    expect(presentation.revisionViews[0].workUnits[0]).toMatchObject({
      presentationState: 'under_review',
      observed: { launched: true, responsibilityAccepted: false },
      reviews: [{ outcome: 'needs_correction' }],
    });
    expect(presentation.revisionViews[0].gates[0]).toMatchObject({
      presentationRole: { kind: 'other', fallbackLabel: 'Gate' },
    });
    expect(presentation.revisionViews[1]).toMatchObject({
      sprintPlanRevisionId: 'revision-2',
      isCurrent: true,
      isSelected: false,
      workUnits: [
        { workUnitId: 'work-unit-2', presentationState: 'launched' },
        { workUnitId: 'work-unit-3', presentationState: 'waiting_for_dependencies' },
      ],
      gates: [{ gateId: 'gate-2', presentationRole: { kind: 'accepted_review_marker' } }],
    });
    expect(presentation.documents.map((document) => document.documentRefId)).toEqual([
      'document-early',
      'document-late',
    ]);
    expect(presentation.internalArtifacts.map((artifact) => artifact.artifactId)).toEqual([
      'artifact-1',
    ]);
    expect(presentation.narratives?.progress).toEqual({
      source: { status: 'unavailable', reason: 'not recorded' },
    });
    expect(presentation.continuation).toMatchObject({
      policy: { automaticEnabled: true },
      eligibility: { status: 'eligible' },
      continuationRequests: [{ continuationRequestId: 'request-1', targetKind: 'next_work_unit' }],
      observedContinuationIds: [],
      initiationObserved: false,
    });
  });
});

function source() {
  return {
    status: 'available' as const,
    sourceKind: 'orchestration_event' as const,
    sourceReferences: ['provenance-1'],
  };
}

function sprintReadModel(): ProductSprintReadModelV1 {
  return {
    sprintId: 'sprint-1',
    epicId: 'epic-1',
    title: 'Sprint',
    summary: 'Summary',
    details: 'Details',
    source: source(),
    sprintPlan: {
      sprintPlanId: 'plan-1',
      currentSprintPlanRevisionId: 'revision-2',
      selectedSprintPlanRevisionId: 'revision-1',
      revisions: [
        {
          sprintPlanRevisionId: 'revision-1',
          revision: 1,
          summary: 'Historical',
          source: source(),
          isCurrent: false,
          isSelected: true,
          workUnitScopes: [
            {
              workUnitScopeId: 'scope-1',
              workUnitId: 'work-unit-1',
              dependsOnWorkUnitScopeIds: [],
              gateIds: ['gate-1'],
            },
          ],
        },
        {
          sprintPlanRevisionId: 'revision-2',
          revision: 2,
          summary: 'Active',
          source: source(),
          supersedesSprintPlanRevisionId: 'revision-1',
          isCurrent: true,
          isSelected: false,
          workUnitScopes: [
            {
              workUnitScopeId: 'scope-2a',
              workUnitId: 'work-unit-2',
              dependsOnWorkUnitScopeIds: [],
              gateIds: ['gate-2'],
            },
            {
              workUnitScopeId: 'scope-2b',
              workUnitId: 'work-unit-3',
              dependsOnWorkUnitScopeIds: ['scope-2a'],
              gateIds: [],
            },
          ],
        },
      ],
    },
    plannerActivities: [
      {
        sprintPlannerActivityId: 'activity-a',
        title: 'A',
        purpose: 'Historical',
        source: source(),
        assessedSprintPlanRevisionIds: ['revision-1'],
      },
      {
        sprintPlannerActivityId: 'activity-b',
        title: 'B',
        purpose: 'Active',
        source: source(),
        assessedSprintPlanRevisionIds: ['revision-2'],
      },
    ],
    revisionViews: [
      {
        sprintPlanRevisionId: 'revision-1',
        revision: 1,
        summary: 'Historical',
        source: source(),
        isCurrent: false,
        isSelected: true,
        workUnitScopes: [
          {
            workUnitScopeId: 'scope-1',
            workUnitId: 'work-unit-1',
            dependsOnWorkUnitScopeIds: [],
            gateIds: ['gate-1'],
          },
        ],
        plannerActivityGroups: [
          {
            sprintPlannerActivityId: 'activity-a',
            title: 'A',
            purpose: 'Historical',
            source: source(),
            membershipSource: source(),
            workUnitScopeIds: ['scope-1'],
          },
        ],
        workUnits: [
          {
            workUnitId: 'work-unit-1',
            title: 'Work',
            summary: 'Summary',
            details: 'Details',
            source: source(),
            workUnitScopeId: 'scope-1',
            sprintPlanRevisionId: 'revision-1',
            fixedExecutionScopeIds: ['scope-1'],
            dependencies: [],
            gateIds: ['gate-1'],
            attempts: [
              { attemptId: 'attempt-1', workUnitExecutionId: 'execution-1', returned: true },
            ],
            reviews: [{ reviewId: 'review-1', outcome: 'needs_correction' }],
            observed: {
              executionRequested: true,
              launched: true,
              returned: true,
              integrated: false,
              responsibilityAccepted: false,
            },
            presentationState: 'under_review',
          },
        ],
        gates: [
          {
            gateId: 'gate-1',
            title: 'Gate',
            summary: 'Summary',
            source: source(),
            criteriaRevisionIds: [],
            feedback: [],
            presentationRole: { kind: 'other', fallbackLabel: 'Gate' },
            presentationSource: source(),
          },
        ],
        reviews: [],
      },
      {
        sprintPlanRevisionId: 'revision-2',
        revision: 2,
        summary: 'Active',
        source: source(),
        supersedesSprintPlanRevisionId: 'revision-1',
        isCurrent: true,
        isSelected: false,
        workUnitScopes: [
          {
            workUnitScopeId: 'scope-2a',
            workUnitId: 'work-unit-2',
            dependsOnWorkUnitScopeIds: [],
            gateIds: ['gate-2'],
          },
          {
            workUnitScopeId: 'scope-2b',
            workUnitId: 'work-unit-3',
            dependsOnWorkUnitScopeIds: ['scope-2a'],
            gateIds: [],
          },
        ],
        plannerActivityGroups: [
          {
            sprintPlannerActivityId: 'activity-b',
            title: 'B',
            purpose: 'Active',
            source: source(),
            membershipSource: source(),
            workUnitScopeIds: ['scope-2b', 'scope-2a'],
          },
        ],
        workUnits: [
          {
            workUnitId: 'work-unit-2',
            title: 'Dependency',
            summary: 'Summary',
            details: 'Details',
            source: source(),
            workUnitScopeId: 'scope-2a',
            sprintPlanRevisionId: 'revision-2',
            fixedExecutionScopeIds: [],
            dependencies: [],
            gateIds: ['gate-2'],
            attempts: [],
            reviews: [],
            observed: {
              executionRequested: true,
              launched: true,
              returned: false,
              integrated: false,
              responsibilityAccepted: false,
            },
            presentationState: 'launched',
          },
          {
            workUnitId: 'work-unit-3',
            title: 'Dependent',
            summary: 'Summary',
            details: 'Details',
            source: source(),
            workUnitScopeId: 'scope-2b',
            sprintPlanRevisionId: 'revision-2',
            fixedExecutionScopeIds: [],
            dependencies: [{ workUnitScopeId: 'scope-2a', workUnitId: 'work-unit-2' }],
            gateIds: [],
            attempts: [],
            reviews: [],
            observed: {
              executionRequested: false,
              launched: false,
              returned: false,
              integrated: false,
              responsibilityAccepted: false,
            },
            presentationState: 'waiting_for_dependencies',
          },
        ],
        gates: [
          {
            gateId: 'gate-2',
            title: 'Gate two',
            summary: 'Summary',
            source: source(),
            criteriaRevisionIds: [],
            feedback: [],
            presentationRole: { kind: 'accepted_review_marker' },
            presentationSource: source(),
          },
        ],
        reviews: [],
      },
    ],
    concerns: [],
    reviews: [],
    documents: [
      {
        documentRefId: 'document-late',
        title: 'Late',
        classification: 'handoff',
        artifactIds: ['artifact-1'],
        changedFileReferenceIds: [],
        provenanceReference: 'provenance-1',
        ownershipSource: source(),
      },
      {
        documentRefId: 'document-early',
        title: 'Early',
        classification: 'handoff',
        artifactIds: [],
        changedFileReferenceIds: [],
        provenanceReference: 'provenance-1',
        ownershipSource: source(),
      },
    ],
    internalArtifacts: [
      {
        artifactId: 'artifact-1',
        kind: 'handoff',
        provenanceReference: 'provenance-1',
        ownershipSource: source(),
      },
    ],
    workspacePresentation: {
      plannerActivityMembership: [
        {
          sprintPlannerActivityId: 'activity-b',
          sprintPlanRevisionId: 'revision-2',
          workUnitScopeIds: ['scope-2'],
          source: source(),
        },
        {
          sprintPlannerActivityId: 'activity-a',
          sprintPlanRevisionId: 'revision-1',
          workUnitScopeIds: ['scope-1'],
          source: source(),
        },
      ],
      gates: [
        { gateId: 'gate-1', role: { kind: 'other', fallbackLabel: 'Gate' }, source: source() },
      ],
      documents: [
        {
          documentRefId: 'document-late',
          displayOrder: 1,
          recordedAt: { source: source(), value: '2026-07-15T10:00:00.000Z' },
          displayCategory: { source: source(), value: 'handoff' },
          sprintPlanRevisionIds: ['revision-1'],
          sprintPlannerActivityIds: ['activity-a'],
          workUnitScopeIds: ['scope-1'],
        },
        {
          documentRefId: 'document-early',
          displayOrder: 0,
          recordedAt: { source: source(), value: '2026-07-15T09:00:00.000Z' },
          displayCategory: { source: source(), value: 'handoff' },
          sprintPlanRevisionIds: [],
          sprintPlannerActivityIds: [],
          workUnitScopeIds: [],
        },
      ],
      narratives: { progress: { source: { status: 'unavailable', reason: 'not recorded' } } },
    },
    agentSessionReferences: [],
    continuation: {
      level: 'sprint',
      policy: { policyId: 'policy-1', automaticEnabled: true },
      eligibility: { evaluationId: 'evaluation-1', status: 'eligible' },
      commandResults: [{ commandId: 'command-1', state: 'acknowledged' }],
      eventEligibilityFacts: [
        { policyEligibilityFactId: 'fact-1', automaticEnabled: true, eligible: true },
      ],
      continuationRequests: [{ continuationRequestId: 'request-1', targetKind: 'next_work_unit' }],
      observedContinuationIds: [],
      initiationObserved: false,
    },
  };
}
