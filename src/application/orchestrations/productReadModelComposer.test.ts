import {
  AGENT_CONTROL_CONTRACTS_V1,
  ARTIFACT_ACCESS_CONTRACTS_V1,
  composeProductOrchestrationReadModels,
  ORCHESTRATION_EVENTS_V1,
  type ProductReadCompositionInputV1,
  type ProductSprintWorkspacePresentationMetadataV1,
} from './index';

describe('product read-model composer', () => {
  it('composes an overview and complete Sprint detail from product contracts only', () => {
    const result = compose(productInput());
    const read = result.epics[0];

    expect(read).toMatchObject({
      epicId: 'epic-1',
      title: 'Epic',
      continuation: { policy: { automaticEnabled: true }, initiationObserved: false },
    });
    expect(read.agentSessionReferences).toEqual(
      expect.arrayContaining([expect.objectContaining({ semanticRole: 'epic_runner' })]),
    );
    const sprint = read.sprints[0];
    expect(result.unassociatedAgentSessionReferences).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ semanticRole: 'other', otherTargetType: 'future_target' }),
      ]),
    );
    expect(sprint).toMatchObject({
      sprintPlan: {
        currentSprintPlanRevisionId: 'revision-1',
        selectedSprintPlanRevisionId: 'revision-1',
      },
      plannerActivities: [expect.objectContaining({ sprintPlannerActivityId: 'activity-1' })],
      revisionViews: [
        expect.objectContaining({
          gates: [
            expect.objectContaining({
              feedback: [expect.objectContaining({ boundary: 'designed_feedback_flow' })],
            }),
          ],
        }),
      ],
      concerns: [expect.objectContaining({ state: 'responsibility_accepted' })],
      reviews: expect.arrayContaining([
        expect.objectContaining({ subjectKind: 'sprint_plan_revision', subjectId: 'revision-1' }),
        expect.objectContaining({ subjectKind: 'document_reference', subjectId: 'document-1' }),
      ]),
      documents: [
        expect.objectContaining({ documentRefId: 'document-1', artifactIds: ['artifact-1'] }),
      ],
      internalArtifacts: [expect.objectContaining({ artifactId: 'artifact-1' })],
      continuation: {
        initiationObserved: true,
        observedContinuationIds: ['continuation-1'],
        continuationRequests: [
          { continuationRequestId: 'continuation-request-1', targetKind: 'next_work_unit' },
        ],
        commandResults: [{ commandId: 'command-sprint', state: 'orchestration_event_recorded' }],
      },
    });
    expect(sprint.agentSessionReferences).toEqual(
      expect.arrayContaining([expect.objectContaining({ semanticRole: 'sprint_runner' })]),
    );
    expect(sprint.revisionViews[0].workUnits).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          fixedExecutionScopeIds: ['scope-1'],
          presentationState: 'responsibility_accepted',
          observed: expect.objectContaining({
            executionRequested: true,
            launched: true,
            returned: true,
            integrated: true,
          }),
          reviews: expect.arrayContaining([expect.objectContaining({ outcome: 'accepted' })]),
        }),
      ]),
    );
  });

  it('keeps policy, eligibility, requests, acknowledgement, and observed continuation distinct', () => {
    const input = productInput();
    input.events.observedContinuations = [];
    input.agentControl.results = [
      {
        agentControlResultId: 'result-sprint',
        agentControlCommandId: 'command-sprint',
        state: 'acknowledged',
        recordedAt: TIME,
      },
    ];
    const continuation = compose(input).epics[0].sprints[0].continuation;

    expect(continuation).toMatchObject({
      policy: { automaticEnabled: true },
      eligibility: { status: 'eligible' },
      commandResults: [{ state: 'acknowledged' }],
      observedContinuationIds: [],
      initiationObserved: false,
    });
  });

  it.each(['pending', 'unavailable', 'unsupported'] as const)(
    'preserves %s overview authority without inventing movement or state',
    (status) => {
      const input = productInput();
      input.referenceIndex.epicOverviews[0].currentMovement = {
        source: { status, reason: `${status} from source` },
      };
      input.referenceIndex.epicOverviews[0].state = {
        source: { status, reason: `${status} from source` },
      };

      const overview = compose(input).epics[0].overview;
      expect(overview.currentMovement).toEqual({
        source: { status, reason: `${status} from source` },
      });
      expect(overview.state).toEqual({ source: { status, reason: `${status} from source` } });
      expect(overview.currentMovement).not.toHaveProperty('value');
      expect(overview.state).not.toHaveProperty('value');
    },
  );

  it('keeps artifacts and Documents with their explicit Sprint owner', () => {
    const input = productInput();
    addSecondSprint(input);
    const sprints = compose(input).epics[0].sprints;

    expect(sprints.find(({ sprintId }) => sprintId === 'sprint-1')?.internalArtifacts).toEqual([
      expect.objectContaining({ artifactId: 'artifact-1' }),
    ]);
    expect(sprints.find(({ sprintId }) => sprintId === 'sprint-2')?.internalArtifacts).toEqual([
      expect.objectContaining({ artifactId: 'artifact-2' }),
    ]);
    expect(sprints.find(({ sprintId }) => sprintId === 'sprint-1')?.documents).toEqual([
      expect.objectContaining({ documentRefId: 'document-1' }),
    ]);
    expect(sprints.find(({ sprintId }) => sprintId === 'sprint-2')?.documents).toEqual([
      expect.objectContaining({ documentRefId: 'document-2' }),
    ]);
    expect(
      sprints.find(({ sprintId }) => sprintId === 'sprint-1')?.workspacePresentation.narratives,
    ).toEqual(expect.objectContaining({ direction: expect.any(Object) }));
    expect(
      sprints.find(({ sprintId }) => sprintId === 'sprint-2')?.workspacePresentation.narratives,
    ).toBeUndefined();
  });

  it('accepts a policy-eligibility Event fact as an event-recorded result reference', () => {
    const input = productInput();
    input.agentControl.results[0].orchestrationEventReference = 'fact-sprint';
    expect(compose(input).epics[0].sprints[0].continuation.commandResults).toEqual([
      { commandId: 'command-sprint', state: 'orchestration_event_recorded' },
    ]);
  });

  it.each([
    [
      'a dangling event reference',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        (input.agentControl.results[0].orchestrationEventReference = 'missing-event'),
      'requires an Orchestration Event fact',
    ],
    [
      'contradictory event outcomes',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        input.agentControl.results.push({
          agentControlResultId: 'result-sprint-2',
          agentControlCommandId: 'command-sprint',
          state: 'orchestration_event_recorded',
          orchestrationEventReference: 'integration-1',
          recordedAt: TIME,
        }),
      'cannot record contradictory',
    ],
    [
      'a mismatched Document provenance',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        (input.artifactAccess.documents[0].provenanceReference = 'provenance-other'),
      'Document contract must match',
    ],
    [
      'mismatched Document artifact membership',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        (input.events.documentReferences[0].artifactIds = []),
      'artifact membership must match',
    ],
    [
      'a missing artifact contract',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        input.events.internalArtifacts.push({
          artifactId: 'artifact-unrepresented',
          provenanceId: 'provenance-1',
        }),
      'artifact ownership entry per Event identity',
    ],
  ])('rejects %s at the product composition boundary', (_label, mutate, message) => {
    const input = productInput();
    mutate(input);
    expect(() => compose(input)).toThrow(message);
  });

  it('keeps every revision view available while selection changes only its selection metadata', () => {
    const input = productInput();
    input.events.sprintPlanRevisions.push({
      sprintPlanRevisionId: 'revision-5',
      sprintPlanId: 'plan-1',
      revision: 5,
      supersedesSprintPlanRevisionId: 'revision-1',
    });
    input.events.workUnitScopes.push({
      workUnitScopeId: 'scope-5a',
      sprintPlanRevisionId: 'revision-5',
      workUnitId: 'work-unit-5a',
      dependsOnWorkUnitScopeIds: [],
      gateIds: ['gate-5'],
    });
    input.events.workUnitScopes.push({
      workUnitScopeId: 'scope-5b',
      sprintPlanRevisionId: 'revision-5',
      workUnitId: 'work-unit-5b',
      dependsOnWorkUnitScopeIds: ['scope-5a'],
      gateIds: [],
    });
    input.events.workUnits.push({ workUnitId: 'work-unit-5a' }, { workUnitId: 'work-unit-5b' });
    input.events.gates.push({ gateId: 'gate-5', sprintPlanRevisionId: 'revision-5' });
    input.referenceIndex.sprintPlanRevisions.push({
      sprintPlanRevisionId: 'revision-5',
      summary: 'Revision 5',
      source: source(),
    });
    input.events.sprintPlannerActivities[0].assessedSprintPlanRevisionIds.push('revision-5');
    input.referenceIndex.sprintWorkspacePresentation!.plannerActivityMembership.push({
      sprintPlannerActivityId: 'activity-1',
      sprintPlanRevisionId: 'revision-5',
      workUnitScopeIds: ['scope-5a', 'scope-5b'],
      source: source(),
    });
    input.referenceIndex.workUnits.push(
      {
        workUnitId: 'work-unit-5a',
        title: 'Independent',
        summary: 'Summary',
        details: 'Details',
        source: source(),
      },
      {
        workUnitId: 'work-unit-5b',
        title: 'Dependent',
        summary: 'Summary',
        details: 'Details',
        source: source(),
      },
    );
    input.referenceIndex.gates.push({
      gateId: 'gate-5',
      title: 'Gate five',
      summary: 'Review',
      source: source(),
    });
    input.referenceIndex.sprintWorkspacePresentation!.gates.push({
      gateId: 'gate-5',
      role: { kind: 'accepted_review_marker' },
      source: source(),
    });

    const terminal = compose(input).epics[0].sprints[0];
    expect(terminal.sprintPlan).toMatchObject({
      currentSprintPlanRevisionId: 'revision-5',
      selectedSprintPlanRevisionId: 'revision-5',
    });
    expect(terminal.revisionViews).toMatchObject([
      {
        sprintPlanRevisionId: 'revision-1',
        isSelected: false,
        workUnits: [{ workUnitId: 'work-unit-1' }],
        gates: [{ gateId: 'gate-1' }],
      },
      {
        sprintPlanRevisionId: 'revision-5',
        isSelected: true,
        plannerActivityGroups: [{ workUnitScopeIds: ['scope-5a', 'scope-5b'] }],
        workUnits: [
          { workUnitId: 'work-unit-5a', presentationState: 'not_started' },
          { workUnitId: 'work-unit-5b', presentationState: 'waiting_for_dependencies' },
        ],
        gates: [{ gateId: 'gate-5', presentationRole: { kind: 'accepted_review_marker' } }],
      },
    ]);
    input.selection = { selectedSprintPlanRevisionIds: { 'sprint-1': 'revision-1' } };
    const historical = compose(input).epics[0].sprints[0];
    expect(historical.sprintPlan.selectedSprintPlanRevisionId).toBe('revision-1');
    expect(withoutSelection(historical.revisionViews)).toEqual(
      withoutSelection(terminal.revisionViews),
    );
  });

  it('composes explicit workspace metadata without treating display relationships as Event facts', () => {
    const input = productInput();
    input.referenceIndex.sprintWorkspacePresentation!.gates[0].role = {
      kind: 'other',
      fallbackLabel: 'Generic review gate',
    };
    input.referenceIndex.sprintWorkspacePresentation!.narratives = [
      {
        sprintId: 'sprint-1',
        progress: { source: { status: 'pending', reason: 'awaiting review evidence' } },
      },
    ];
    input.referenceIndex.sprintWorkspacePresentation!.problems = [
      {
        problemId: 'problem-1',
        sprintId: 'sprint-1',
        title: 'Keep the relationship explicit.',
        source: source(),
        graphElementRefs: [
          { kind: 'sprint_planner_activity', id: 'activity-1' },
          { kind: 'work_unit', id: 'work-unit-1' },
        ],
      },
    ];
    input.referenceIndex.sprintWorkspacePresentation!.epicPlannerObjectives = [
      {
        objectiveId: 'objective-1',
        sprintId: 'sprint-1',
        title: 'Preserve the planned Sprint task.',
        source: source(),
      },
      {
        objectiveId: 'objective-2',
        sprintId: 'sprint-1',
        title: 'Make the review relationship explicit.',
        source: source(),
      },
    ];
    input.referenceIndex.sprintWorkspacePresentation!.workUnitLifecycle = [
      {
        entryId: 'lifecycle-1',
        sprintId: 'sprint-1',
        workUnitId: 'work-unit-1',
        sequence: 0,
        kind: 'work',
        title: 'Work',
        summary: 'Recorded work.',
        agentSessionId: 'session-1',
        agentRole: 'worker',
        invocationId: 'invocation-1',
        source: source(),
      },
    ];

    const presentation = compose(input).epics[0].sprints[0].workspacePresentation;
    expect(presentation.plannerActivityMembership).toEqual([
      expect.objectContaining({ workUnitScopeIds: ['scope-1'] }),
    ]);
    expect(presentation.gates[0]).toMatchObject({
      role: { kind: 'other', fallbackLabel: 'Generic review gate' },
    });
    expect(presentation.narratives?.progress).toEqual({
      source: { status: 'pending', reason: 'awaiting review evidence' },
    });
    expect(presentation.narratives?.progress).not.toHaveProperty('value');
    expect(presentation.problems?.[0].graphElementRefs).toEqual([
      { kind: 'sprint_planner_activity', id: 'activity-1' },
      { kind: 'work_unit', id: 'work-unit-1' },
    ]);
    expect(presentation.epicPlannerObjectives?.map(({ title }) => title)).toEqual([
      'Preserve the planned Sprint task.',
      'Make the review relationship explicit.',
    ]);
    expect(presentation.workUnitLifecycle?.[0]).toMatchObject({
      entryId: 'lifecycle-1',
      agentSessionId: 'session-1',
    });
  });

  it('projects Epic Planner objectives only to their owning Sprint', () => {
    const input = productInput();
    addSecondSprint(input);
    input.referenceIndex.sprintWorkspacePresentation!.epicPlannerObjectives = [
      {
        objectiveId: 'objective-1',
        sprintId: 'sprint-1',
        title: 'First Sprint task.',
        source: source(),
      },
      {
        objectiveId: 'objective-2',
        sprintId: 'sprint-2',
        title: 'Second Sprint task.',
        source: source(),
      },
    ];

    const sprints = compose(input).epics[0].sprints;
    expect(
      sprints[0].workspacePresentation.epicPlannerObjectives?.map(({ objectiveId }) => objectiveId),
    ).toEqual(['objective-1']);
    expect(
      sprints[1].workspacePresentation.epicPlannerObjectives?.map(({ objectiveId }) => objectiveId),
    ).toEqual(['objective-2']);
  });

  it.each([
    [
      'cross-plan Planner Activity membership',
      (input: Mutable<ProductReadCompositionInputV1>) => {
        addSecondSprint(input);
        input.referenceIndex.sprintWorkspacePresentation!.plannerActivityMembership[0] = {
          sprintPlannerActivityId: 'activity-1',
          sprintPlanRevisionId: 'revision-2',
          workUnitScopeIds: ['scope-2'],
          source: source(),
        };
      },
      'same plan',
    ],
    [
      'unknown Work Unit scope membership',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        (input.referenceIndex.sprintWorkspacePresentation!.plannerActivityMembership[0].workUnitScopeIds =
          ['scope-missing']),
      'missing Work Unit scope',
    ],
    [
      'one Work Unit scope assigned to multiple Planner Activities',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        input.referenceIndex.sprintWorkspacePresentation!.plannerActivityMembership.push({
          sprintPlannerActivityId: 'activity-1',
          sprintPlanRevisionId: 'revision-1',
          workUnitScopeIds: ['scope-1'],
          source: source(),
        }),
      'exactly one Planner Activity',
    ],
    [
      'cross-Sprint Document revision link',
      (input: Mutable<ProductReadCompositionInputV1>) => {
        addSecondSprint(input);
        input.referenceIndex.sprintWorkspacePresentation!.documents[0].sprintPlanRevisionIds = [
          'revision-2',
        ];
      },
      'Document Sprint',
    ],
    [
      'Epic Planner objective without an identity',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        (input.referenceIndex.sprintWorkspacePresentation!.epicPlannerObjectives = [
          { objectiveId: ' ', sprintId: 'sprint-1', title: 'Task', source: source() },
        ]),
      'requires an identity',
    ],
    [
      'Epic Planner objective owned by an unknown Sprint',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        (input.referenceIndex.sprintWorkspacePresentation!.epicPlannerObjectives = [
          {
            objectiveId: 'objective-1',
            sprintId: 'sprint-missing',
            title: 'Task',
            source: source(),
          },
        ]),
      'unknown Sprint',
    ],
    [
      'duplicate Epic Planner objective identity',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        (input.referenceIndex.sprintWorkspacePresentation!.epicPlannerObjectives = [
          { objectiveId: 'objective-1', sprintId: 'sprint-1', title: 'First', source: source() },
          { objectiveId: 'objective-1', sprintId: 'sprint-1', title: 'Second', source: source() },
        ]),
      'cannot repeat an objective identity',
    ],
    [
      'Epic Planner objective without sourced authority',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        (input.referenceIndex.sprintWorkspacePresentation!.epicPlannerObjectives = [
          {
            objectiveId: 'objective-1',
            sprintId: 'sprint-1',
            title: 'Task',
            source: { ...source(), sourceReferences: ['missing-provenance'] },
          },
        ]),
      'source must name known facts or provenance',
    ],
    [
      'invented pending narrative value',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        (input.referenceIndex.sprintWorkspacePresentation!.narratives = [
          {
            sprintId: 'sprint-1',
            attention: {
              source: { status: 'unsupported', reason: 'not supplied' },
              value: 'not allowed',
            } as never,
          },
        ]),
      'cannot invent a value',
    ],
  ])('rejects %s at the workspace metadata boundary', (_label, mutate, message) => {
    const input = productInput();
    mutate(input);
    expect(() => compose(input)).toThrow(message);
  });

  it('rejects lifecycle entries that cross Sprint or Agent Session relationships', () => {
    const crossSprint = productInput();
    addSecondSprint(crossSprint);
    crossSprint.referenceIndex.sprintWorkspacePresentation!.workUnitLifecycle = [
      lifecycleEntry({ sprintId: 'sprint-2', workUnitId: 'work-unit-1' }),
    ];
    expect(() => compose(crossSprint)).toThrow('Work Unit must belong to its Sprint');

    const unrelatedSession = productInput();
    unrelatedSession.events.agentSessions.push({ agentSessionId: 'session-2' });
    unrelatedSession.events.agentSessionReferences.push({
      agentSessionRefId: 'session-ref-unrelated',
      agentSessionId: 'session-2',
      targetKind: 'sprint',
      targetId: 'sprint-1',
      semanticRole: 'sprint_runner',
    });
    unrelatedSession.referenceIndex.agentSessions.push({
      agentSessionId: 'session-2',
      title: 'Unrelated session',
      source: source(),
    });
    unrelatedSession.referenceIndex.sprintWorkspacePresentation!.workUnitLifecycle = [
      lifecycleEntry({ agentSessionId: 'session-2' }),
    ];
    expect(() => compose(unrelatedSession)).toThrow(
      'Agent Session must be associated with its Work Unit or owning planner activity and Sprint',
    );
  });

  it('accepts a Sprint Planner lifecycle turn only through exact owning activity membership', () => {
    const input = productInput();
    input.events.agentSessionReferences = input.events.agentSessionReferences.filter(
      ({ targetKind }) => targetKind !== 'work_unit_execution',
    );
    input.referenceIndex.sprintWorkspacePresentation!.workUnitLifecycle = [
      lifecycleEntry({
        kind: 'planning',
        agentRole: 'sprint_planner',
        invocationId: 'planner-scope-turn',
      }),
    ];

    expect(
      compose(input).epics[0].sprints[0].workspacePresentation.workUnitLifecycle?.[0],
    ).toMatchObject({
      kind: 'planning',
      agentRole: 'sprint_planner',
      agentSessionId: 'session-1',
    });

    input.events.sprintPlannerActivities.push({
      sprintPlannerActivityId: 'activity-unrelated',
      sprintPlanId: 'plan-1',
      assessedSprintPlanRevisionIds: ['revision-1'],
    });
    input.referenceIndex.plannerActivities.push({
      sprintPlannerActivityId: 'activity-unrelated',
      title: 'Unrelated planning step',
      purpose: 'Does not own the Work Unit.',
      source: source(),
    });
    input.events.agentSessionReferences.find(
      ({ targetKind }) => targetKind === 'sprint_planner_activity',
    )!.targetId = 'activity-unrelated';
    expect(() => compose(input)).toThrow(
      'Agent Session must be associated with its Work Unit or owning planner activity and Sprint',
    );
  });

  it('rejects duplicate lifecycle sequence numbers within one Work Unit', () => {
    const input = productInput();
    input.referenceIndex.sprintWorkspacePresentation!.workUnitLifecycle = [
      lifecycleEntry({ entryId: 'lifecycle-1' }),
      lifecycleEntry({ entryId: 'lifecycle-2' }),
    ];
    expect(() => compose(input)).toThrow('sequence must be unique within a Work Unit');
  });

  it.each([
    [
      'provider thread identity',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        ((input.events.agentSessions[0] as unknown as Record<string, unknown>).providerThreadId =
          'adapter-only'),
      'providerThreadId',
    ],
    [
      'a raw Document path',
      (input: Mutable<ProductReadCompositionInputV1>) =>
        (input.artifactAccess.documents[0].title = 'C:\\private\\handoff.md'),
      'raw path',
    ],
  ])('rejects %s before it can enter a product read model', (_label, mutate, message) => {
    const input = productInput();
    mutate(input);
    expect(() => compose(input)).toThrow(message);
  });
});

const TIME = '2026-07-15T10:00:00.000Z';
function source() {
  return {
    status: 'available' as const,
    sourceKind: 'orchestration_event' as const,
    sourceReferences: ['provenance-1'],
  };
}
type LifecycleEntry = Mutable<
  NonNullable<ProductSprintWorkspacePresentationMetadataV1['workUnitLifecycle']>
>[number];
function lifecycleEntry(overrides: Partial<LifecycleEntry> = {}): LifecycleEntry {
  return {
    entryId: 'lifecycle-1',
    sprintId: 'sprint-1',
    workUnitId: 'work-unit-1',
    sequence: 1,
    kind: 'work',
    title: 'Work',
    summary: 'Recorded work.',
    agentSessionId: 'session-1',
    agentRole: 'worker',
    invocationId: 'invocation-1',
    source: source(),
    ...overrides,
  };
}
function productInput(): Mutable<ProductReadCompositionInputV1> {
  return {
    events: {
      version: ORCHESTRATION_EVENTS_V1,
      epics: [{ epicId: 'epic-1' }],
      sprints: [{ sprintId: 'sprint-1', epicId: 'epic-1' }],
      sprintPlans: [{ sprintPlanId: 'plan-1', sprintId: 'sprint-1' }],
      sprintPlanRevisions: [
        { sprintPlanRevisionId: 'revision-1', sprintPlanId: 'plan-1', revision: 1 },
      ],
      workUnits: [{ workUnitId: 'work-unit-1' }],
      workUnitScopes: [
        {
          workUnitScopeId: 'scope-1',
          sprintPlanRevisionId: 'revision-1',
          workUnitId: 'work-unit-1',
          dependsOnWorkUnitScopeIds: [],
          gateIds: ['gate-1'],
        },
      ],
      sprintPlannerActivities: [
        {
          sprintPlannerActivityId: 'activity-1',
          sprintPlanId: 'plan-1',
          assessedSprintPlanRevisionIds: ['revision-1'],
        },
      ],
      workUnitExecutions: [
        {
          workUnitExecutionId: 'execution-1',
          workUnitId: 'work-unit-1',
          fixedWorkUnitScopeId: 'scope-1',
        },
      ],
      attempts: [
        {
          attemptId: 'attempt-1',
          workUnitExecutionId: 'execution-1',
          fixedWorkUnitScopeId: 'scope-1',
        },
      ],
      agentSessions: [{ agentSessionId: 'session-1' }],
      agentSessionReferences: [
        {
          agentSessionRefId: 'session-ref-epic',
          agentSessionId: 'session-1',
          targetKind: 'epic',
          targetId: 'epic-1',
          semanticRole: 'epic_runner',
        },
        {
          agentSessionRefId: 'session-ref-sprint',
          agentSessionId: 'session-1',
          targetKind: 'sprint',
          targetId: 'sprint-1',
          semanticRole: 'sprint_runner',
        },
        {
          agentSessionRefId: 'session-ref-worker',
          agentSessionId: 'session-1',
          targetKind: 'work_unit_execution',
          targetId: 'execution-1',
          semanticRole: 'work_unit_worker',
        },
        {
          agentSessionRefId: 'session-ref-planner',
          agentSessionId: 'session-1',
          targetKind: 'sprint_planner_activity',
          targetId: 'activity-1',
          semanticRole: 'sprint_planner',
        },
        {
          agentSessionRefId: 'session-ref-handler',
          agentSessionId: 'session-1',
          targetKind: 'work_unit_execution',
          targetId: 'execution-1',
          semanticRole: 'work_unit_handler',
        },
        {
          agentSessionRefId: 'session-ref-reviewer',
          agentSessionId: 'session-1',
          targetKind: 'work_unit_execution',
          targetId: 'execution-1',
          semanticRole: 'reviewer',
        },
        {
          agentSessionRefId: 'session-ref-other',
          agentSessionId: 'session-1',
          targetKind: 'other',
          targetId: 'future-1',
          semanticRole: 'other',
          otherTargetType: 'future_target',
          otherSemanticRole: 'future_role',
        },
      ],
      gates: [{ gateId: 'gate-1', sprintPlanRevisionId: 'revision-1' }],
      gateCriteriaRevisions: [
        { gateCriteriaRevisionId: 'criteria-1', gateId: 'gate-1', revision: 1 },
      ],
      feedbackRecords: [
        {
          feedbackRecordId: 'feedback-1',
          gateId: 'gate-1',
          boundary: 'designed_feedback_flow',
          provenanceId: 'provenance-1',
        },
      ],
      policyEligibilityFacts: [
        {
          policyEligibilityFactId: 'fact-sprint',
          level: 'sprint',
          targetId: 'sprint-1',
          autoFlowEnabled: true,
          eligible: true,
          provenanceId: 'provenance-1',
        },
        {
          policyEligibilityFactId: 'fact-epic',
          level: 'epic',
          targetId: 'epic-1',
          autoFlowEnabled: true,
          eligible: true,
          provenanceId: 'provenance-1',
        },
      ],
      executionRequests: [
        {
          executionRequestId: 'request-1',
          workUnitExecutionId: 'execution-1',
          provenanceId: 'provenance-1',
        },
      ],
      observedLaunches: [
        {
          observedLaunchId: 'launch-1',
          executionRequestId: 'request-1',
          workUnitExecutionId: 'execution-1',
          attemptId: 'attempt-1',
          provenanceId: 'provenance-1',
        },
      ],
      observedReturns: [
        {
          observedReturnId: 'return-1',
          observedLaunchId: 'launch-1',
          attemptId: 'attempt-1',
          provenanceId: 'provenance-1',
        },
      ],
      reviews: [
        {
          reviewId: 'review-1',
          subjectKind: 'work_unit_execution',
          subjectId: 'execution-1',
          outcome: 'accepted',
          rationaleArtifactId: 'artifact-1',
          provenanceId: 'provenance-1',
        },
        {
          reviewId: 'review-plan-1',
          subjectKind: 'sprint_plan_revision',
          subjectId: 'revision-1',
          outcome: 'needs_correction',
          provenanceId: 'provenance-1',
        },
        {
          reviewId: 'review-document-1',
          subjectKind: 'document_reference',
          subjectId: 'document-1',
          outcome: 'accepted',
          provenanceId: 'provenance-1',
        },
      ],
      observedIntegrations: [
        {
          observedIntegrationId: 'integration-1',
          workUnitExecutionId: 'execution-1',
          provenanceId: 'provenance-1',
        },
      ],
      observedCompletions: [
        {
          observedCompletionId: 'completion-1',
          subjectKind: 'work_unit_execution',
          subjectId: 'execution-1',
          responsibilityAccepted: true,
          provenanceId: 'provenance-1',
        },
      ],
      continuationRequests: [
        {
          continuationRequestId: 'continuation-request-1',
          policyEligibilityFactId: 'fact-sprint',
          targetKind: 'next_work_unit',
          targetId: 'sprint-1',
          provenanceId: 'provenance-1',
        },
      ],
      observedContinuations: [
        {
          observedContinuationId: 'continuation-1',
          continuationRequestId: 'continuation-request-1',
          provenanceId: 'provenance-1',
        },
      ],
      observedHandoffs: [],
      internalArtifacts: [{ artifactId: 'artifact-1', provenanceId: 'provenance-1' }],
      documentReferences: [
        { documentRefId: 'document-1', artifactIds: ['artifact-1'], provenanceId: 'provenance-1' },
      ],
      provenance: [
        {
          provenanceId: 'provenance-1',
          sourceKind: 'application',
          recordedAt: TIME,
          causalFactIds: [],
          actorAgentSessionRefId: 'session-ref-epic',
        },
      ],
    },
    agentControl: {
      version: AGENT_CONTROL_CONTRACTS_V1,
      promptProvenance: [
        {
          promptProvenanceId: 'prompt-1',
          sourceKind: 'application_produced',
          sourceReference: 'product-test',
          causalInputReferences: [],
        },
      ],
      continuationPolicies: [
        {
          continuationPolicyId: 'policy-sprint',
          level: 'sprint',
          sprintId: 'sprint-1',
          autoFlowEnabled: true,
        },
        {
          continuationPolicyId: 'policy-epic',
          level: 'epic',
          epicId: 'epic-1',
          autoFlowEnabled: true,
        },
      ],
      continuationEligibilityEvaluations: [
        {
          continuationEligibilityEvaluationId: 'eligibility-sprint',
          continuationPolicyId: 'policy-sprint',
          level: 'sprint',
          target: { kind: 'next_ready_work_unit_planner', sprintId: 'sprint-1' },
          requiredConditionsSatisfied: true,
          designedForFeedback: false,
          allPendingDevelopmentTechnicallyBlocked: false,
          recordedAt: TIME,
          result: { status: 'eligible' },
        },
        {
          continuationEligibilityEvaluationId: 'eligibility-epic',
          continuationPolicyId: 'policy-epic',
          level: 'epic',
          target: { kind: 'next_sprint_planner', epicId: 'epic-1' },
          requiredConditionsSatisfied: true,
          designedForFeedback: false,
          allPendingDevelopmentTechnicallyBlocked: false,
          recordedAt: TIME,
          result: { status: 'eligible' },
        },
      ],
      commands: [
        command(
          'command-sprint',
          'request_next_ready_work_unit_planner',
          'session-ref-sprint',
          { kind: 'next_ready_work_unit_planner', sprintId: 'sprint-1' },
          'sprint',
          'sprint-1',
          'policy-sprint',
          'eligibility-sprint',
        ),
        command(
          'command-epic',
          'request_next_sprint_planner',
          'session-ref-epic',
          { kind: 'next_sprint_planner', epicId: 'epic-1' },
          'epic',
          'epic-1',
          'policy-epic',
          'eligibility-epic',
        ),
      ],
      results: [
        {
          agentControlResultId: 'result-sprint',
          agentControlCommandId: 'command-sprint',
          state: 'orchestration_event_recorded',
          orchestrationEventReference: 'launch-1',
          recordedAt: TIME,
        },
      ],
    },
    artifactAccess: {
      version: ARTIFACT_ACCESS_CONTRACTS_V1,
      artifacts: [
        {
          artifactId: 'artifact-1',
          kind: 'handoff_note',
          provenanceReference: 'provenance-1',
          relatedFactReferences: ['launch-1'],
        },
      ],
      changedFileReferences: [],
      documents: [
        {
          documentRefId: 'document-1',
          classification: 'handoff_note',
          title: 'Handoff',
          artifactIds: ['artifact-1'],
          changedFileReferenceIds: [],
          provenanceReference: 'provenance-1',
        },
      ],
      requests: [],
      results: [],
    },
    referenceIndex: {
      epics: [
        {
          epicId: 'epic-1',
          title: 'Epic',
          goal: 'Compose read models',
          source: source(),
        },
      ],
      epicOverviews: [
        {
          epicId: 'epic-1',
          currentMovement: { source: source(), value: { kind: 'planning_next_work' } },
          state: { source: source(), value: 'running' },
        },
      ],
      sprints: [
        {
          sprintId: 'sprint-1',
          title: 'Sprint',
          summary: 'Summary',
          details: 'Details',
          source: source(),
        },
      ],
      sprintPlanRevisions: [
        { sprintPlanRevisionId: 'revision-1', summary: 'Revision 1', source: source() },
      ],
      plannerActivities: [
        {
          sprintPlannerActivityId: 'activity-1',
          title: 'Planning',
          purpose: 'Assess plan',
          source: source(),
        },
      ],
      workUnits: [
        {
          workUnitId: 'work-unit-1',
          title: 'Work',
          summary: 'Summary',
          details: 'Details',
          source: source(),
        },
      ],
      gates: [{ gateId: 'gate-1', title: 'Gate', summary: 'Review', source: source() }],
      concerns: [
        {
          concernId: 'concern-1',
          sprintId: 'sprint-1',
          title: 'Concern',
          summary: 'Summary',
          details: 'Details',
          requiredWorkUnitIds: ['work-unit-1'],
          stateAuthority: {
            kind: 'explicit_decision',
            decision: 'accepted',
            provenanceId: 'provenance-1',
          },
          source: source(),
        },
      ],
      agentSessions: [{ agentSessionId: 'session-1', title: 'Neutral session', source: source() }],
      artifactOwnership: [{ artifactId: 'artifact-1', sprintId: 'sprint-1', source: source() }],
      documentOwnership: [{ documentRefId: 'document-1', sprintId: 'sprint-1', source: source() }],
      sprintWorkspacePresentation: {
        plannerActivityMembership: [
          {
            sprintPlannerActivityId: 'activity-1',
            sprintPlanRevisionId: 'revision-1',
            workUnitScopeIds: ['scope-1'],
            source: source(),
          },
        ],
        gates: [
          {
            gateId: 'gate-1',
            role: { kind: 'accepted_review_marker' },
            source: source(),
          },
        ],
        documents: [
          {
            documentRefId: 'document-1',
            displayOrder: 0,
            recordedAt: { source: source(), value: TIME },
            displayCategory: { source: source(), value: 'handoff' },
            sprintPlanRevisionIds: ['revision-1'],
            sprintPlannerActivityIds: ['activity-1'],
            workUnitScopeIds: ['scope-1'],
          },
        ],
        narratives: [
          {
            sprintId: 'sprint-1',
            direction: { source: source(), value: 'Follow the accepted plan.' },
          },
        ],
      },
    },
  } as unknown as Mutable<ProductReadCompositionInputV1>;
}
function compose(input: Mutable<ProductReadCompositionInputV1>) {
  return composeProductOrchestrationReadModels(input as unknown as ProductReadCompositionInputV1);
}
function withoutSelection<T extends { readonly isSelected: boolean }>(views: readonly T[]) {
  return views.map(({ isSelected, ...view }) => {
    void isSelected;
    return view;
  });
}
function addSecondSprint(input: Mutable<ProductReadCompositionInputV1>) {
  input.events.sprints.push({ sprintId: 'sprint-2', epicId: 'epic-1' });
  input.events.sprintPlans.push({ sprintPlanId: 'plan-2', sprintId: 'sprint-2' });
  input.events.sprintPlanRevisions.push({
    sprintPlanRevisionId: 'revision-2',
    sprintPlanId: 'plan-2',
    revision: 1,
  });
  input.events.workUnits.push({ workUnitId: 'work-unit-2' });
  input.events.workUnitScopes.push({
    workUnitScopeId: 'scope-2',
    sprintPlanRevisionId: 'revision-2',
    workUnitId: 'work-unit-2',
    dependsOnWorkUnitScopeIds: [],
    gateIds: [],
  });
  input.events.sprintPlannerActivities.push({
    sprintPlannerActivityId: 'activity-2',
    sprintPlanId: 'plan-2',
    assessedSprintPlanRevisionIds: ['revision-2'],
  });
  input.events.agentSessionReferences.push({
    agentSessionRefId: 'session-ref-sprint-2',
    agentSessionId: 'session-1',
    targetKind: 'sprint',
    targetId: 'sprint-2',
    semanticRole: 'sprint_runner',
  });
  input.events.internalArtifacts.push({ artifactId: 'artifact-2', provenanceId: 'provenance-1' });
  input.events.documentReferences.push({
    documentRefId: 'document-2',
    artifactIds: ['artifact-2'],
    provenanceId: 'provenance-1',
  });
  input.artifactAccess.artifacts.push({
    artifactId: 'artifact-2' as never,
    kind: 'handoff_note',
    provenanceReference: 'provenance-1',
  });
  input.artifactAccess.documents.push({
    documentRefId: 'document-2' as never,
    classification: 'handoff_note',
    title: 'Second handoff',
    artifactIds: ['artifact-2' as never],
    changedFileReferenceIds: [],
    provenanceReference: 'provenance-1',
  });
  input.referenceIndex.sprints.push({
    sprintId: 'sprint-2',
    title: 'Sprint two',
    summary: 'Second summary',
    details: 'Second details',
    source: source(),
  });
  input.referenceIndex.sprintPlanRevisions.push({
    sprintPlanRevisionId: 'revision-2',
    summary: 'Revision 2',
    source: source(),
  });
  input.referenceIndex.plannerActivities.push({
    sprintPlannerActivityId: 'activity-2',
    title: 'Second planning',
    purpose: 'Assess second plan',
    source: source(),
  });
  input.referenceIndex.workUnits.push({
    workUnitId: 'work-unit-2',
    title: 'Second work',
    summary: 'Second work summary',
    details: 'Second work details',
    source: source(),
  });
  input.referenceIndex.artifactOwnership.push({
    artifactId: 'artifact-2',
    sprintId: 'sprint-2',
    source: source(),
  });
  input.referenceIndex.documentOwnership.push({
    documentRefId: 'document-2',
    sprintId: 'sprint-2',
    source: source(),
  });
  input.referenceIndex.sprintWorkspacePresentation!.plannerActivityMembership.push({
    sprintPlannerActivityId: 'activity-2',
    sprintPlanRevisionId: 'revision-2',
    workUnitScopeIds: ['scope-2'],
    source: source(),
  });
  input.referenceIndex.sprintWorkspacePresentation!.documents.push({
    documentRefId: 'document-2',
    displayOrder: 1,
    recordedAt: { source: source(), value: TIME },
    displayCategory: { source: source(), value: 'handoff' },
    sprintPlanRevisionIds: ['revision-2'],
    sprintPlannerActivityIds: ['activity-2'],
    workUnitScopeIds: ['scope-2'],
  });
}
function command(
  id: string,
  commandKind: 'request_next_ready_work_unit_planner' | 'request_next_sprint_planner',
  recipientAgentSessionRefId: string,
  target:
    | { kind: 'next_ready_work_unit_planner'; sprintId: string }
    | { kind: 'next_sprint_planner'; epicId: string },
  scopeKind: 'sprint' | 'epic',
  scopeId: string,
  policyId: string,
  eligibilityEvaluationId: string,
) {
  return {
    agentControlCommandId: id,
    commandKind,
    recipientAgentSessionRefId,
    target,
    idempotency: { key: id, scopeKind, scopeId },
    initiatedBy: { sourceKind: 'application_produced' as const, sourceReference: id },
    promptProvenanceId: 'prompt-1',
    recordedAt: TIME,
    preconditionEvidenceReference: eligibilityEvaluationId,
    continuation: { policyId, eligibilityEvaluationId },
  };
}
type Mutable<T> = T extends readonly (infer Item)[]
  ? Mutable<Item>[]
  : T extends object
    ? { -readonly [Key in keyof T]: Mutable<T[Key]> }
    : T;
