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
        workSlicePlanningPointGroups: view.workSlicePlanningPointGroups.map((group) => ({
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
        workSlicePlanningPointGroups: [
          { workSlicePlanningPointId: 'activity-a', workUnitScopeIds: ['scope-1'] },
        ],
      },
      {
        sprintPlanRevisionId: 'revision-2',
        workSlicePlanningPointGroups: [
          { workSlicePlanningPointId: 'activity-b', workUnitScopeIds: ['scope-2a', 'scope-2b'] },
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
    expect(presentation.revisionViews[1].workUnits[0].attemptHistory[0]?.implementerOutcome).toMatchObject({
      submittedOutcome: {
        summaryClaim: 'Implemented the bounded change.',
        validationStatementClaim: 'Focused checks passed.',
      },
      evidence: {
        changedFiles: [{ evidenceRef: 'evidence-1', contentFingerprint: 'content-1' }],
      },
      terminalLifecycle: { status: 'completed' },
      applicationAcceptedAt: '2026-08-04T00:00:10Z',
      handlerReviewReadyAt: '2026-08-04T00:00:11Z',
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
    expect(presentation.epicRunnerObjectives.map(({ title }) => title)).toEqual([
      'Preserve the Sprint task.',
      'Make the plan reviewable.',
    ]);
    expect(presentation.sprintRunnerConcerns).toEqual([
      expect.objectContaining({
        sprintRunnerConcernId: 'sprintRunnerConcern-1',
        graphElementRefs: [
          { kind: 'work_slice_planning_point', id: 'activity-a' },
          { kind: 'work_unit', id: 'work-unit-1' },
        ],
      }),
    ]);
    expect(presentation.workUnitLifecycle.map(({ entryId }) => entryId)).toEqual([
      'lifecycle-1',
      'lifecycle-2',
    ]);
    expect(presentation.continuation).toMatchObject({
      policy: { automaticEnabled: true },
      eligibility: { status: 'eligible' },
      continuationRequests: [
        { continuationRequestId: 'request-1', targetKind: 'next_work_slice_planner' },
      ],
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
    planningState: {
      source: source(),
      value: {
        kind: 'started_plan',
        currentWorkSlicePlanningPointId: 'activity-b',
        repositoryAssessmentSummary: 'Recorded current branch and repository state.',
        reevaluatedAt: '2026-07-15T09:00:00.000Z',
      },
    },
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
    workSlicePlanningPoints: [
      {
        workSlicePlanningPointId: 'activity-a',
        title: 'A',
        purpose: 'Historical',
        source: source(),
        assessedSprintPlanRevisionIds: ['revision-1'],
      },
      {
        workSlicePlanningPointId: 'activity-b',
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
        workSlicePlanningPointGroups: [
          {
            workSlicePlanningPointId: 'activity-a',
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
        workSlicePlanningPointGroups: [
          {
            workSlicePlanningPointId: 'activity-b',
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
            retryAttempts: [],
            attemptHistory: [{
              ordinal: 0,
              attemptId: 'attempt-1',
              implementerOutcome: {
              attemptId: 'attempt-1',
              implementerSessionId: 'implementer-session-1',
              originalImplementerInvocationId: 'implementer-invocation-1',
              reportingInvocationId: 'reporting-invocation-1',
              reportingHarnessRevisionId: 'reporting-revision-1',
              reportingHarnessConfigurationDigest: 'reporting-digest-1',
              reportingHarnessRepositoryCommitRef: 'reporting-commit-1',
              reportingRequestedAt: '2026-08-04T00:00:00Z',
              reportingPreparedAt: '2026-08-04T00:00:01Z',
              reportingHarnessBoundAt: '2026-08-04T00:00:02Z',
              reportingLaunchRequestedAt: '2026-08-04T00:00:03Z',
              reportingLaunchAcceptedAt: '2026-08-04T00:00:04Z',
              reportingReadyAt: '2026-08-04T00:00:05Z',
              submittedOutcome: {
                variant: 'review_pending',
                summaryClaim: 'Implemented the bounded change.',
                validationStatementClaim: 'Focused checks passed.',
                semanticPayloadFingerprint: 'payload-1',
                submittedAt: '2026-08-04T00:00:06Z',
                validationAt: '2026-08-04T00:00:06Z',
                validationResult: 'valid',
              },
              evidence: {
                changedFiles: [
                  {
                    evidenceRef: 'evidence-1',
                    displayName: 'src/feature.ts',
                    changeKind: 'modified',
                    contentFingerprint: 'content-1',
                  },
                ],
                comparisonFingerprint: 'comparison-1',
                readyAt: '2026-08-04T00:00:07Z',
              },
              semanticCompletion: {
                invocationId: 'reporting-invocation-1',
                completedAt: '2026-08-04T00:00:08Z',
              },
              terminalLifecycle: {
                status: 'completed',
                observedAt: '2026-08-04T00:00:09Z',
              },
              applicationAcceptedAt: '2026-08-04T00:00:10Z',
              handlerReviewReadyAt: '2026-08-04T00:00:11Z',
              },
            }],
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
      workSlicePlanningPointMembership: [
        {
          workSlicePlanningPointId: 'activity-b',
          sprintPlanRevisionId: 'revision-2',
          workUnitScopeIds: ['scope-2'],
          source: source(),
        },
        {
          workSlicePlanningPointId: 'activity-a',
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
          workSlicePlanningPointIds: ['activity-a'],
          workUnitScopeIds: ['scope-1'],
        },
        {
          documentRefId: 'document-early',
          displayOrder: 0,
          recordedAt: { source: source(), value: '2026-07-15T09:00:00.000Z' },
          displayCategory: { source: source(), value: 'handoff' },
          sprintPlanRevisionIds: [],
          workSlicePlanningPointIds: [],
          workUnitScopeIds: [],
        },
      ],
      epicRunnerObjectives: [
        {
          objectiveId: 'objective-1',
          sprintId: 'sprint-1',
          title: 'Preserve the Sprint task.',
          source: source(),
        },
        {
          objectiveId: 'objective-2',
          sprintId: 'sprint-1',
          title: 'Make the plan reviewable.',
          source: source(),
        },
      ],
      sprintRunnerConcerns: [
        {
          sprintRunnerConcernId: 'sprintRunnerConcern-1',
          sprintId: 'sprint-1',
          title: 'Connect the Plan and Work Unit.',
          source: source(),
          graphElementRefs: [
            { kind: 'work_slice_planning_point', id: 'activity-a' },
            { kind: 'work_unit', id: 'work-unit-1' },
          ],
        },
      ],
      workUnitLifecycle: [
        {
          entryId: 'lifecycle-2',
          sprintId: 'sprint-1',
          workUnitId: 'work-unit-1',
          sequence: 2,
          kind: 'review',
          title: 'Review',
          summary: 'Reviewed.',
          agentSessionId: 'session-1',
          agentRole: 'work_unit_handler',
          invocationId: 'invocation-2',
          source: source(),
        },
        {
          entryId: 'lifecycle-1',
          sprintId: 'sprint-1',
          workUnitId: 'work-unit-1',
          sequence: 1,
          kind: 'work',
          title: 'Work',
          summary: 'Worked.',
          agentSessionId: 'session-1',
          agentRole: 'work_unit_implementer',
          invocationId: 'invocation-1',
          source: source(),
        },
      ],
      narratives: { progress: { source: { status: 'unavailable', reason: 'not recorded' } } },
    },
    agentSessionReferences: [
      {
        agentSessionRefId: 'session-ref-1',
        agentSessionId: 'session-1',
        title: 'Recorded session',
        source: source(),
        targetKind: 'work_unit_execution',
        targetId: 'execution-1',
        semanticRole: 'work_unit_implementer',
      },
    ],
    continuation: {
      level: 'sprint',
      policy: { policyId: 'policy-1', automaticEnabled: true },
      eligibility: { evaluationId: 'evaluation-1', status: 'eligible' },
      commandResults: [{ commandId: 'command-1', state: 'acknowledged' }],
      eventEligibilityFacts: [
        { policyEligibilityFactId: 'fact-1', automaticEnabled: true, eligible: true },
      ],
      continuationRequests: [
        { continuationRequestId: 'request-1', targetKind: 'next_work_slice_planner' },
      ],
      observedContinuationIds: [],
      initiationObserved: false,
    },
  };
}
