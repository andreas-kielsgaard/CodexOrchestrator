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
    expect(input.events.agentSessionReferences).toEqual([
      expect.objectContaining({
        agentSessionId: 'agent-session-fixture',
        targetKind: 'epic',
        targetId: 'epic',
        semanticRole: 'epic_plan_builder',
      }),
    ]);
    expect(read.epics[0]?.agentSessionReferences).toEqual([
      expect.objectContaining({
        agentSessionId: 'agent-session-fixture',
        semanticRole: 'epic_plan_builder',
      }),
    ]);
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
