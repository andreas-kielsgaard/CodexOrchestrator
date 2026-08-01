import {
  decodeOrchestrationEventsV1,
  deriveWorkUnitObservedEvidence,
  ORCHESTRATION_EVENTS_V1,
  type OrchestrationEventsV1,
} from './index';

describe('Orchestration events', () => {
  it('accepts a single revision and an ordered multi-revision chain', () => {
    const single = validFacts();
    single.sprintPlanRevisions = [single.sprintPlanRevisions[0]];
    single.workUnits = single.workUnits.filter(
      ({ workUnitId }) => workUnitId !== 'work-unit-future',
    );
    single.workUnitScopes = single.workUnitScopes.filter(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === 'plan-revision-1',
    );
    single.sprintPlannerActivities[0].assessedSprintPlanRevisionIds = ['plan-revision-1'];
    expect(decodeOrchestrationEventsV1(single).sprintPlanRevisions).toHaveLength(1);
    expect(decodeOrchestrationEventsV1(validFacts()).sprintPlanRevisions).toHaveLength(2);
  });

  it.each([
    [
      'multiple roots',
      (facts: Mutable<OrchestrationEventsV1>) =>
        delete facts.sprintPlanRevisions[1].supersedesSprintPlanRevisionId,
      'exactly one revision root',
    ],
    [
      'branching successors',
      (facts: Mutable<OrchestrationEventsV1>) =>
        facts.sprintPlanRevisions.push({
          sprintPlanRevisionId: 'plan-revision-3',
          sprintPlanId: 'sprint-plan-1',
          revision: 3,
          supersedesSprintPlanRevisionId: 'plan-revision-1',
        }),
      'at most one direct successor',
    ],
    [
      'disconnected lineage',
      (facts: Mutable<OrchestrationEventsV1>) =>
        facts.sprintPlanRevisions.push({
          sprintPlanRevisionId: 'plan-revision-3',
          sprintPlanId: 'sprint-plan-1',
          revision: 3,
        }),
      'exactly one revision root',
    ],
    [
      'cycles',
      (facts: Mutable<OrchestrationEventsV1>) =>
        (facts.sprintPlanRevisions[0].supersedesSprintPlanRevisionId = 'plan-revision-2'),
      'invalid sprint plan revision supersession chain',
    ],
    [
      'reverse numbering',
      (facts: Mutable<OrchestrationEventsV1>) => (facts.sprintPlanRevisions[0].revision = 3),
      'numbers must increase along supersession',
    ],
  ])('rejects %s revision lineage', (_name, mutate, message) => {
    const facts = validFacts();
    mutate(facts);
    expect(() => decodeOrchestrationEventsV1(facts)).toThrow(message);
  });

  it('rejects cross-plan supersession', () => {
    const facts = validFacts();
    facts.sprints.push({ sprintId: 'sprint-2', epicId: 'epic-1' });
    facts.sprintPlans.push({ sprintPlanId: 'sprint-plan-2', sprintId: 'sprint-2' });
    facts.sprintPlanRevisions.push({
      sprintPlanRevisionId: 'plan-revision-3',
      sprintPlanId: 'sprint-plan-2',
      revision: 1,
    });
    facts.sprintPlanRevisions[1].supersedesSprintPlanRevisionId = 'plan-revision-3';
    expect(() => decodeOrchestrationEventsV1(facts)).toThrow(
      'a revision may supersede only a revision of its own sprint plan',
    );
  });

  it('retains observed history on a superseded revision while leaving unstarted future work without an execution', () => {
    const rawFacts = validFacts();
    rawFacts.workUnitScopes.push({
      workUnitScopeId: 'scope-r2-observed',
      sprintPlanRevisionId: 'plan-revision-2',
      workUnitId: 'work-unit-observed',
      dependsOnWorkUnitScopeIds: [],
      gateIds: [],
    });
    const facts = decodeOrchestrationEventsV1(rawFacts);

    expect(facts.workUnitExecutions).toEqual([
      expect.objectContaining({
        workUnitExecutionId: 'execution-observed',
        fixedWorkUnitScopeId: 'scope-r1-observed',
      }),
    ]);
    expect(
      facts.workUnitExecutions.some(
        ({ fixedWorkUnitScopeId }) => fixedWorkUnitScopeId === 'scope-r1-unstarted',
      ),
    ).toBe(false);
    expect(deriveWorkUnitObservedEvidence(facts, 'work-unit-observed')).toMatchObject({
      plannedScopeIds: ['scope-r1-observed', 'scope-r2-observed'],
      executionRequested: true,
      observedLaunched: true,
      observedReturned: true,
      observedReviewed: true,
      observedIntegrated: true,
      responsibilityAccepted: true,
    });
  });

  it('rejects identity collapse even when a one-to-one scenario makes it convenient', () => {
    const facts = validFacts();
    facts.sprintPlans[0].sprintPlanId = facts.sprints[0].sprintId;

    expect(() => decodeOrchestrationEventsV1(facts)).toThrow(
      'identity sprint-1 must not be shared by convenience',
    );
  });

  it('rejects an attempt that retargets an instantiated execution to revised scope', () => {
    const facts = validFacts();
    facts.attempts[0].fixedWorkUnitScopeId = 'scope-r2-future';

    expect(() => decodeOrchestrationEventsV1(facts)).toThrow(
      'attempt scope must equal its execution fixed scope',
    );
  });

  it('retains requested-only history on a superseded scope without deriving observed work', () => {
    const facts = validFacts();
    facts.workUnitExecutions.push({
      workUnitExecutionId: 'execution-requested-superseded',
      workUnitId: 'work-unit-unstarted',
      fixedWorkUnitScopeId: 'scope-r1-unstarted',
    });
    facts.executionRequests.push({
      executionRequestId: 'request-requested-superseded',
      workUnitExecutionId: 'execution-requested-superseded',
      provenanceId: 'provenance-12',
    });

    const decoded = decodeOrchestrationEventsV1(facts);
    expect(decoded.workUnitExecutions).toContainEqual(
      expect.objectContaining({ fixedWorkUnitScopeId: 'scope-r1-unstarted' }),
    );
    expect(deriveWorkUnitObservedEvidence(decoded, 'work-unit-unstarted')).toEqual({
      workUnitId: 'work-unit-unstarted',
      plannedScopeIds: ['scope-r1-unstarted'],
      executionRequested: true,
      observedLaunched: false,
      observedReturned: false,
      observedReviewed: false,
      observedIntegrated: false,
      responsibilityAccepted: false,
    });
  });

  it('rejects a bare execution record for work left in a superseded revision', () => {
    const facts = validFacts();
    facts.workUnitExecutions.push({
      workUnitExecutionId: 'execution-unstarted',
      workUnitId: 'work-unit-unstarted',
      fixedWorkUnitScopeId: 'scope-r1-unstarted',
    });

    expect(() => decodeOrchestrationEventsV1(facts)).toThrow(
      'a superseded scope may retain only explicit request or observed execution history',
    );
  });

  it.each([
    ['sprint', 'next_work_unit', 'sprint-2'],
    ['epic', 'next_sprint_planner', 'epic-2'],
  ] as const)(
    'rejects a %s continuation request targeting a different policy owner',
    (level, targetKind, targetId) => {
      const facts = validFacts();
      addSecondEpic(facts);
      if (level === 'epic') {
        facts.policyEligibilityFacts.push({
          policyEligibilityFactId: 'policy-epic-1',
          level: 'epic',
          targetId: 'epic-1',
          autoFlowEnabled: true,
          eligible: true,
          provenanceId: 'provenance-12',
        });
        facts.continuationRequests[0].policyEligibilityFactId = 'policy-epic-1';
      }
      facts.continuationRequests[0].targetKind = targetKind;
      facts.continuationRequests[0].targetId = targetId;

      expect(() => decodeOrchestrationEventsV1(facts)).toThrow(
        'continuation request target must equal its policy target',
      );
    },
  );

  it('does not turn a planned or requested execution into an observed launch, return, integration, or completion', () => {
    const facts = validFacts();
    facts.workUnitExecutions.push({
      workUnitExecutionId: 'execution-requested-only',
      workUnitId: 'work-unit-future',
      fixedWorkUnitScopeId: 'scope-r2-future',
    });
    facts.executionRequests.push({
      executionRequestId: 'request-requested-only',
      workUnitExecutionId: 'execution-requested-only',
      provenanceId: 'provenance-12',
    });

    const decoded = decodeOrchestrationEventsV1(facts);
    expect(deriveWorkUnitObservedEvidence(decoded, 'work-unit-future')).toEqual({
      workUnitId: 'work-unit-future',
      plannedScopeIds: ['scope-r2-future'],
      executionRequested: true,
      observedLaunched: false,
      observedReturned: false,
      observedReviewed: false,
      observedIntegrated: false,
      responsibilityAccepted: false,
    });
  });

  it('associates neutral Agent Sessions with roots, planners, handlers, and extensible participants', () => {
    const events = validFacts();
    events.agentSessionReferences.push(
      {
        agentSessionRefId: 'agent-session-reference-epic',
        agentSessionId: 'agent-session-1',
        targetKind: 'epic',
        targetId: 'epic-1',
        semanticRole: 'epic',
      },
      {
        agentSessionRefId: 'agent-session-reference-builder',
        agentSessionId: 'agent-session-1',
        targetKind: 'epic',
        targetId: 'epic-1',
        semanticRole: 'epic',
      },
      {
        agentSessionRefId: 'agent-session-reference-sprint',
        agentSessionId: 'agent-session-1',
        targetKind: 'sprint',
        targetId: 'sprint-1',
        semanticRole: 'sprint',
      },
      {
        agentSessionRefId: 'agent-session-reference-work-unit-planner',
        agentSessionId: 'agent-session-1',
        targetKind: 'sprint_planner_activity',
        targetId: 'planner-activity-1',
        semanticRole: 'work_slice_planner',
      },
      {
        agentSessionRefId: 'agent-session-reference-handler',
        agentSessionId: 'agent-session-1',
        targetKind: 'work_unit_execution',
        targetId: 'execution-observed',
        semanticRole: 'work_unit_handler',
      },
      {
        agentSessionRefId: 'agent-session-reference-implementer',
        agentSessionId: 'agent-session-1',
        targetKind: 'work_unit_execution',
        targetId: 'execution-observed',
        semanticRole: 'work_unit_implementer',
      },
    );

    expect(() => decodeOrchestrationEventsV1(events)).not.toThrow();
  });

  it('rejects an Agent Session role that does not match its association target', () => {
    const events = validFacts();
    events.agentSessionReferences[0].semanticRole = 'sprint';

    expect(() => decodeOrchestrationEventsV1(events)).toThrow(
      'agent session reference role must match its association target',
    );
  });

  it('accepts only the three feedback boundaries and rejects presentation or provider fields', () => {
    const withInvalidFeedback = validFacts();
    (withInvalidFeedback.feedbackRecords[0] as { boundary: string }).boundary =
      'routine_progression';
    expect(() => decodeOrchestrationEventsV1(withInvalidFeedback)).toThrow(
      'feedback boundary is invalid',
    );

    const withDisplayField = validFacts() as Record<string, unknown>;
    withDisplayField.displayLabel = 'Completed';
    expect(() => decodeOrchestrationEventsV1(withDisplayField)).toThrow(
      'displayLabel is presentation, provider, or persistence data rather than an Orchestration Event',
    );

    const withProviderField = validFacts() as Record<string, unknown>;
    withProviderField.providerThreadId = 'adapter-only';
    expect(() => decodeOrchestrationEventsV1(withProviderField)).toThrow(
      'providerThreadId is presentation, provider, or persistence data rather than an Orchestration Event',
    );
  });
});

function validFacts(): Mutable<OrchestrationEventsV1> {
  return {
    version: ORCHESTRATION_EVENTS_V1,
    epics: [{ epicId: 'epic-1' }],
    sprints: [{ sprintId: 'sprint-1', epicId: 'epic-1' }],
    sprintPlans: [{ sprintPlanId: 'sprint-plan-1', sprintId: 'sprint-1' }],
    sprintPlanRevisions: [
      { sprintPlanRevisionId: 'plan-revision-1', sprintPlanId: 'sprint-plan-1', revision: 1 },
      {
        sprintPlanRevisionId: 'plan-revision-2',
        sprintPlanId: 'sprint-plan-1',
        revision: 2,
        supersedesSprintPlanRevisionId: 'plan-revision-1',
      },
    ],
    workUnits: [
      { workUnitId: 'work-unit-observed' },
      { workUnitId: 'work-unit-unstarted' },
      { workUnitId: 'work-unit-future' },
    ],
    workUnitScopes: [
      {
        workUnitScopeId: 'scope-r1-observed',
        sprintPlanRevisionId: 'plan-revision-1',
        workUnitId: 'work-unit-observed',
        dependsOnWorkUnitScopeIds: [],
        gateIds: ['gate-1'],
      },
      {
        workUnitScopeId: 'scope-r1-unstarted',
        sprintPlanRevisionId: 'plan-revision-1',
        workUnitId: 'work-unit-unstarted',
        dependsOnWorkUnitScopeIds: ['scope-r1-observed'],
        gateIds: [],
      },
      {
        workUnitScopeId: 'scope-r2-future',
        sprintPlanRevisionId: 'plan-revision-2',
        workUnitId: 'work-unit-future',
        dependsOnWorkUnitScopeIds: [],
        gateIds: [],
      },
    ],
    sprintPlannerActivities: [
      {
        sprintPlannerActivityId: 'planner-activity-1',
        sprintPlanId: 'sprint-plan-1',
        assessedSprintPlanRevisionIds: ['plan-revision-1', 'plan-revision-2'],
      },
    ],
    workUnitExecutions: [
      {
        workUnitExecutionId: 'execution-observed',
        workUnitId: 'work-unit-observed',
        fixedWorkUnitScopeId: 'scope-r1-observed',
      },
    ],
    attempts: [
      {
        attemptId: 'attempt-observed',
        workUnitExecutionId: 'execution-observed',
        fixedWorkUnitScopeId: 'scope-r1-observed',
      },
    ],
    agentSessions: [{ agentSessionId: 'agent-session-1' }],
    agentSessionReferences: [
      {
        agentSessionRefId: 'agent-session-reference-1',
        agentSessionId: 'agent-session-1',
        targetKind: 'sprint_planner_activity',
        targetId: 'planner-activity-1',
        semanticRole: 'work_slice_planner',
      },
    ],
    gates: [{ gateId: 'gate-1', sprintPlanRevisionId: 'plan-revision-1' }],
    gateCriteriaRevisions: [
      { gateCriteriaRevisionId: 'gate-criteria-1', gateId: 'gate-1', revision: 1 },
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
        policyEligibilityFactId: 'policy-1',
        level: 'sprint',
        targetId: 'sprint-1',
        autoFlowEnabled: true,
        eligible: true,
        provenanceId: 'provenance-2',
      },
    ],
    executionRequests: [
      {
        executionRequestId: 'request-observed',
        workUnitExecutionId: 'execution-observed',
        provenanceId: 'provenance-3',
      },
    ],
    observedLaunches: [
      {
        observedLaunchId: 'launch-observed',
        executionRequestId: 'request-observed',
        workUnitExecutionId: 'execution-observed',
        attemptId: 'attempt-observed',
        provenanceId: 'provenance-4',
      },
    ],
    observedReturns: [
      {
        observedReturnId: 'return-observed',
        observedLaunchId: 'launch-observed',
        attemptId: 'attempt-observed',
        provenanceId: 'provenance-5',
      },
    ],
    reviews: [
      {
        reviewId: 'review-1',
        subjectKind: 'work_unit_execution',
        subjectId: 'execution-observed',
        outcome: 'accepted',
        rationaleArtifactId: 'artifact-1',
        provenanceId: 'provenance-6',
      },
    ],
    observedIntegrations: [
      {
        observedIntegrationId: 'integration-1',
        workUnitExecutionId: 'execution-observed',
        provenanceId: 'provenance-7',
      },
    ],
    observedCompletions: [
      {
        observedCompletionId: 'completion-1',
        subjectKind: 'work_unit_execution',
        subjectId: 'execution-observed',
        responsibilityAccepted: true,
        provenanceId: 'provenance-8',
      },
    ],
    continuationRequests: [
      {
        continuationRequestId: 'continuation-request-1',
        policyEligibilityFactId: 'policy-1',
        targetKind: 'next_work_unit',
        targetId: 'sprint-1',
        provenanceId: 'provenance-9',
      },
    ],
    observedContinuations: [
      {
        observedContinuationId: 'continuation-observed-1',
        continuationRequestId: 'continuation-request-1',
        provenanceId: 'provenance-10',
      },
    ],
    observedHandoffs: [],
    internalArtifacts: [{ artifactId: 'artifact-1', provenanceId: 'provenance-11' }],
    documentReferences: [
      {
        documentRefId: 'document-reference-1',
        artifactIds: ['artifact-1'],
        provenanceId: 'provenance-11',
      },
    ],
    provenance: Array.from({ length: 12 }, (_, index) => provenance(`provenance-${index + 1}`)),
  };
}

function provenance(provenanceId: string) {
  return {
    provenanceId,
    sourceKind: 'agent_session' as const,
    recordedAt: '2026-07-14T12:00:00.000Z',
    causalFactIds: [],
    actorAgentSessionRefId: 'agent-session-reference-1',
  };
}

function addSecondEpic(facts: Mutable<OrchestrationEventsV1>): void {
  facts.epics.push({ epicId: 'epic-2' });
  facts.sprints.push({ sprintId: 'sprint-2', epicId: 'epic-2' });
  facts.sprintPlans.push({ sprintPlanId: 'sprint-plan-2', sprintId: 'sprint-2' });
  facts.sprintPlanRevisions.push({
    sprintPlanRevisionId: 'plan-revision-3',
    sprintPlanId: 'sprint-plan-2',
    revision: 1,
  });
}

type Mutable<T> = T extends readonly (infer Item)[]
  ? Mutable<Item>[]
  : T extends object
    ? { -readonly [Key in keyof T]: Mutable<T[Key]> }
    : T;
