import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
  decodeOrchestrationNativeQueryV2,
  nativeQueryProductCompositionInputV2,
  projectEpicPlanProposal,
} from './nativeQuery';
import { composeProductOrchestrationReadModels } from './productReadModelComposer';
import { createEpicInitiationCapability } from './epicInitiationCapability';

const fixture = (name: string): unknown =>
  JSON.parse(
    readFileSync(
      resolve('src-tauri/src/orchestration/fixtures/orchestration-native-query-v2', name),
      'utf8',
    ),
  );

type MutableFixture = {
  proposalRevisions: Array<{
    proposalRevisionId: string;
    proposal: { sprints: Array<{ title: string; concernSummaries: string[] }> };
  }>;
  recordedProposalEvents: Array<{ commandId: string }>;
  provenanceLinks: Array<{ agentSessionAssociationId: string }>;
};

describe('orchestration native query v1', () => {
  it('decodes the Rust canonical proposal fixture and projects only its proposal', () => {
    const query = decodeOrchestrationNativeQueryV2(fixture('valid-proposal.json'));
    expect(projectEpicPlanProposal(query, 'epic-planning-draft-fixture')).toEqual({
      kind: 'available',
      revision: {
        id: 'proposal-revision-fixture',
        recordedAt: '2026-07-15T12:00:00.000Z',
      },
      suggestedEpicName: 'Suggested Epic fixture',
      sprints: [
        {
          title: 'Sprint fixture',
          intendedMovement: 'Move fixture forward.',
          concernSummaries: ['Concern fixture.'],
        },
      ],
    });
  });

  it('keeps an empty durable draft before-plan and rejects unknown wire fields', () => {
    const query = decodeOrchestrationNativeQueryV2(fixture('valid-empty.json'));
    expect(projectEpicPlanProposal(query, 'epic-planning-draft-1')).toEqual(
      expect.objectContaining({ kind: 'unavailable' }),
    );
    expect(() => decodeOrchestrationNativeQueryV2({ ...query, inventedRoot: true })).toThrow(
      'unknown field',
    );
  });

  it('projects durable File Review ownership and rejects an unknown owner Sprint', () => {
    const value = fixture('valid-initiated-epic.json') as Record<string, unknown>;
    value.fileReviewDocuments = [
      {
        documentRefId: 'review-doc',
        epicId: 'epic-fixture',
        sprintId: 'sprint-fixture',
        provenanceId: 'init-provenance-fixture',
        title: 'Changed files',
        artifactId: 'review-artifact',
        changedFiles: [
          { changedFileReferenceId: 'changed-1', displayName: 'src/a.ts', changeKind: 'modified' },
        ],
      },
    ];
    const query = decodeOrchestrationNativeQueryV2(value);
    const models = composeProductOrchestrationReadModels(
      nativeQueryProductCompositionInputV2(query),
    );
    expect(models.epics[0].sprints[0].documents[0]).toMatchObject({
      documentRefId: 'review-doc',
      artifactIds: ['review-artifact'],
      changedFileReferenceIds: ['changed-1'],
    });
    (value.fileReviewDocuments as Array<Record<string, unknown>>)[0].sprintId = 'missing-sprint';
    expect(() =>
      composeProductOrchestrationReadModels(
        nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(value)),
      ),
    ).toThrow('artifact ownership references an unknown Sprint');
  });

  it('projects only a correlated initiated Epic with ordered preparatory Sprints and no Work Units', () => {
    const value = fixture('valid-proposal.json') as Record<string, unknown>;
    const revision = (value.proposalRevisions as Array<Record<string, unknown>>)[0];
    value.initiationCommands = [
      {
        commandId: 'init-command',
        epicPlanningDraftId: 'epic-planning-draft-fixture',
        expectedRevisionToken: revision.revisionToken,
        actorId: 'application-user',
        idempotencyKey: 'initiate:draft:revision',
        payloadFingerprint: 'fingerprint',
        recordedAt: '2026-07-15T12:00:00.000Z',
      },
    ];
    value.initiationResults = [
      {
        resultId: 'init-result',
        commandId: 'init-command',
        recordedAt: '2026-07-15T12:00:00.000Z',
      },
    ];
    value.initiationEvents = [
      {
        eventId: 'init-event',
        commandId: 'init-command',
        resultId: 'init-result',
        recordedAt: '2026-07-15T12:00:00.000Z',
      },
    ];
    value.initiationProvenance = [
      {
        provenanceId: 'init-provenance',
        commandId: 'init-command',
        resultId: 'init-result',
        eventId: 'init-event',
        recordedAt: '2026-07-15T12:00:00.000Z',
      },
    ];
    value.materialSnapshots = [
      {
        materialSnapshotId: 'snapshot',
        epicPlanningDraftId: 'epic-planning-draft-fixture',
        proposalRevisionId: 'proposal-revision-fixture',
        version: 1,
        proposal: revision.proposal,
        contentHash: 'a'.repeat(64),
        recordedAt: '2026-07-15T12:00:00.000Z',
      },
    ];
    value.initiatedEpics = [
      {
        initiationId: 'initiation',
        epicPlanningDraftId: 'epic-planning-draft-fixture',
        proposalRevisionId: 'proposal-revision-fixture',
        materialSnapshotId: 'snapshot',
        epicId: 'epic',
        recordedAt: '2026-07-15T12:00:00.000Z',
        commandId: 'init-command',
        resultId: 'init-result',
        eventId: 'init-event',
        provenanceId: 'init-provenance',
      },
    ];
    value.initiatedSprints = [
      {
        sprintId: 'sprint',
        epicId: 'epic',
        ordinal: 0,
        title: 'Sprint fixture',
        intendedMovement: 'Move fixture forward.',
        concernSummaries: ['Concern fixture.'],
        sprintPlanId: 'plan',
        sprintPlanRevisionId: 'revision',
      },
    ];
    (value.planningDrafts as Array<Record<string, unknown>>)[0].status = 'initiated';

    const query = decodeOrchestrationNativeQueryV2(value);
    expect(createEpicInitiationCapability(query, 'epic-planning-draft-fixture')).toMatchObject({
      status: 'already_initiated',
    });
    const input = nativeQueryProductCompositionInputV2(query);
    const read = composeProductOrchestrationReadModels(input);
    expect(read.epics).toHaveLength(1);
    expect(read.epics[0]?.sprints).toHaveLength(1);
    expect(input.events.workUnits).toEqual([]);
    expect(input.events.sprintPlanRevisions).toHaveLength(1);
    expect(input.events.reviews).toEqual([]);
    expect(input.events.agentSessionReferences).toEqual([]);
    expect(read.epics[0]?.agentSessionReferences).toEqual([]);
  });

  it('projects only settled materialized responsibilities with their accepted revision and dependencies', () => {
    const value = fixture('valid-initiated-epic.json') as Record<string, unknown>;
    value.workUnitMaterializations = [
      {
        materializationId: 'materialization-1',
        planningPointId: 'point-1',
        acceptedRevisionId: 'accepted-revision-1',
        epicId: 'epic-fixture',
        sprintId: 'sprint-fixture',
        workSliceId: 'slice-1',
        authorizationRecordedAt: '2026-08-02T00:00:00Z',
        attemptRecordedAt: '2026-08-02T00:00:01Z',
        workUnitsCreatedAt: '2026-08-02T00:00:02Z',
        relationshipsCompletedAt: '2026-08-02T00:00:03Z',
        settledAt: '2026-08-02T00:00:04Z',
      },
    ];
    value.workUnits = [
      {
        workUnitId: 'unit-1',
        materializationId: 'materialization-1',
        workSliceId: 'slice-1',
        acceptedRevisionId: 'accepted-revision-1',
        laneOrdinal: 0,
        laneTitle: 'First responsibility',
        specification: 'First specification.',
        handlerActivation: {
          attemptId: 'handler-attempt-1',
          handlerSessionId: 'handler-session-1',
          handlerInvocationId: 'handler-invocation-1',
          eligibilityState: 'eligible',
          requestedAt: '2026-08-02T00:01:00Z',
          authorizedAt: '2026-08-02T00:01:01Z',
          attemptCreatedAt: '2026-08-02T00:01:02Z',
          executionSupportGrantedAt: '2026-08-02T00:01:03Z',
          isolatedWorktreeReadyAt: '2026-08-02T00:01:04Z',
          handlerSessionCreatedAt: '2026-08-02T00:01:05Z',
          handlerInvocationPreparedAt: '2026-08-02T00:01:06Z',
          handlerHarnessBoundAt: '2026-08-02T00:01:07Z',
          launchRequestedAt: '2026-08-02T00:01:08Z',
          launchAcceptedAt: '2026-08-02T00:01:09Z',
          handlerReadyAt: '2026-08-02T00:01:10Z',
          providerActivationObservedAt: '2026-08-02T00:01:11Z',
        },
      },
      {
        workUnitId: 'unit-2',
        materializationId: 'materialization-1',
        workSliceId: 'slice-1',
        acceptedRevisionId: 'accepted-revision-1',
        laneOrdinal: 1,
        laneTitle: 'Second responsibility',
        specification: 'Second specification.',
        handlerActivation: {
          attemptId: 'handler-attempt-2',
          eligibilityState: 'blocked',
          blockedReason: 'prerequisite_satisfaction_not_authoritative',
          requestedAt: '2026-08-02T00:01:00Z',
        },
      },
    ];
    value.workUnitRelationships = [
      {
        relationshipId: 'point',
        materializationId: 'materialization-1',
        relationshipKind: 'planning_point',
        fromId: 'point-1',
        toId: 'slice-1',
      },
      {
        relationshipId: 'sprint',
        materializationId: 'materialization-1',
        relationshipKind: 'sprint',
        fromId: 'sprint-fixture',
        toId: 'slice-1',
      },
      {
        relationshipId: 'lane-1',
        materializationId: 'materialization-1',
        relationshipKind: 'lane',
        fromId: 'slice-1',
        toId: 'unit-1',
        ordinal: 0,
      },
      {
        relationshipId: 'order-1',
        materializationId: 'materialization-1',
        relationshipKind: 'order',
        fromId: 'slice-1',
        toId: 'unit-1',
        ordinal: 0,
      },
      {
        relationshipId: 'lane-2',
        materializationId: 'materialization-1',
        relationshipKind: 'lane',
        fromId: 'slice-1',
        toId: 'unit-2',
        ordinal: 1,
      },
      {
        relationshipId: 'order-2',
        materializationId: 'materialization-1',
        relationshipKind: 'order',
        fromId: 'slice-1',
        toId: 'unit-2',
        ordinal: 1,
      },
      {
        relationshipId: 'dependency',
        materializationId: 'materialization-1',
        relationshipKind: 'depends_on',
        fromId: 'unit-2',
        toId: 'unit-1',
      },
    ];
    const query = decodeOrchestrationNativeQueryV2(value);
    const read = composeProductOrchestrationReadModels(nativeQueryProductCompositionInputV2(query));
    const sprint = read.epics[0]!.sprints[0]!;
    expect(sprint.workUnitMaterializations).toMatchObject([
      { acceptedRevisionId: 'accepted-revision-1', stage: 'settled' },
    ]);
    expect(sprint.revisionViews[0]!.workUnits).toMatchObject([
      { workUnitId: 'unit-1' },
      { workUnitId: 'unit-2', dependencies: [{ workUnitId: 'unit-1' }] },
    ]);
    const input = nativeQueryProductCompositionInputV2(query);
    expect(input.referenceIndex.workUnits[0]!.details).toContain(
      'Handler launch accepted and application Handler readiness recorded.',
    );
    expect(input.referenceIndex.workUnits[0]!.details).toContain(
      'Provider activity observed separately',
    );
    expect(input.referenceIndex.workUnits[1]!.details).toContain('Handler activation blocked');

    (value.workUnitRelationships as Array<Record<string, unknown>>).push({
      relationshipId: 'duplicate-dependency',
      materializationId: 'materialization-1',
      relationshipKind: 'depends_on',
      fromId: 'unit-2',
      toId: 'unit-1',
    });
    expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow(
      'duplicate Work Unit relationship',
    );
    (value.workUnitRelationships as Array<Record<string, unknown>>).pop();

    (value.workUnitRelationships as Array<Record<string, unknown>>)[2]!.toId = 'missing-unit';
    expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow(
      'requires matching lane and order relationships',
    );
  });

  it('keeps partial materialization stages separate from Work Unit production truth', () => {
    const value = fixture('valid-initiated-epic.json') as Record<string, unknown>;
    value.workUnitMaterializations = [
      {
        materializationId: 'materialization-1',
        planningPointId: 'point-1',
        acceptedRevisionId: 'accepted-revision-1',
        epicId: 'epic-fixture',
        sprintId: 'sprint-fixture',
        workSliceId: 'slice-1',
        authorizationRecordedAt: '2026-08-02T00:00:00Z',
        attemptRecordedAt: '2026-08-02T00:00:01Z',
      },
    ];
    value.workUnits = [];
    value.workUnitRelationships = [];
    const read = composeProductOrchestrationReadModels(
      nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(value)),
    );
    expect(read.epics[0]!.sprints[0]!.workUnitMaterializations).toMatchObject([
      { stage: 'attempt_recorded' },
    ]);
    expect(read.epics[0]!.sprints[0]!.revisionViews[0]!.workUnits).toEqual([]);
  });

  it('reports blocked and ready initiation from the selected durable draft without caller-owned retry keys', () => {
    const proposalOnly = decodeOrchestrationNativeQueryV2(fixture('valid-proposal.json'));
    const ready = createEpicInitiationCapability(proposalOnly, 'epic-planning-draft-fixture');
    expect(ready.status).toBe('ready');
    expect(ready).toEqual({
      status: 'ready',
      request: expect.objectContaining({
        idempotencyKey: 'initiate:epic-planning-draft-fixture:proposal-revision-fixture',
      }),
    });
    expect(createEpicInitiationCapability(proposalOnly, 'missing-draft')).toMatchObject({
      status: 'blocked',
    });
  });

  it('keeps a canceled draft distinct from an empty or missing proposal', () => {
    const proposalOnly = fixture('valid-proposal.json') as {
      planningDrafts: Array<Record<string, unknown>>;
    };
    proposalOnly.planningDrafts[0].status = 'canceled';
    proposalOnly.planningDrafts[0].canceledAt = '2026-07-15T13:00:00.000Z';
    const query = decodeOrchestrationNativeQueryV2(proposalOnly);
    expect(createEpicInitiationCapability(query, 'epic-planning-draft-fixture')).toEqual({
      status: 'blocked',
      reason: 'This Epic Planning Draft was canceled and cannot be initiated.',
    });
  });

  it('rejects a draft status that does not match durable initiation facts', () => {
    const value = fixture('valid-proposal.json') as Record<string, unknown>;
    (value.planningDrafts as Array<Record<string, unknown>>)[0].status = 'initiated';
    expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow(
      'planning draft initiation status does not match durable initiation facts',
    );
  });

  it('rejects duplicate or unconsumed initiation lifecycle facts', () => {
    const duplicateDraft = fixture('valid-initiated-epic.json') as {
      initiatedEpics: Array<Record<string, unknown>>;
    };
    duplicateDraft.initiatedEpics.push({
      ...duplicateDraft.initiatedEpics[0],
      initiationId: 'another-initiation',
      epicId: 'another-epic',
    });
    expect(() => decodeOrchestrationNativeQueryV2(duplicateDraft)).toThrow(
      'duplicate initiated Epic planning draft ID',
    );

    const unconsumedCommand = fixture('valid-initiated-epic.json') as {
      initiationCommands: Array<Record<string, unknown>>;
    };
    unconsumedCommand.initiationCommands.push({
      ...unconsumedCommand.initiationCommands[0],
      commandId: 'unconsumed-command',
      idempotencyKey: 'unconsumed-key',
    });
    expect(() => decodeOrchestrationNativeQueryV2(unconsumedCommand)).toThrow(
      'initiation command does not contribute to exactly one initiated Epic',
    );
  });

  it.each([
    [
      'empty Sprint list',
      (value: MutableFixture) => (value.proposalRevisions[0].proposal.sprints = []),
    ],
    [
      'blank title',
      (value: MutableFixture) => (value.proposalRevisions[0].proposal.sprints[0].title = '  '),
    ],
    [
      'too many concerns',
      (value: MutableFixture) =>
        (value.proposalRevisions[0].proposal.sprints[0].concernSummaries = Array(21).fill('x')),
    ],
    [
      'duplicate proposal revision',
      (value: MutableFixture) => value.proposalRevisions.push({ ...value.proposalRevisions[0] }),
    ],
    [
      'event command mismatch',
      (value: MutableFixture) => (value.recordedProposalEvents[0].commandId = 'other-command'),
    ],
    [
      'provenance association mismatch',
      (value: MutableFixture) => (value.provenanceLinks[0].agentSessionAssociationId = 'missing'),
    ],
    [
      'initiated Epic correlation mismatch',
      (value: MutableFixture) => {
        const root = value as unknown as Record<string, unknown>;
        root.initiationCommands = [
          {
            commandId: 'command',
            epicPlanningDraftId: 'epic-planning-draft-fixture',
            expectedRevisionToken: 'proposal-token-fixture',
            actorId: 'application-user',
            idempotencyKey: 'key',
            payloadFingerprint: 'fingerprint',
            recordedAt: 't',
          },
        ];
        root.initiationResults = [{ resultId: 'result', commandId: 'command', recordedAt: 't' }];
        root.initiationEvents = [
          { eventId: 'event', commandId: 'command', resultId: 'result', recordedAt: 't' },
        ];
        root.initiationProvenance = [
          {
            provenanceId: 'provenance',
            commandId: 'command',
            resultId: 'result',
            eventId: 'event',
            recordedAt: 't',
          },
        ];
        root.materialSnapshots = [
          {
            materialSnapshotId: 'snapshot',
            epicPlanningDraftId: 'epic-planning-draft-fixture',
            proposalRevisionId: 'proposal-revision-fixture',
            version: 1,
            proposal: value.proposalRevisions[0].proposal,
            contentHash: 'a'.repeat(64),
            recordedAt: 't',
          },
        ];
        root.initiatedEpics = [
          {
            initiationId: 'initiation',
            epicPlanningDraftId: 'epic-planning-draft-fixture',
            proposalRevisionId: 'proposal-revision-fixture',
            materialSnapshotId: 'snapshot',
            epicId: 'epic',
            recordedAt: 't',
            commandId: 'other-command',
            resultId: 'result',
            eventId: 'event',
            provenanceId: 'provenance',
          },
        ];
        root.initiatedSprints = [];
      },
    ],
  ])('rejects malformed %s semantic content', (_label, mutate) => {
    const value = fixture('valid-proposal.json') as MutableFixture;
    mutate(value);
    expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow(
      'Invalid orchestration native query',
    );
  });
});
