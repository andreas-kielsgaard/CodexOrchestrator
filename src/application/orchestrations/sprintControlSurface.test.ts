import {
  decodeSprintExecutionSnapshotV1,
  decodeSprintPlannerOutputV1,
  deriveConcernState,
  projectSprintControlSurface,
} from './sprintControlSurface';
import type {
  SprintExecutionSnapshotV1,
  SprintPlannerOutputV1,
  WorkUnitPresentationState,
} from './sprintControlSurface';

describe('Sprint control-surface read boundary', () => {
  it('accepts a single revision and an ordered multi-revision chain', () => {
    const single = mutablePlanner();
    single.planRevisions = [single.planRevisions[0]];
    single.sprintPlannerActivities = [single.sprintPlannerActivities[0]];
    single.planChanges = [];
    single.workUnits = [
      {
        ...single.workUnits[0],
        specRevisions: [single.workUnits[0].specRevisions[0]],
      },
    ];
    single.concerns[0].requiredWorkUnitIds = ['work-unit-1'];
    single.gates = [];
    single.documents = [];

    expect(decodeSprintPlannerOutputV1(single).planRevisions).toHaveLength(1);
    expect(decodeSprintPlannerOutputV1(plannerOutput()).planRevisions).toHaveLength(2);
  });

  it.each<[string, PlannerMutator, string]>([
    [
      'multiple roots',
      (planner) => delete planner.planRevisions[1].supersedesPlanRevisionId,
      'exactly one revision root',
    ],
    [
      'branching successors',
      (planner) =>
        planner.planRevisions.push({
          ...planner.planRevisions[1],
          id: 'sprint-plan-revision-3',
          revision: 3,
          supersedesPlanRevisionId: 'sprint-plan-revision-1',
        }),
      'at most one direct successor',
    ],
    [
      'disconnected lineage',
      (planner) =>
        planner.planRevisions.push({
          ...planner.planRevisions[1],
          id: 'sprint-plan-revision-3',
          revision: 3,
          supersedesPlanRevisionId: undefined,
        }),
      'exactly one revision root',
    ],
    [
      'cycles',
      (planner) => (planner.planRevisions[0].supersedesPlanRevisionId = 'sprint-plan-revision-2'),
      'invalid supersession chain',
    ],
    [
      'reverse numbering',
      (planner) => (planner.planRevisions[0].revision = 3),
      'numbers must increase along supersession',
    ],
  ])('rejects %s revision lineage', (_name, mutate, message) => {
    const planner = mutablePlanner();
    mutate(planner);
    expect(() => decodeSprintPlannerOutputV1(planner)).toThrow(message);
  });

  it('rejects a dangling supersession reference', () => {
    const planner = mutablePlanner();
    planner.planRevisions[1].supersedesPlanRevisionId = 'missing-plan-revision';
    expect(() => decodeSprintPlannerOutputV1(planner)).toThrow(
      'dangling superseded revision reference',
    );
  });

  it('decodes revisions, Work Units, concerns, Documents, and observed execution history', () => {
    const planner = plannerOutput();
    const snapshot = mutableSnapshot();
    expect(decodeSprintPlannerOutputV1(planner)).toEqual(planner);
    expect(decodeSprintExecutionSnapshotV1(snapshot, planner)).toEqual(snapshot);
    const projection = projectSprintControlSurface(planner, snapshot);
    expect(projection.workUnits.find(({ id }) => id === 'work-unit-1')).toMatchObject({
      executionState: 'accepted',
      presentationState: 'completed',
      journey: { attempts: 1, accepted: true, launched: true },
    });
    expect(projection.workUnits.find(({ id }) => id === 'work-unit-2')).toMatchObject({
      executionState: 'projected',
      presentationState: 'not_started',
      journey: { attempts: 0, accepted: false, launched: false },
    });
    expect(projection.documents.map(({ id }) => id)).toEqual([
      'generated-document-1',
      'document-1',
    ]);
  });

  it('projects either revision without borrowing later Work Units or specifications', () => {
    const planner = plannerOutput();
    const snapshot = mutableSnapshot();
    const first = projectSprintControlSurface(planner, snapshot, 'sprint-plan-revision-1');
    const second = projectSprintControlSurface(planner, snapshot, 'sprint-plan-revision-2');
    expect(first.workUnits.map(({ id }) => id)).toEqual(['work-unit-1']);
    expect(first.workUnits.map(({ id }) => id)).not.toContain('work-unit-2');
    expect(second.workUnits.map(({ id }) => id)).toEqual(['work-unit-1', 'work-unit-2']);
    expect(second.revisionGraph).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'sprint-plan-revision-1', isSelected: false }),
        expect.objectContaining({
          id: 'sprint-plan-revision-2',
          isSelected: true,
          isActive: true,
        }),
      ]),
    );
  });

  it('keeps Sprint Plan, Revision, Sprint Planner Activity, Work Unit, and Agent Session identities independent', () => {
    const read = projectSprintControlSurface(plannerOutput(), executionSnapshot()).readModel;
    const ids = [
      read.sprintPlan.sprintPlanId,
      read.sprintPlanRevisions[0].sprintPlanRevisionId,
      read.sprintPlannerActivities[0].sprintPlannerActivityId,
      read.workUnits[0].workUnitId,
      read.agentSessionReferences[0].agentSessionRefId,
    ];
    expect(new Set(ids)).toHaveLength(ids.length);
    expect(
      read.sprintPlanRevisions.every(
        ({ sprintPlanId }) => sprintPlanId === read.sprintPlan.sprintPlanId,
      ),
    ).toBe(true);
    expect(read.sprintPlan.sprintPlanId).not.toBe(
      read.sprintPlannerActivities[0].sprintPlannerActivityId,
    );
  });

  it('projects explicit Sprint Plan relationships without cloning Sprint Planner Activities into Plan identity', () => {
    const projection = projectSprintControlSurface(plannerOutput(), executionSnapshot());
    expect(projection.readModel.sprintPlan).toEqual({
      sprintPlanId: 'sprint-plan-1',
      sprintId: 'sprint-1',
    });
    expect(projection.mapLayout.nodes).toContainEqual(
      expect.objectContaining({ id: 'sprint_plan:sprint-plan-1', type: 'sprint_plan' }),
    );
    expect(projection.mapLayout.edges).toContainEqual(
      expect.objectContaining({
        from: 'sprint_plan:sprint-plan-1',
        to: 'plan_revision:sprint-plan-revision-2',
        kind: 'revision',
      }),
    );
    expect(projection).not.toHaveProperty('plans');
    expect(projection.sprintPlannerActivityGroups).toBe(projection.sprintPlannerActivities);
  });

  it('does not infer request, launch, or accepted completion from planned compatibility facts', () => {
    const read = projectSprintControlSurface(
      plannerOutput(),
      executionSnapshot(),
    ).readModel.workUnits.find(({ workUnitId }) => workUnitId === 'work-unit-2');
    expect(read).toMatchObject({
      executionRequestObserved: false,
      launchObserved: false,
      responsibilityAccepted: false,
      presentationState: 'not_started',
    });
    expect(read?.fixedExecutionScopeId).toBeUndefined();
  });

  it('keeps Sprint and Epic continuation separate and requires observed initiation', () => {
    const snapshot = mutableSnapshot();
    snapshot.continuation.sprint = {
      automaticEnabled: true,
      status: 'continuation_requested',
      initiationObserved: false,
    };
    const continuation = projectSprintControlSurface(plannerOutput(), snapshot).readModel
      .continuation;
    expect(continuation.sprint).toEqual({
      automaticEnabled: true,
      status: 'continuation_requested',
      initiationObserved: false,
    });
    expect(continuation.epic).toEqual({
      automaticEnabled: false,
      status: 'ready_for_manual',
      initiationObserved: false,
    });
  });

  it('validates active Sprint Plan ownership and a conversation-produced direct plan change', () => {
    const projection = projectSprintControlSurface(plannerOutput(), executionSnapshot());
    expect(projection.activePlanRevisionId).toBe('sprint-plan-revision-2');
    expect(projection.planChanges).toEqual([
      expect.objectContaining({
        source: 'sprint_conversation',
        priorPlanRevisionId: 'sprint-plan-revision-1',
        resultingPlanRevisionId: 'sprint-plan-revision-2',
        priorSprintPlannerActivityId: 'sprint-planner-activity-1',
        resultingSprintPlannerActivityId: 'sprint-planner-activity-2',
      }),
    ]);
    expect(projection.sprintPlannerActivityGroups.map(({ id }) => id)).toEqual([
      'sprint-planner-activity-2',
    ]);
  });

  it('projects concern, Document provenance, Work Unit detail, and semantic map inputs', () => {
    const projection = projectSprintControlSurface(plannerOutput(), executionSnapshot());
    expect(projection.concerns[0]).toMatchObject({
      requiredWorkUnitIds: ['work-unit-1', 'work-unit-2'],
      state: 'waiting_for_dependencies',
    });
    expect(projection.documents).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'document-1', provenance: 'planner' }),
        expect.objectContaining({
          id: 'generated-document-1',
          provenance: 'execution',
          sourceDocumentId: 'document-1',
        }),
      ]),
    );
    expect(projection.mapLayout.nodes).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: 'sprint_planner_activity:sprint-planner-activity-2',
          type: 'sprint_planner_activity',
        }),
        expect.objectContaining({ id: 'work_unit:work-unit-2', type: 'work_unit' }),
        expect.objectContaining({ id: 'gate:gate-1', type: 'gate' }),
      ]),
    );
    expect(projection.mapLayout.edges).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          from: 'work_unit:work-unit-1',
          to: 'work_unit:work-unit-2',
          kind: 'dependency',
          dependencyKind: 'hard',
        }),
        expect.objectContaining({
          from: 'work_unit:work-unit-1',
          to: 'gate:gate-1',
          kind: 'gate',
        }),
      ]),
    );
  });

  it('derives concern precedence and mixed-progress waiting states in reusable projection logic', () => {
    const concern = plannerOutput().concerns[0];
    const derive = (
      states: readonly (readonly [string, WorkUnitPresentationState])[],
      decision?: SprintExecutionSnapshotV1['concernDecisions'][number],
    ) => deriveConcernState(concern, decision, new Map(states));

    expect(
      derive([
        ['work-unit-1', 'not_started'],
        ['work-unit-2', 'not_started'],
      ]),
    ).toBe('not_started');
    expect(
      derive([
        ['work-unit-1', 'completed'],
        ['work-unit-2', 'not_started'],
      ]),
    ).toBe('waiting_for_dependencies');
    expect(
      derive([
        ['work-unit-1', 'working'],
        ['work-unit-2', 'blocked'],
      ]),
    ).toBe('blocked');
    expect(
      derive([
        ['work-unit-1', 'working'],
        ['work-unit-2', 'under_review'],
      ]),
    ).toBe('working');
    expect(
      derive([
        ['work-unit-1', 'under_review'],
        ['work-unit-2', 'completed'],
      ]),
    ).toBe('under_review');
    expect(
      derive([
        ['work-unit-1', 'completed'],
        ['work-unit-2', 'completed'],
      ]),
    ).toBe('completed');
    expect(
      derive(
        [
          ['work-unit-1', 'blocked'],
          ['work-unit-2', 'blocked'],
        ],
        { concernId: concern.id, kind: 'accepted', summary: 'Explicitly accepted.' },
      ),
    ).toBe('completed');
    expect(
      derive(
        [
          ['work-unit-1', 'completed'],
          ['work-unit-2', 'completed'],
        ],
        { concernId: concern.id, kind: 'deferred', summary: 'Explicitly deferred.' },
      ),
    ).toBe('deferred');
  });

  it('keeps illustrative concern coverage separate from canonical execution facts', () => {
    const concern = plannerOutput().concerns[0];
    const states = new Set([
      deriveConcernState(concern, undefined, new Map()),
      deriveConcernState(concern, undefined, new Map([['work-unit-1', 'blocked']])),
      deriveConcernState(concern, undefined, new Map([['work-unit-1', 'working']])),
      deriveConcernState(concern, undefined, new Map([['work-unit-1', 'under_review']])),
      deriveConcernState(
        concern,
        { concernId: concern.id, kind: 'deferred', summary: 'Demo.' },
        new Map(),
      ),
    ]);
    expect(states).toEqual(
      new Set(['not_started', 'blocked', 'working', 'under_review', 'deferred']),
    );
    expect(executionSnapshot().concernDecisions).toEqual([]);
  });

  it('derives concern deferral and does not complete it after one accepted linked Work Unit', () => {
    const planner = plannerOutput();
    const snapshot = mutableSnapshot();
    expect(projectSprintControlSurface(planner, snapshot).concerns[0].state).not.toBe('completed');
    snapshot.concernDecisions = [
      { concernId: 'concern-1', kind: 'deferred', summary: 'Awaiting user choice.' },
    ];
    expect(projectSprintControlSurface(planner, snapshot).concerns[0].state).toBe('deferred');
    snapshot.concernDecisions = [
      { concernId: 'concern-1', kind: 'accepted', summary: 'Accepted explicitly.' },
    ];
    expect(projectSprintControlSurface(planner, snapshot).concerns[0].state).toBe('completed');
  });

  it('supports a declared parallel group in a separate valid synthetic planner fixture', () => {
    const planner = mutablePlanner();
    planner.parallelGroups = [
      {
        id: 'parallel-1',
        rationale: 'Independent recorded work.',
        planRevisionId: 'sprint-plan-revision-2',
        workUnitIds: ['work-unit-1', 'work-unit-2'],
      },
    ];
    planner.workUnits.forEach((unit) => (unit.parallelGroupId = 'parallel-1'));
    expect(decodeSprintPlannerOutputV1(planner)).toEqual(planner);
  });

  it.each<[string, PlannerMutator]>([
    ['missing title', (planner) => (planner.sprint.title = '')],
    [
      'invalid gate discriminant',
      (planner) => ((planner.gates[0] as { kind: string }).kind = 'wrong'),
    ],
    ['duplicate revision number', (planner) => (planner.planRevisions[1].revision = 1)],
    ['undeclared parallel group', (planner) => (planner.workUnits[0].parallelGroupId = 'missing')],
    [
      'parallel member outside its revision',
      (planner) => {
        planner.parallelGroups = [
          {
            id: 'parallel-1',
            rationale: 'Invalid cross-revision membership.',
            planRevisionId: 'sprint-plan-revision-1',
            workUnitIds: ['work-unit-2'],
          },
        ];
      },
    ],
    [
      'illegal dependency gate',
      (planner) =>
        (planner.workUnits[1].dependencies = [
          { workUnitId: 'work-unit-1', kind: 'hard', gateId: 'gate-1' },
        ]),
    ],
    [
      'invalid specification lineage',
      (planner) =>
        (planner.workUnits[1].specRevisions[0].planRevisionId = 'sprint-plan-revision-1'),
    ],
    [
      'invalid plan change source',
      (planner) => ((planner.planChanges[0] as { source: string }).source = 'worker'),
    ],
  ])('rejects planner output with %s', (_label, mutate) => {
    const planner = mutablePlanner();
    mutate(planner);
    expect(() => decodeSprintPlannerOutputV1(planner)).toThrow(
      'Invalid Sprint control surface data',
    );
  });

  it.each<[string, SnapshotMutator]>([
    [
      'invalid work state',
      (snapshot) => ((snapshot.workUnits[0] as { state: string }).state = 'done'),
    ],
    [
      'invalid attempt outcome',
      (snapshot) => ((snapshot.workUnits[0].attempts[0] as { outcome: string }).outcome = 'done'),
    ],
    ['invalid event kind', (snapshot) => ((snapshot.events[0] as { kind: string }).kind = 'done')],
    [
      'invalid continuation status',
      (snapshot) => {
        (snapshot.continuation.sprint as { automaticEnabled: unknown }).automaticEnabled = 'yes';
        (snapshot.continuation.sprint as { status: string }).status = 'done';
      },
    ],
    [
      'deferred state without an explicit deferred event',
      (snapshot) => {
        snapshot.workUnits[0].state = 'deferred';
        snapshot.workUnits[0].actualLaunch = undefined;
      },
    ],
  ])('rejects execution snapshot with %s', (_label, mutate) => {
    const snapshot = mutableSnapshot();
    mutate(snapshot);
    expect(() => decodeSprintExecutionSnapshotV1(snapshot, plannerOutput())).toThrow(
      'Invalid Sprint control surface data',
    );
  });
});

function plannerOutput(): SprintPlannerOutputV1 {
  return {
    version: 'sprint-planner-output/v1',
    epicId: 'epic-1',
    sprint: {
      id: 'sprint-1',
      title: 'Recorded Sprint',
      summary: 'A recorded Sprint for control-surface verification.',
      details: 'The fixture models Sprint planning and Work Unit facts only.',
    },
    sprintPlan: { id: 'sprint-plan-1', sprintId: 'sprint-1' },
    concerns: [
      {
        id: 'concern-1',
        title: 'Recorded concern',
        summary: 'A concern linked to both Work Units.',
        details: 'Used to verify concern projection.',
        requiredWorkUnitIds: ['work-unit-1', 'work-unit-2'],
      },
    ],
    planRevisions: [
      {
        id: 'sprint-plan-revision-1',
        revision: 1,
        summary: 'Initial Sprint plan.',
        workUnitIds: ['work-unit-1'],
      },
      {
        id: 'sprint-plan-revision-2',
        revision: 2,
        summary: 'Revised Sprint plan.',
        supersedesPlanRevisionId: 'sprint-plan-revision-1',
        workUnitIds: ['work-unit-1', 'work-unit-2'],
      },
    ],
    sprintPlannerActivities: [
      {
        id: 'sprint-planner-activity-1',
        title: 'Initial planning',
        purpose: 'Define the first Work Unit.',
        planRevisionId: 'sprint-plan-revision-1',
        workUnitIds: ['work-unit-1'],
        userReviewGateIds: [],
      },
      {
        id: 'sprint-planner-activity-2',
        title: 'Revision planning',
        purpose: 'Define the revised Work Units.',
        planRevisionId: 'sprint-plan-revision-2',
        workUnitIds: ['work-unit-1', 'work-unit-2'],
        userReviewGateIds: ['gate-1'],
      },
    ],
    planChanges: [
      {
        id: 'plan-change-1',
        source: 'sprint_conversation',
        summary: 'Recorded direct Sprint plan change.',
        priorPlanRevisionId: 'sprint-plan-revision-1',
        resultingPlanRevisionId: 'sprint-plan-revision-2',
        priorSprintPlannerActivityId: 'sprint-planner-activity-1',
        resultingSprintPlannerActivityId: 'sprint-planner-activity-2',
      },
    ],
    parallelGroups: [],
    workUnits: [
      {
        id: 'work-unit-1',
        shortTitle: 'Accepted Work Unit',
        summary: 'Accepted recorded work.',
        details: 'Observed launch and acceptance are recorded.',
        concernIds: ['concern-1'],
        dependencies: [],
        specRevisions: [
          {
            id: 'work-unit-1-spec-1',
            revision: 1,
            planRevisionId: 'sprint-plan-revision-1',
            summary: 'Initial specification.',
            details: 'Initial recorded details.',
          },
          {
            id: 'work-unit-1-spec-2',
            revision: 2,
            planRevisionId: 'sprint-plan-revision-2',
            summary: 'Revised specification.',
            details: 'Revised recorded details.',
          },
        ],
      },
      {
        id: 'work-unit-2',
        shortTitle: 'Projected Work Unit',
        summary: 'Projected recorded work.',
        details: 'No launch is recorded.',
        concernIds: ['concern-1'],
        dependencies: [{ workUnitId: 'work-unit-1', kind: 'hard' }],
        specRevisions: [
          {
            id: 'work-unit-2-spec-1',
            revision: 1,
            planRevisionId: 'sprint-plan-revision-2',
            summary: 'Projected specification.',
            details: 'Projected recorded details.',
          },
        ],
      },
    ],
    gates: [
      {
        id: 'gate-1',
        kind: 'user',
        specRevisions: [
          {
            id: 'gate-1-spec-1',
            revision: 1,
            planRevisionId: 'sprint-plan-revision-2',
            summary: 'Review accepted Work Unit evidence.',
            requiresWorkUnitIds: ['work-unit-1'],
            requiresGateIds: [],
          },
        ],
      },
    ],
    documents: [
      {
        id: 'document-1',
        title: 'Recorded Sprint plan',
        kind: 'plan',
        sprintPlannerActivityId: 'sprint-planner-activity-2',
        planRevisionId: 'sprint-plan-revision-2',
        recordedAt: '2026-07-15T09:00:00.000Z',
      },
    ],
  };
}

function executionSnapshot(): SprintExecutionSnapshotV1 {
  return {
    version: 'sprint-execution-snapshot/v1',
    sprintId: 'sprint-1',
    activePlanRevisionId: 'sprint-plan-revision-2',
    workUnits: [
      {
        workUnitId: 'work-unit-1',
        state: 'accepted',
        projectedAt: '2026-07-15T09:01:00.000Z',
        actualLaunch: {
          launchedAt: '2026-07-15T09:02:00.000Z',
          agentSessionId: 'agent-session-work-unit-1',
        },
        attempts: [
          {
            id: 'attempt-1',
            specRevisionId: 'work-unit-1-spec-2',
            outcome: 'accepted',
            recordedAt: '2026-07-15T09:03:00.000Z',
          },
        ],
      },
      {
        workUnitId: 'work-unit-2',
        state: 'projected',
        projectedAt: '2026-07-15T09:04:00.000Z',
        attempts: [],
      },
    ],
    events: [
      {
        id: 'event-acceptance-1',
        kind: 'acceptance',
        workUnitId: 'work-unit-1',
        gateId: 'gate-1',
        summary: 'Recorded Work Unit acceptance.',
        recordedAt: '2026-07-15T09:05:00.000Z',
      },
    ],
    concernDecisions: [],
    generatedDocuments: [
      {
        id: 'generated-document-1',
        title: 'Recorded review',
        sourceDocumentId: 'document-1',
        workUnitId: 'work-unit-1',
        kind: 'review',
        recordedAt: '2026-07-15T09:06:00.000Z',
      },
    ],
    agentSessions: [
      { id: 'agent-session-sprint-1', title: 'Sprint Planner', role: 'sprint' },
      {
        id: 'agent-session-work-unit-1',
        title: 'Work Unit worker',
        role: 'work_unit_worker',
        workUnitId: 'work-unit-1',
      },
    ],
    continuation: {
      sprint: { automaticEnabled: false, status: 'not_ready', initiationObserved: false },
      epic: { automaticEnabled: false, status: 'ready_for_manual', initiationObserved: false },
    },
  };
}

function mutablePlanner(): Mutable<SprintPlannerOutputV1> {
  return structuredClone(plannerOutput()) as Mutable<SprintPlannerOutputV1>;
}

function mutableSnapshot(): Mutable<SprintExecutionSnapshotV1> {
  return structuredClone(executionSnapshot()) as Mutable<SprintExecutionSnapshotV1>;
}

type Mutable<T> = T extends readonly (infer Item)[]
  ? Mutable<Item>[]
  : T extends object
    ? { -readonly [Key in keyof T]: Mutable<T[Key]> }
    : T;
type PlannerMutator = (planner: Mutable<SprintPlannerOutputV1>) => unknown;
type SnapshotMutator = (snapshot: Mutable<SprintExecutionSnapshotV1>) => unknown;
