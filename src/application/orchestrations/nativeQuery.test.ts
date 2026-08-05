import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
  decodeOrchestrationNativeQueryV2,
  nativeQueryProductCompositionInputV2,
  projectEpicPlanProposal,
} from './nativeQuery';
import { composeProductOrchestrationReadModels } from './productReadModelComposer';
import { createEpicInitiationCapability } from './epicInitiationCapability';
import { presentProductOrchestrations } from '../../app/orchestrationPresentation';

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

  it('requires the productive execution bundle to be all present or all absent and rejects private additions', () => {
    const value = fixture('valid-empty.json') as Record<string, unknown>;
    value.workUnitExecutionStates = [];
    expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow('execution projection bundle is incomplete');
    Object.assign(value, {
      workSliceExecutionGraphCompletions: [], workSliceExecutionSettlements: [],
      workSlicePlanningPointExecutionSettlements: [], workSliceExecutionAttentions: [],
    });
    const query = decodeOrchestrationNativeQueryV2(value);
    expect(query.workUnitExecutionStates).toEqual([]);
    (value.workUnitExecutionStates as unknown[]).push({ workUnitId: 'private', materializationId: 'private', acceptedRevisionId: 'private', state: 'ready', recordedAt: '2026-08-05T00:00:00Z', graphFingerprint: 'private' });
    expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow('unknown field');
  });

  it('decodes the Rust-authored settled multi-root execution graph into product read models', () => {
    const query = decodeOrchestrationNativeQueryV2(fixture('valid-execution-graph.json'));
    const sprint = composeProductOrchestrationReadModels(
      nativeQueryProductCompositionInputV2(query),
    ).epics[0]!.sprints[0]!;
    const materialization = sprint.workUnitMaterializations![0]!;
    expect(query.workUnits.map((unit) => unit.workUnitId)).toEqual([
      'execution-root-a',
      'execution-root-b',
      'execution-middle',
      'execution-leaf',
    ]);
    expect(sprint.revisionViews[0]!.workUnits).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          workUnitId: 'execution-root-a',
          executionState: expect.objectContaining({ state: 'settled' }),
        }),
        expect.objectContaining({
          workUnitId: 'execution-leaf',
          executionState: expect.objectContaining({ state: 'settled' }),
        }),
      ]),
    );
    expect(materialization.execution).toMatchObject({
      graphCompletion: { completedAt: '2026-08-05T00:00:00Z' },
      settlement: { settledAt: '2026-08-05T00:00:00Z' },
      planningPointSettlement: { settledAt: '2026-08-05T00:00:00Z' },
    });
  });

  it('still requires attemptHistory in the productive native Work Unit contract', () => {
    const value = fixture('valid-execution-graph.json') as Record<string, unknown>;
    const unit = (value.workUnits as Array<Record<string, unknown>>)[0]!;
    delete unit.attemptHistory;

    expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow('Work Unit attemptHistory');
  });

  it('projects selected unresolved execution states and blocks terminal facts for targeted attention', () => {
    const value = fixture('valid-execution-graph.json') as Record<string, unknown>;
    value.workSliceExecutionGraphCompletions = [];
    value.workSliceExecutionSettlements = [];
    value.workSlicePlanningPointExecutionSettlements = [];
    value.workSliceExecutionAttentions = [
      { materializationId: 'execution-materialization-fixture', recordedAt: '2026-08-05T00:00:01Z' },
    ];
    const states = value.workUnitExecutionStates as Array<Record<string, unknown>>;
    states[0]!.state = 'ready';
    states[1]!.state = 'active';
    states[2]!.state = 'retry_authorized';
    states[3]!.state = 'handed_back';
    states.push({
      ...states[0], workUnitId: 'execution-root-a', state: 'attention', recordedAt: '2026-08-05T00:00:01Z',
    });
    states.splice(0, 1);
    const query = decodeOrchestrationNativeQueryV2(value);
    expect(query.workUnitExecutionStates.map((state) => state.state).sort()).toEqual([
      'active', 'attention', 'handed_back', 'retry_authorized',
    ]);
    expect(query.workSliceExecutionGraphCompletions).toEqual([]);
    expect(query.workSliceExecutionSettlements).toEqual([]);
    expect(query.workSlicePlanningPointExecutionSettlements).toEqual([]);
  });

  it.each(['waiting_on_prerequisites', 'ready'] as const)(
    'decodes %s for every Work Unit without inventing terminal facts',
    (state) => {
      const value = fixture('valid-execution-graph.json') as Record<string, unknown>;
      value.workSliceExecutionGraphCompletions = [];
      value.workSliceExecutionSettlements = [];
      value.workSlicePlanningPointExecutionSettlements = [];
      for (const entry of value.workUnitExecutionStates as Array<Record<string, unknown>>)
        entry.state = state;
      const query = decodeOrchestrationNativeQueryV2(value);
      expect(query.workUnitExecutionStates.map((entry) => entry.state)).toEqual([
        state,
        state,
        state,
        state,
      ]);
      expect(query.workSliceExecutionSettlements).toEqual([]);
    },
  );

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
        actionContinuation: {
          attemptId: 'handler-attempt-1',
          handlerSessionId: 'handler-session-1',
          originalHandlerInvocationId: 'handler-invocation-1',
          actionInvocationId: 'handler-action-invocation-1',
          actionHarnessRevisionId: 'handler-action-revision-1',
          requestedAt: '2026-08-02T00:01:12Z',
          authorizedAt: '2026-08-02T00:01:13Z',
          invocationPreparedAt: '2026-08-02T00:01:14Z',
          harnessBoundAt: '2026-08-02T00:01:15Z',
          launchRequestedAt: '2026-08-02T00:01:16Z',
          launchAcceptedAt: '2026-08-02T00:01:17Z',
          providerActivationObservedAt: '2026-08-02T00:01:18Z',
          actionReadyAt: '2026-08-02T00:01:19Z',
        },
        implementerActivation: {
          attemptId: 'handler-attempt-1',
          handlerActionInvocationId: 'handler-action-invocation-1',
          implementerSessionId: 'implementer-session-1',
          implementerInvocationId: 'implementer-invocation-1',
          implementerHarnessRevisionId: 'implementer-revision-1',
          requestedAt: '2026-08-02T00:01:20Z',
          authorizedAt: '2026-08-02T00:01:21Z',
          executionSupportGrantedAt: '2026-08-02T00:01:22Z',
          isolatedWorktreeReadyAt: '2026-08-02T00:01:23Z',
          implementerSessionCreatedAt: '2026-08-02T00:01:24Z',
          implementerInvocationPreparedAt: '2026-08-02T00:01:25Z',
          implementerHarnessBoundAt: '2026-08-02T00:01:26Z',
          launchRequestedAt: '2026-08-02T00:01:27Z',
          launchAcceptedAt: '2026-08-02T00:01:28Z',
          providerActivationObservedAt: '2026-08-02T00:01:29Z',
          implementerReadyAt: '2026-08-02T00:01:30Z',
        },
        attemptHistory: [],
        retryAttempts: [],
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
        attemptHistory: [],
        retryAttempts: [],
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
      {
        workUnitId: 'unit-1',
        handlerActivation: {
          eligibilityState: 'eligible',
          stage: 'handler_ready',
          providerActivityObserved: true,
        },
        actionContinuation: { stage: 'action_ready', providerActivityObserved: true },
        implementerActivation: { stage: 'implementer_ready', providerActivityObserved: true },
      },
      {
        workUnitId: 'unit-2',
        dependencies: [{ workUnitId: 'unit-1' }],
        handlerActivation: {
          eligibilityState: 'blocked',
          blockedReason: 'prerequisite_satisfaction_not_authoritative',
        },
      },
    ]);
    const input = nativeQueryProductCompositionInputV2(query);
    expect(input.referenceIndex.workUnits[0]!.details).toContain(
      'Handler launch accepted and application Handler readiness recorded.',
    );
    expect(input.referenceIndex.workUnits[0]!.details).toContain(
      'Provider activity observed separately',
    );
    expect(input.referenceIndex.workUnits[1]!.details).toContain('Handler activation blocked');

    const failedActivation = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    const failedImplementer = (failedActivation.workUnits as Array<Record<string, unknown>>)[0]!
      .implementerActivation as Record<string, unknown>;
    delete failedImplementer.implementerReadyAt;
    failedImplementer.failureReason = 'launch_terminal_failure';
    expect(
      nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(failedActivation))
        .referenceIndex.workUnits[0]!.implementerActivation,
    ).toMatchObject({ stage: 'failed', failureReason: 'launch_terminal_failure' });

    const failedAction = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    const failedContinuation = (failedAction.workUnits as Array<Record<string, unknown>>)[0]!
      .actionContinuation as Record<string, unknown>;
    delete failedContinuation.launchAcceptedAt;
    delete failedContinuation.providerActivationObservedAt;
    delete failedContinuation.actionReadyAt;
    failedContinuation.failureReason = 'handler_action_launch_not_accepted';
    expect(
      nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(failedAction))
        .referenceIndex.workUnits[0]!.actionContinuation,
    ).toMatchObject({
      stage: 'failed',
      failureReason: 'handler_action_launch_not_accepted',
    });

    const failedReadyAction = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    (
      (failedReadyAction.workUnits as Array<Record<string, unknown>>)[0]!
        .actionContinuation as Record<string, unknown>
    ).failureReason = 'forged_failure';
    expect(() => decodeOrchestrationNativeQueryV2(failedReadyAction)).toThrow(
      'failed Handler action continuation cannot be application-ready',
    );

    const malformedCorrelation = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    (
      (malformedCorrelation.workUnits as Array<Record<string, unknown>>)[0]!
        .implementerActivation as Record<string, unknown>
    ).handlerActionInvocationId = 'foreign-action';
    expect(() => decodeOrchestrationNativeQueryV2(malformedCorrelation)).toThrow(
      'Implementer activation does not match the Handler action invocation',
    );

    const missingHandlerPrerequisite = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    delete (
      (missingHandlerPrerequisite.workUnits as Array<Record<string, unknown>>)[0]!
        .handlerActivation as Record<string, unknown>
    ).isolatedWorktreeReadyAt;
    expect(() => decodeOrchestrationNativeQueryV2(missingHandlerPrerequisite)).toThrow(
      'Handler activation has a phase without its prerequisite',
    );

    const foreignOriginalHandler = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    (
      (foreignOriginalHandler.workUnits as Array<Record<string, unknown>>)[0]!
        .actionContinuation as Record<string, unknown>
    ).originalHandlerInvocationId = 'foreign-original-handler';
    expect(() => decodeOrchestrationNativeQueryV2(foreignOriginalHandler)).toThrow(
      'Handler action continuation does not match the original Handler Session and invocation',
    );

    const staleBlockedAction = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    (
      (staleBlockedAction.workUnits as Array<Record<string, unknown>>)[0]!
        .actionContinuation as Record<string, unknown>
    ).blockedReason = 'stale_action_block';
    expect(() => decodeOrchestrationNativeQueryV2(staleBlockedAction)).toThrow(
      'blocked Handler action continuation cannot have authorized action phases',
    );

    const reusedActionInvocation = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    const reusedImplementer = (
      reusedActionInvocation.workUnits as Array<Record<string, unknown>>
    )[0]!.implementerActivation as Record<string, unknown>;
    reusedImplementer.implementerInvocationId = reusedImplementer.handlerActionInvocationId;
    expect(() => decodeOrchestrationNativeQueryV2(reusedActionInvocation)).toThrow(
      'Implementer invocation must differ from the Handler action invocation',
    );

    const launchAcceptedNotReady = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    const acceptedActivation = (
      launchAcceptedNotReady.workUnits as Array<Record<string, unknown>>
    )[0]!.handlerActivation as Record<string, unknown>;
    delete acceptedActivation.handlerReadyAt;
    const acceptedNotReadyInput = nativeQueryProductCompositionInputV2(
      decodeOrchestrationNativeQueryV2(launchAcceptedNotReady),
    );
    expect(acceptedNotReadyInput.referenceIndex.workUnits[0]!.details).toContain(
      'Handler launch accepted; application Handler readiness is not yet recorded.',
    );
    expect(acceptedNotReadyInput.referenceIndex.workUnits[0]!.details).not.toContain(
      'acceptance is not yet recorded',
    );

    const missingEligibility = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    (missingEligibility.workUnits as Array<Record<string, unknown>>)[0]!.handlerActivation = {
      attemptId: 'handler-attempt-1',
    };
    expect(() => decodeOrchestrationNativeQueryV2(missingEligibility)).toThrow(
      'invalid Handler eligibility state',
    );

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

  it('projects Implementer reporting states and rejects malformed authority or readiness', () => {
    const absent = implementerOutcomeNativeFixture();
    const absentQuery = decodeOrchestrationNativeQueryV2(absent);
    expect(primaryAttempt(absentQuery.workUnits[0]!)?.implementerOutcome).toBeUndefined();

    const inProgress = implementerOutcomeNativeFixture();
    (inProgress.workUnits as Array<Record<string, unknown>>)[0]!.implementerOutcome =
      implementerOutcomeFixture('in_progress');
    const inProgressInput = nativeQueryProductCompositionInputV2(
      decodeOrchestrationNativeQueryV2(inProgress),
    );
    expect(primaryAttempt(inProgressInput.referenceIndex.workUnits[0]!)?.implementerOutcome).toMatchObject({
      reportingRequestedAt: '2026-08-04T00:00:00Z',
      reportingPreparedAt: '2026-08-04T00:00:01Z',
    });
    expect(primaryAttempt(inProgressInput.referenceIndex.workUnits[0]!)?.implementerOutcome).not.toHaveProperty(
      'submittedOutcome',
    );

    for (const status of ['failed', 'canceled'] as const) {
      const terminal = implementerOutcomeNativeFixture();
      (terminal.workUnits as Array<Record<string, unknown>>)[0]!.implementerOutcome =
        implementerOutcomeFixture(status);
      expect(
        primaryAttempt(decodeOrchestrationNativeQueryV2(terminal).workUnits[0]!)?.implementerOutcome
          ?.terminalLifecycle,
      ).toMatchObject({ status });
    }

    const reviewReady = implementerOutcomeNativeFixture();
    (reviewReady.workUnits as Array<Record<string, unknown>>)[0]!.implementerOutcome =
      implementerOutcomeFixture('review_ready');
    const readyQuery = decodeOrchestrationNativeQueryV2(reviewReady);
    const readyOutcome = primaryAttempt(readyQuery.workUnits[0]!)!.implementerOutcome!;
    expect(readyOutcome.submittedOutcome).toMatchObject({
      variant: 'review_pending',
      summaryClaim: 'Implemented the bounded change.',
      validationStatementClaim: 'Focused checks passed.',
    });
    expect(readyOutcome.evidence?.changedFiles).toEqual([
      expect.objectContaining({ evidenceRef: 'evidence-1', contentFingerprint: 'content-1' }),
    ]);
    expect(readyOutcome.applicationAcceptedAt).toBe('2026-08-04T00:00:10Z');
    expect(readyOutcome.handlerReviewReadyAt).toBe('2026-08-04T00:00:11Z');
    expect(
      composeProductOrchestrationReadModels(nativeQueryProductCompositionInputV2(readyQuery))
        .epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!.attemptHistory[0]?.implementerOutcome,
    ).toEqual(readyOutcome);

    const malformed = [
      (() => {
        const outcome = implementerOutcomeFixture('review_ready');
        delete (outcome.evidence as Record<string, unknown>).comparisonFingerprint;
        return outcome;
      })(),
      { ...implementerOutcomeFixture('review_ready'), semanticCompletion: undefined },
      { ...implementerOutcomeFixture('review_ready'), applicationAcceptedAt: undefined },
      { ...implementerOutcomeFixture('review_ready'), implementerSessionId: 'foreign-session' },
      {
        ...implementerOutcomeFixture('review_ready'),
        reportingInvocationId: 'implementer-invocation-1',
      },
      {
        ...implementerOutcomeFixture('review_ready'),
        terminalLifecycle: { status: 'unknown', observedAt: '2026-08-04T00:00:09Z' },
      },
      {
        ...implementerOutcomeFixture('review_ready'),
        submittedOutcome: {
          ...(implementerOutcomeFixture('review_ready').submittedOutcome as object),
          variant: 'accepted',
        },
      },
      {
        ...implementerOutcomeFixture('in_progress'),
        reportingPreparedAt: '2026-08-03T23:59:59Z',
      },
    ];
    for (const implementerOutcome of malformed) {
      const value = implementerOutcomeNativeFixture();
      (value.workUnits as Array<Record<string, unknown>>)[0]!.implementerOutcome =
        implementerOutcome;
      expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow(
        'Invalid orchestration native query',
      );
    }
  });

  it('projects the Handler review boundary without inventing later workflow', () => {
    const pending = implementerOutcomeNativeFixture();
    const pendingUnit = (pending.workUnits as Array<Record<string, unknown>>)[0]!;
    pendingUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
    pendingUnit.handlerReview = handlerReviewFixture('pending');
    const pendingQuery = decodeOrchestrationNativeQueryV2(pending);
    expect(primaryAttempt(pendingQuery.workUnits[0]!)?.handlerReview).toMatchObject({
      reviewReadyAt: '2026-08-04T00:00:16Z',
      delivered: { comparisonFingerprint: 'comparison-1' },
    });
    const pendingUnitModel = composeProductOrchestrationReadModels(
      nativeQueryProductCompositionInputV2(pendingQuery),
    ).epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!;
    expect(primaryAttempt(pendingUnitModel)?.handlerReview?.semanticJudgment).toBeUndefined();
    expect(primaryAttempt(pendingUnitModel)?.handlerDecision).toBeUndefined();

    for (const variant of ['accepted', 'returned'] as const) {
      const value = implementerOutcomeNativeFixture();
      const unit = (value.workUnits as Array<Record<string, unknown>>)[0]!;
      unit.implementerOutcome = implementerOutcomeFixture('review_ready');
      unit.handlerReview = handlerReviewFixture(variant);
      unit.handlerDecision = handlerDecisionFixture(variant);
      if (variant === 'returned') {
        (unit.handlerReview as Record<string, unknown>).conflict = {
          occurredAt: '2026-08-04T00:00:16Z',
          reason: 'divergent_review_judgment',
        };
      }
      const query = decodeOrchestrationNativeQueryV2(value);
      expect(primaryAttempt(query.workUnits[0]!)?.handlerDecision?.variant).toBe(variant);
      expect(primaryAttempt(query.workUnits[0]!)?.handlerReview?.semanticJudgment?.variant).toBe(
        variant === 'accepted' ? 'accept' : 'return',
      );
      expect(primaryAttempt(query.workUnits[0]!)?.handlerReview?.lifecycle?.status).toBe('completed');
    }

    const failed = implementerOutcomeNativeFixture();
    const failedUnit = (failed.workUnits as Array<Record<string, unknown>>)[0]!;
    failedUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
    failedUnit.handlerReview = handlerReviewFixture('failed');
    expect(primaryAttempt(decodeOrchestrationNativeQueryV2(failed).workUnits[0]!)?.handlerDecision).toBeUndefined();

    const malformed = [
      (() => {
        const value = implementerOutcomeNativeFixture();
        const unit = (value.workUnits as Array<Record<string, unknown>>)[0]!;
        unit.implementerOutcome = implementerOutcomeFixture('review_ready');
        unit.handlerReview = handlerReviewFixture('accepted');
        unit.handlerDecision = handlerDecisionFixture('accepted');
        (unit.handlerDecision as Record<string, unknown>).settlementReadyAt =
          '2026-08-04T00:00:15Z';
        return value;
      })(),
      (() => {
        const value = implementerOutcomeNativeFixture();
        const unit = (value.workUnits as Array<Record<string, unknown>>)[0]!;
        unit.implementerOutcome = implementerOutcomeFixture('review_ready');
        unit.handlerReview = handlerReviewFixture('pending');
        unit.handlerDecision = {
          reviewInvocationId: 'review-invocation-1',
          variant: 'accepted',
          fingerprint: 'decision-1',
          recordedAt: '2026-08-04T00:00:14Z',
          implementationAcceptedAt: '2026-08-04T00:00:14Z',
        };
        return value;
      })(),
      (() => {
        const value = implementerOutcomeNativeFixture();
        const unit = (value.workUnits as Array<Record<string, unknown>>)[0]!;
        unit.implementerOutcome = implementerOutcomeFixture('review_ready');
        unit.handlerReview = handlerReviewFixture('accepted');
        (unit.handlerReview as Record<string, unknown>).semanticJudgment = {
          variant: 'unknown',
          fingerprint: 'judgment-1',
          recordedAt: '2026-08-04T00:00:12Z',
        };
        return value;
      })(),
    ];
    malformed.forEach((value) =>
      expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow(
        'Invalid orchestration native query',
      ),
    );
  });

  it('projects returned Work Unit retry stages and rejects private or incoherent retry facts', () => {
    const value = implementerOutcomeNativeFixture();
    const unit = (value.workUnits as Array<Record<string, unknown>>)[0]!;
    unit.implementerOutcome = implementerOutcomeFixture('review_ready');
    unit.handlerReview = handlerReviewFixture('returned');
    unit.handlerDecision = handlerDecisionFixture('returned');
    unit.retryAttempts = [retryAttemptFixture('ready')];

    const query = decodeOrchestrationNativeQueryV2(value);
    expect(query.workUnits[0]!.retryAttempts[0]).toMatchObject({
      ordinal: 1,
      originAttemptId: 'attempt-1',
      retryAttemptId: 'retry-attempt-1',
      providerActivationObservedAt: '2026-08-04T00:00:31Z',
      retryReadyAt: '2026-08-04T00:00:32Z',
    });
    const model = composeProductOrchestrationReadModels(
      nativeQueryProductCompositionInputV2(query),
    ).epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!;
    expect(model.retryAttempts).toEqual(query.workUnits[0]!.retryAttempts);
    expect(model.retryAttempts[0]).not.toHaveProperty('candidateCommitId');
    expect(model.retryAttempts[0]).not.toHaveProperty('candidateTreeId');
    expect(model.retryAttempts[0]).not.toHaveProperty('privateRefName');

    const partial = implementerOutcomeNativeFixture();
    const partialUnit = (partial.workUnits as Array<Record<string, unknown>>)[0]!;
    partialUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
    partialUnit.handlerReview = handlerReviewFixture('returned');
    partialUnit.handlerDecision = handlerDecisionFixture('returned');
    partialUnit.retryAttempts = [retryAttemptFixture('partial')];
    expect(decodeOrchestrationNativeQueryV2(partial).workUnits[0]!.retryAttempts[0]).toMatchObject({
      candidatePinnedAt: '2026-08-04T00:00:21Z',
      implementerHarnessBoundAt: '2026-08-04T00:00:27Z',
    });

    const failed = implementerOutcomeNativeFixture();
    const failedUnit = (failed.workUnits as Array<Record<string, unknown>>)[0]!;
    failedUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
    failedUnit.handlerReview = handlerReviewFixture('returned');
    failedUnit.handlerDecision = handlerDecisionFixture('returned');
    failedUnit.retryAttempts = [retryAttemptFixture('failed')];
    expect(decodeOrchestrationNativeQueryV2(failed).workUnits[0]!.retryAttempts[0]).toMatchObject({
      launchRequestedAt: '2026-08-04T00:00:29Z',
      failureReason: 'retry_terminal_launch_failed',
    });

    const absent = implementerOutcomeNativeFixture();
    expect(decodeOrchestrationNativeQueryV2(absent).workUnits[0]!.retryAttempts).toEqual([]);

    const structurallyExtensible = implementerOutcomeNativeFixture();
    const extensibleUnit = (structurallyExtensible.workUnits as Array<Record<string, unknown>>)[0]!;
    extensibleUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
    extensibleUnit.handlerReview = handlerReviewFixture('returned');
    extensibleUnit.handlerDecision = handlerDecisionFixture('returned');
    extensibleUnit.retryAttempts = [{ ...retryAttemptFixture('partial'), ordinal: 2, retryAttemptId: 'retry-attempt-2' }];
    expect(() => decodeOrchestrationNativeQueryV2(structurallyExtensible)).toThrow(
      'exact predecessor history member',
    );

    const malformed = [
      { ...retryAttemptFixture('ready'), originAttemptId: 'foreign-attempt' },
      { ...retryAttemptFixture('ready'), candidateCommitId: 'private-candidate' },
      { ...retryAttemptFixture('ready'), inventedField: true },
      { ...retryAttemptFixture('ready'), failureReason: '' },
      { ...retryAttemptFixture('partial'), authorizedAt: '2026-08-04T00:00:20Z' },
      {
        ...retryAttemptFixture('partial'),
        launchRequestedAt: '2026-08-04T00:00:29Z',
        retryReadyAt: '2026-08-04T00:00:30Z',
        launchAcceptedAt: undefined,
      },
      { ...retryAttemptFixture('ready'), failureReason: 'retry_failed' },
      { ...retryAttemptFixture('partial'), providerActivationObservedAt: '2026-08-04T00:00:28Z', launchRequestedAt: undefined },
      { ...retryAttemptFixture('ready'), providerActivationObservedAt: '2026-08-04T00:00:29Z' },
    ];
    for (const retryAttempt of malformed) {
      const malformedValue = implementerOutcomeNativeFixture();
      const malformedUnit = (malformedValue.workUnits as Array<Record<string, unknown>>)[0]!;
      malformedUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
      malformedUnit.handlerReview = handlerReviewFixture('returned');
      malformedUnit.handlerDecision = handlerDecisionFixture('returned');
      malformedUnit.retryAttempts = [retryAttempt];
      expect(() => decodeOrchestrationNativeQueryV2(malformedValue)).toThrow(
        'Invalid orchestration native query',
      );
    }

    const missingDecision = implementerOutcomeNativeFixture();
    const missingDecisionUnit = (missingDecision.workUnits as Array<Record<string, unknown>>)[0]!;
    missingDecisionUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
    missingDecisionUnit.retryAttempts = [retryAttemptFixture('partial')];
    expect(() => decodeOrchestrationNativeQueryV2(missingDecision)).toThrow(
      'retry attempt requires a returned Handler decision',
    );

    const acceptedDecision = implementerOutcomeNativeFixture();
    const acceptedDecisionUnit = (acceptedDecision.workUnits as Array<Record<string, unknown>>)[0]!;
    acceptedDecisionUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
    acceptedDecisionUnit.handlerReview = handlerReviewFixture('accepted');
    acceptedDecisionUnit.handlerDecision = handlerDecisionFixture('accepted');
    acceptedDecisionUnit.retryAttempts = [retryAttemptFixture('partial')];
    expect(() => decodeOrchestrationNativeQueryV2(acceptedDecision)).toThrow(
      'retry attempt requires a returned Handler decision',
    );
  });

  it('requires generalized retry authorization from the exact predecessor and preserves dispositions', () => {
    const meaningful = implementerOutcomeNativeFixture();
    const meaningfulUnit = (meaningful.workUnits as Array<Record<string, unknown>>)[0]!;
    meaningfulUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
    meaningfulUnit.handlerReview = handlerReviewFixture('returned');
    meaningfulUnit.handlerDecision = handlerDecisionFixture('returned');
    delete (meaningfulUnit.handlerDecision as Record<string, unknown>).retryRequiredAt;
    (meaningfulUnit as Record<string, unknown>).attemptHistory = [
      {
        ordinal: 0,
        attemptId: 'attempt-1',
        implementerOutcome: meaningfulUnit.implementerOutcome,
        handlerReview: meaningfulUnit.handlerReview,
        handlerDecision: meaningfulUnit.handlerDecision,
        incompleteDisposition: {
          attemptId: 'attempt-1',
          reviewInvocationId: 'review-invocation-1',
          decisionFingerprint: 'decision-1',
          classification: 'refinement_needed',
          meaningfulProgress: true,
          recordedAt: '2026-08-04T00:00:18Z',
          nextAttemptAuthorizedAt: '2026-08-04T00:00:19Z',
        },
      },
    ];
    meaningfulUnit.retryAttempts = [retryAttemptFixture('partial')];
    const decoded = decodeOrchestrationNativeQueryV2(meaningful);
    const model = composeProductOrchestrationReadModels(
      nativeQueryProductCompositionInputV2(decoded),
    ).epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!;
    expect(model.attemptHistory[0]!.incompleteDisposition).toMatchObject({
      classification: 'refinement_needed',
      meaningfulProgress: true,
      nextAttemptAuthorizedAt: '2026-08-04T00:00:19Z',
    });

    const gapped = implementerOutcomeNativeFixture();
    const gappedUnit = (gapped.workUnits as Array<Record<string, unknown>>)[0]!;
    gappedUnit.attemptHistory = [
      { ordinal: 0, attemptId: 'attempt-1', implementerOutcome: implementerOutcomeFixture('review_ready') },
      { ordinal: 2, attemptId: 'attempt-3', implementerOutcome: { ...implementerOutcomeFixture('review_ready'), attemptId: 'attempt-3' } },
    ];
    expect(() => decodeOrchestrationNativeQueryV2(gapped)).toThrow(
      'strictly ordered without gaps',
    );
  });

  it('projects factual Handback phases and structured movement while failing closed on impossible effects', () => {
    const partial = implementerOutcomeNativeFixture();
    const partialUnit = (partial.workUnits as Array<Record<string, unknown>>)[0]!;
    partialUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
    partialUnit.handlerReview = handlerReviewFixture('returned');
    partialUnit.handlerDecision = handlerDecisionFixture('returned');
    delete (partialUnit.handlerDecision as Record<string, unknown>).retryRequiredAt;
    partialUnit.attemptHistory = [
      {
        ordinal: 0,
        attemptId: 'attempt-1',
        implementerOutcome: partialUnit.implementerOutcome,
        handlerReview: partialUnit.handlerReview,
        handlerDecision: partialUnit.handlerDecision,
        incompleteDisposition: {
          attemptId: 'attempt-1',
          reviewInvocationId: 'review-invocation-1',
          decisionFingerprint: 'decision-1',
          classification: 'blocked',
          meaningfulProgress: false,
          recordedAt: '2026-08-04T00:00:18Z',
          noProgressHandback: {
            handbackId: 'handback-1',
            sourceAttemptId: 'attempt-1',
            sourceReviewInvocationId: 'review-invocation-1',
            contextFingerprint: 'context-1',
            persistedAt: '2026-08-04T00:00:19Z',
            deliveryIntendedAt: '2026-08-04T00:00:20Z',
            sprintRunnerDelivery: {
              deliveryRequestedAt: '2026-08-04T00:00:21Z',
            },
          },
        },
      },
    ];
    const partialModel = composeProductOrchestrationReadModels(
      nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(partial)),
    ).epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!;
    expect(partialModel.attemptHistory[0]!.incompleteDisposition?.noProgressHandback).toMatchObject({
      persistedAt: '2026-08-04T00:00:19Z',
      sprintRunnerDelivery: { deliveryRequestedAt: '2026-08-04T00:00:21Z' },
    });

    const reopened = JSON.parse(JSON.stringify(partial)) as Record<string, unknown>;
    const reopenedDisposition = (
      ((reopened.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!
        .incompleteDisposition as Record<string, unknown>
    );
    const reopenedHandback = reopenedDisposition.noProgressHandback as Record<string, unknown>;
    reopenedHandback.sprintRunnerDelivery = {
      deliveryRequestedAt: '2026-08-04T00:00:21Z',
      deliveryPersistedAt: '2026-08-04T00:00:22Z',
      harnessBoundAt: '2026-08-04T00:00:23Z',
      launchRequestedAt: '2026-08-04T00:00:24Z',
      launchAcceptedAt: '2026-08-04T00:00:25Z',
      providerActivationObservedAt: '2026-08-04T00:00:26Z',
      semanticReassessmentRecordedAt: '2026-08-04T00:00:27Z',
      selectedMovementKind: 'wait_for_agent_dependency',
      selectedMovement: {
        movementKind: 'wait_for_agent_dependency',
        rationale: 'The current concern remains open.',
        dependencyOwner: 'bounded Work Unit Handler',
        dependencyOwnerClassification: 'work_unit_handler',
        enablingResult: 'A persisted Handler result.',
        resumptionPath: 'Reconcile this exact Handback after that result.',
      },
    };
    reopenedHandback.epicRunnerReceiver = {
      sprintId: 'sprint-fixture',
      epicId: 'epic-fixture',
      deliveryRequestedAt: '2026-08-04T00:00:28Z',
      deliveryPersistedAt: '2026-08-04T00:00:29Z',
      harnessBoundAt: '2026-08-04T00:00:30Z',
      launchRequestedAt: '2026-08-04T00:00:31Z',
      launchAcceptedAt: '2026-08-04T00:00:32Z',
      semanticReassessmentRecordedAt: '2026-08-04T00:00:33Z',
      disposition: {
        movementKind: 'return_context_to_sprint_runner',
        rationale: 'The concern remains unresolved after Epic reassessment.',
        downstreamRequest: {
          target: 'sprint_runner',
          request: 'Reconsider the same Sprint-local concern.',
          resumptionPath: 'Resume from the unchanged concern.',
        },
      },
    };
    const reopenedModel = composeProductOrchestrationReadModels(
      nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(reopened)),
    ).epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!;
    expect(reopenedModel.attemptHistory[0]!.incompleteDisposition?.noProgressHandback?.sprintRunnerDelivery).toMatchObject({
      launchAcceptedAt: '2026-08-04T00:00:25Z',
      selectedMovement: {
        dependencyOwner: 'bounded Work Unit Handler',
        enablingResult: 'A persisted Handler result.',
        resumptionPath: 'Reconcile this exact Handback after that result.',
      },
    });
    expect(reopenedModel.attemptHistory[0]!.incompleteDisposition?.noProgressHandback?.epicRunnerReceiver).toMatchObject({
      sprintId: 'sprint-fixture',
      epicId: 'epic-fixture',
      disposition: { movementKind: 'return_context_to_sprint_runner' },
    });

    const attention = JSON.parse(JSON.stringify(reopened)) as Record<string, unknown>;
    const attentionHandback = (((attention.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>).noProgressHandback as Record<string, unknown>;
    const attentionReceiver = attentionHandback.epicRunnerReceiver as Record<string, unknown>;
    delete attentionReceiver.disposition;
    attentionReceiver.disposition = {
      movementKind: 'human_or_external_attention',
      rationale: 'The concern needs authority outside the Epic Runner.',
      humanExternalAttention: {
        reason: 'A bounded authority decision is needed.',
        authorityNeeded: 'External dependency owner.',
        evidenceContext: 'The exact Sprint concern remains unresolved.',
        resumptionPath: 'Resume from the unchanged Sprint concern.',
      },
    };
    const attentionRead = composeProductOrchestrationReadModels(
      nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(attention)),
    );
    expect(attentionRead.epics[0]!.epicEscalationReceivers?.[0]!.disposition).toMatchObject({
      movementKind: 'human_or_external_attention',
      humanExternalAttention: { authorityNeeded: 'External dependency owner.' },
    });

    for (const movement of [
      {
        selectedMovementKind: 'continue_eligible_work',
        selectedMovement: {
          movementKind: 'continue_eligible_work',
          rationale: 'Another authorized responsibility can proceed.',
          eligibleWorkSummary: 'Continue the independent Work Unit.',
        },
      },
      {
        selectedMovementKind: 'local_exhaustion_escalate',
        selectedMovement: {
          movementKind: 'local_exhaustion_escalate',
          rationale: 'No further local Sprint movement is recorded.',
          localExhaustionSummary: 'Local Sprint Runner options are exhausted.',
        },
        escalationIntentRecordedAt: '2026-08-04T00:00:27Z',
        escalationDeliveryRequestedAt: '2026-08-04T00:00:28Z',
      },
    ]) {
      const value = JSON.parse(JSON.stringify(reopened)) as Record<string, unknown>;
      const disposition = ((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>;
      const handback = disposition.noProgressHandback as Record<string, unknown>;
      const delivery = handback.sprintRunnerDelivery as Record<string, unknown>;
      delete delivery.selectedMovement;
      delete delivery.selectedMovementKind;
      delete delivery.escalationIntentRecordedAt;
      delete delivery.escalationDeliveryRequestedAt;
      Object.assign(delivery, movement);
      const model = composeProductOrchestrationReadModels(
        nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(value)),
      ).epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!;
      expect(model.attemptHistory[0]!.incompleteDisposition?.noProgressHandback?.sprintRunnerDelivery).toMatchObject(movement);
    }

    const boundedMovement = JSON.parse(JSON.stringify(reopened)) as Record<string, unknown>;
    const boundedDelivery = (((boundedMovement.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>).noProgressHandback as Record<string, unknown>;
    boundedDelivery.sprintRunnerDelivery = {
      deliveryRequestedAt: '2026-08-04T00:00:21Z',
      deliveryPersistedAt: '2026-08-04T00:00:22Z',
      harnessBoundAt: '2026-08-04T00:00:23Z',
      launchRequestedAt: '2026-08-04T00:00:24Z',
      launchAcceptedAt: '2026-08-04T00:00:25Z',
      semanticReassessmentRecordedAt: '2026-08-04T00:00:27Z',
      selectedMovementKind: 'future_bounded_move',
      selectedMovement: {
        movementKind: 'future_bounded_move',
        rationale: 'The concern remains open.',
        boundedDetails: [
          { label: 'eligibleWorkSummary', value: 'Alternate-shaped detail.' },
          { label: 'dependencyOwner', value: 'Owner-shaped detail.' },
          { label: 'dependencyOwnerClassification', value: 'work_unit_handler' },
          { label: 'enablingResult', value: 'Enabling-shaped detail.' },
          { label: 'resumptionPath', value: 'Resumption-shaped detail.' },
          { label: 'localExhaustionSummary', value: 'Exhaustion-shaped detail.' },
        ],
      },
    };
    expect(
      composeProductOrchestrationReadModels(
        nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(boundedMovement)),
      ).epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!.attemptHistory[0]!.incompleteDisposition?.noProgressHandback?.sprintRunnerDelivery,
    ).toMatchObject({
      selectedMovement: {
        movementKind: 'future_bounded_move',
        rationale: 'The concern remains open.',
        boundedDetails: [
          { label: 'eligibleWorkSummary', value: 'Alternate-shaped detail.' },
          { label: 'dependencyOwner', value: 'Owner-shaped detail.' },
          { label: 'dependencyOwnerClassification', value: 'work_unit_handler' },
          { label: 'enablingResult', value: 'Enabling-shaped detail.' },
          { label: 'resumptionPath', value: 'Resumption-shaped detail.' },
          { label: 'localExhaustionSummary', value: 'Exhaustion-shaped detail.' },
        ],
      },
    });

    const invalid = [
      (value: Record<string, unknown>) => {
        const delivery = ((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>;
        (delivery.noProgressHandback as Record<string, unknown>).sprintRunnerDelivery = { deliveryRequestedAt: '2026-08-04T00:00:21Z', launchAcceptedAt: '2026-08-04T00:00:25Z' };
      },
      (value: Record<string, unknown>) => {
        const delivery = ((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>;
        (delivery.noProgressHandback as Record<string, unknown>).sprintRunnerDelivery = { deliveryRequestedAt: '2026-08-04T00:00:21Z', deliveryPersistedAt: '2026-08-04T00:00:20Z' };
      },
      (value: Record<string, unknown>) => {
        const delivery = ((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>;
        (delivery.noProgressHandback as Record<string, unknown>).sprintRunnerDelivery = { deliveryRequestedAt: '2026-08-04T00:00:21Z', deliveryPersistedAt: '2026-08-04T00:00:22Z', harnessBoundAt: '2026-08-04T00:00:23Z', launchRequestedAt: '2026-08-04T00:00:24Z', launchAcceptedAt: '2026-08-04T00:00:25Z', semanticReassessmentRecordedAt: '2026-08-04T00:00:26Z', selectedMovementKind: 'wait_for_agent_dependency', selectedMovement: { movementKind: 'wait_for_agent_dependency', rationale: 'x', dependencyOwner: 'human approval', dependencyOwnerClassification: 'work_unit_handler', enablingResult: 'x', resumptionPath: 'x' } };
      },
      (value: Record<string, unknown>) => {
        const handback = (((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>).noProgressHandback as Record<string, unknown>;
        handback.receiverSessionId = 'private';
      },
      (value: Record<string, unknown>) => {
        const handback = (((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>).noProgressHandback as Record<string, unknown>;
        const receiver = handback.epicRunnerReceiver as Record<string, unknown>;
        receiver.epicId = 'foreign-epic';
      },
      (value: Record<string, unknown>) => {
        const handback = (((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>).noProgressHandback as Record<string, unknown>;
        const receiver = handback.epicRunnerReceiver as Record<string, unknown>;
        receiver.launchAcceptedAt = '2026-08-04T00:00:30Z';
        receiver.launchRequestedAt = '2026-08-04T00:00:31Z';
      },
      (value: Record<string, unknown>) => {
        const delivery = ((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>;
        (delivery.noProgressHandback as Record<string, unknown>).sprintRunnerDelivery = { deliveryRequestedAt: '2026-08-04T00:00:21Z', launchAcceptedAt: '2026-08-04T00:00:25Z', semanticReassessmentRecordedAt: '2026-08-04T00:00:26Z', selectedMovementKind: 'future_bounded_move', selectedMovement: { movementKind: 'future_bounded_move', rationale: 'x', dependencyOwner: 'bounded Work Unit Handler' } };
      },
      (value: Record<string, unknown>) => {
        const delivery = ((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>;
        (delivery.noProgressHandback as Record<string, unknown>).sprintRunnerDelivery = { deliveryRequestedAt: '2026-08-04T00:00:21Z', launchAcceptedAt: '2026-08-04T00:00:25Z', semanticReassessmentRecordedAt: '2026-08-04T00:00:26Z', selectedMovementKind: 'future_bounded_move' };
      },
      (value: Record<string, unknown>) => {
        const delivery = ((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>;
        (delivery.noProgressHandback as Record<string, unknown>).sprintRunnerDelivery = { deliveryRequestedAt: '2026-08-04T00:00:21Z', launchAcceptedAt: '2026-08-04T00:00:25Z', semanticReassessmentRecordedAt: '2026-08-04T00:00:26Z', selectedMovement: { movementKind: 'future_bounded_move', rationale: 'x' } };
      },
      (value: Record<string, unknown>) => {
        const delivery = ((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>;
        (delivery.noProgressHandback as Record<string, unknown>).sprintRunnerDelivery = { deliveryRequestedAt: '2026-08-04T00:00:21Z', launchAcceptedAt: '2026-08-04T00:00:25Z', semanticReassessmentRecordedAt: '2026-08-04T00:00:26Z', selectedMovementKind: 'local_exhaustion_escalate', selectedMovement: { movementKind: 'local_exhaustion_escalate', rationale: 'x', localExhaustionSummary: 'x' }, escalationDeliveryRequestedAt: '2026-08-04T00:00:28Z' };
      },
      (value: Record<string, unknown>) => {
        const delivery = ((value.workUnits as Array<Record<string, unknown>>)[0]!.attemptHistory as Array<Record<string, unknown>>)[0]!.incompleteDisposition as Record<string, unknown>;
        (delivery.noProgressHandback as Record<string, unknown>).sprintRunnerDelivery = { deliveryRequestedAt: '2026-08-04T00:00:21Z', launchAcceptedAt: '2026-08-04T00:00:25Z', semanticReassessmentRecordedAt: '2026-08-04T00:00:26Z', selectedMovementKind: 'local_exhaustion_escalate', selectedMovement: { movementKind: 'local_exhaustion_escalate', rationale: 'x', localExhaustionSummary: 'x' }, escalationIntentRecordedAt: '2026-08-04T00:00:28Z', escalationDeliveryRequestedAt: '2026-08-04T00:00:27Z' };
      },
    ];
    for (const mutate of invalid) {
      const value = JSON.parse(JSON.stringify(reopened)) as Record<string, unknown>;
      mutate(value);
      expect(() => {
        const decoded = decodeOrchestrationNativeQueryV2(value);
        composeProductOrchestrationReadModels(nativeQueryProductCompositionInputV2(decoded));
      }).toThrow();
    }
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

  it('projects only exact productive Work Unit Session turns and application-owned inspection evidence', () => {
    const value = implementerOutcomeNativeFixture();
    const unit = (value.workUnits as Array<Record<string, unknown>>)[0]!;
    unit.implementerOutcome = implementerOutcomeFixture('review_ready');
    unit.handlerReview = handlerReviewFixture('accepted');
    value.workUnitInspections = [workUnitInspectionFixture()];

    const query = decodeOrchestrationNativeQueryV2(value);
    const inspection = query.workUnitInspections[0]!;
    expect(inspection.activities).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          role: 'handler',
          agentSessionId: 'handler-session-1',
          invocationId: 'review-invocation-1',
          primaryStage: 'handler_review',
          applicationSummary: expect.objectContaining({
            peerEvidenceActivityIds: ['activity-reporting'],
          }),
        }),
        expect.objectContaining({
          role: 'implementer',
          agentSessionId: 'implementer-session-1',
          invocationId: 'reporting-invocation-1',
          primaryStage: 'implementer_reporting',
        }),
      ]),
    );
    expect(inspection.fileEvidence).toMatchObject({
      status: 'available',
      owner: 'application',
      sourceActivityId: 'activity-reporting',
    });
    expect(inspection.testEvidence).toMatchObject({ owner: 'application' });

    const input = nativeQueryProductCompositionInputV2(query);
    expect(input.events.agentSessionReferences).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          agentSessionId: 'handler-session-1',
          agentInvocationId: 'review-invocation-1',
          semanticRole: 'work_unit_handler',
        }),
        expect.objectContaining({
          agentSessionId: 'implementer-session-1',
          agentInvocationId: 'reporting-invocation-1',
          semanticRole: 'work_unit_implementer',
        }),
      ]),
    );
    const readModels = composeProductOrchestrationReadModels(input);
    expect(readModels.epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!.inspection).toMatchObject({
      fileEvidence: { status: 'available' },
    });
    expect(
      presentProductOrchestrations(readModels).epics[0]!.plan.items[0]!.workspaceAdjunct
        ?.workUnitSessions,
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          sessionId: 'implementer-session-1',
          invocationId: 'reporting-invocation-1',
          role: 'implementer',
        }),
      ]),
    );

    const foreign = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    ((foreign.workUnitInspections as Array<Record<string, unknown>>)[0]!.activities as Array<Record<string, unknown>>)[0]!
      .invocationId = 'foreign-invocation';
    expect(() => decodeOrchestrationNativeQueryV2(foreign)).toThrow(
      'Work Unit inspection activity is foreign, stale, or duplicated',
    );
  });
});

function implementerOutcomeNativeFixture(): Record<string, unknown> {
  const value = fixture('valid-initiated-epic.json') as Record<string, unknown>;
  value.workUnitMaterializations = [
    {
      materializationId: 'materialization-1',
      planningPointId: 'point-1',
      acceptedRevisionId: 'accepted-revision-1',
      epicId: 'epic-fixture',
      sprintId: 'sprint-fixture',
      workSliceId: 'slice-1',
      authorizationRecordedAt: '2026-08-03T00:00:00Z',
      attemptRecordedAt: '2026-08-03T00:00:01Z',
      workUnitsCreatedAt: '2026-08-03T00:00:02Z',
      relationshipsCompletedAt: '2026-08-03T00:00:03Z',
      settledAt: '2026-08-03T00:00:04Z',
    },
  ];
  value.workUnits = [
    {
      workUnitId: 'unit-1',
      materializationId: 'materialization-1',
      workSliceId: 'slice-1',
      acceptedRevisionId: 'accepted-revision-1',
      laneOrdinal: 0,
      laneTitle: 'Bounded responsibility',
      specification: 'Implement and report one bounded outcome.',
      handlerActivation: {
        attemptId: 'attempt-1',
        handlerSessionId: 'handler-session-1',
        handlerInvocationId: 'handler-invocation-1',
        handlerHarnessRevisionId: 'handler-revision-1',
        eligibilityState: 'eligible',
        requestedAt: '2026-08-03T00:01:00Z',
        authorizedAt: '2026-08-03T00:01:01Z',
        attemptCreatedAt: '2026-08-03T00:01:02Z',
        executionSupportGrantedAt: '2026-08-03T00:01:03Z',
        isolatedWorktreeReadyAt: '2026-08-03T00:01:04Z',
        handlerSessionCreatedAt: '2026-08-03T00:01:05Z',
        handlerInvocationPreparedAt: '2026-08-03T00:01:06Z',
        handlerHarnessBoundAt: '2026-08-03T00:01:07Z',
        launchRequestedAt: '2026-08-03T00:01:08Z',
        launchAcceptedAt: '2026-08-03T00:01:09Z',
        handlerReadyAt: '2026-08-03T00:01:10Z',
      },
      actionContinuation: {
        attemptId: 'attempt-1',
        handlerSessionId: 'handler-session-1',
        originalHandlerInvocationId: 'handler-invocation-1',
        actionInvocationId: 'handler-action-1',
        actionHarnessRevisionId: 'handler-action-revision-1',
        requestedAt: '2026-08-03T00:01:11Z',
        authorizedAt: '2026-08-03T00:01:12Z',
        invocationPreparedAt: '2026-08-03T00:01:13Z',
        harnessBoundAt: '2026-08-03T00:01:14Z',
        launchRequestedAt: '2026-08-03T00:01:15Z',
        launchAcceptedAt: '2026-08-03T00:01:16Z',
        actionReadyAt: '2026-08-03T00:01:17Z',
      },
      implementerActivation: {
        attemptId: 'attempt-1',
        handlerActionInvocationId: 'handler-action-1',
        implementerSessionId: 'implementer-session-1',
        implementerInvocationId: 'implementer-invocation-1',
        implementerHarnessRevisionId: 'implementer-revision-1',
        requestedAt: '2026-08-03T00:01:18Z',
        authorizedAt: '2026-08-03T00:01:19Z',
        executionSupportGrantedAt: '2026-08-03T00:01:20Z',
        isolatedWorktreeReadyAt: '2026-08-03T00:01:21Z',
        implementerSessionCreatedAt: '2026-08-03T00:01:22Z',
        implementerInvocationPreparedAt: '2026-08-03T00:01:23Z',
        implementerHarnessBoundAt: '2026-08-03T00:01:24Z',
        launchRequestedAt: '2026-08-03T00:01:25Z',
        launchAcceptedAt: '2026-08-03T00:01:26Z',
        implementerReadyAt: '2026-08-03T00:01:27Z',
      },
      attemptHistory: [],
      retryAttempts: [],
    },
  ];
  // Keep the old test call sites concise while exercising only the new serialized history shape.
  const unit = (value.workUnits as Array<Record<string, unknown>>)[0]!;
  const history = () => unit.attemptHistory as Array<Record<string, unknown>>;
  const member = () => {
    const existing = history()[0];
    if (existing) return existing;
    const created: Record<string, unknown> = { ordinal: 0, attemptId: 'attempt-1' };
    history().push(created);
    return created;
  };
  Object.defineProperties(unit, {
    implementerOutcome: { enumerable: false, get: () => member().implementerOutcome, set: (value) => { member().implementerOutcome = value; } },
    handlerReview: { enumerable: false, get: () => member().handlerReview, set: (value) => { member().handlerReview = value; } },
    handlerDecision: { enumerable: false, get: () => member().handlerDecision, set: (value) => { member().handlerDecision = value; } },
  });
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
      relationshipId: 'lane',
      materializationId: 'materialization-1',
      relationshipKind: 'lane',
      fromId: 'slice-1',
      toId: 'unit-1',
      ordinal: 0,
    },
    {
      relationshipId: 'order',
      materializationId: 'materialization-1',
      relationshipKind: 'order',
      fromId: 'slice-1',
      toId: 'unit-1',
      ordinal: 0,
    },
  ];
  return value;
}

function primaryAttempt<T extends { readonly attemptHistory: readonly { readonly ordinal: number }[] }>(
  unit: T,
): T['attemptHistory'][number] | undefined {
  return unit.attemptHistory.find((member) => member.ordinal === 0);
}

function workUnitInspectionFixture(): Record<string, unknown> {
  const unavailable = (reason: string) => ({ owner: 'application', reason });
  return {
    workUnitId: 'unit-1',
    materializationId: 'materialization-1',
    activities: [
      { activityId: 'activity-handler', attemptId: 'attempt-1', role: 'handler', agentSessionId: 'handler-session-1', invocationId: 'handler-invocation-1', primaryStage: 'handler_activation' },
      { activityId: 'activity-action', attemptId: 'attempt-1', role: 'handler', agentSessionId: 'handler-session-1', invocationId: 'handler-action-1', primaryStage: 'handler_action' },
      { activityId: 'activity-implementer', attemptId: 'attempt-1', role: 'implementer', agentSessionId: 'implementer-session-1', invocationId: 'implementer-invocation-1', primaryStage: 'implementer_activation' },
      {
        activityId: 'activity-reporting', attemptId: 'attempt-1', role: 'implementer', agentSessionId: 'implementer-session-1', invocationId: 'reporting-invocation-1', primaryStage: 'implementer_reporting',
        applicationSummary: {
          owner: 'application',
          applicationEvents: ['submission_recorded', 'file_evidence_recorded', 'semantic_completion_recorded', 'terminal_lifecycle_observed', 'application_acceptance_recorded', 'handler_review_ready'],
          peerEvidenceActivityIds: [],
          mcpCallDetail: unavailable('No application-owned MCP-call detail is available for this reporting turn.'),
        },
      },
      {
        activityId: 'activity-review', attemptId: 'attempt-1', role: 'handler', agentSessionId: 'handler-session-1', invocationId: 'review-invocation-1', primaryStage: 'handler_review',
        applicationSummary: {
          owner: 'application',
          applicationEvents: ['review_delivery_persisted', 'review_judgment_recorded', 'review_lifecycle_observed'],
          peerEvidenceActivityIds: ['activity-reporting'],
          mcpCallDetail: unavailable('No application-owned MCP-call detail is available for this review turn.'),
        },
      },
    ],
    fileEvidence: {
      status: 'available', owner: 'application', sourceActivityId: 'activity-reporting',
      changedFiles: [{ evidenceRef: 'evidence-1', displayName: 'src/feature.ts', changeKind: 'modified', contentFingerprint: 'content-1' }],
    },
    testEvidence: unavailable('No application-owned test-detail evidence is available for this Work Unit.'),
  };
}

function implementerOutcomeFixture(
  state: 'in_progress' | 'failed' | 'canceled' | 'review_ready',
): Record<string, unknown> {
  const outcome: Record<string, unknown> = {
    attemptId: 'attempt-1',
    implementerSessionId: 'implementer-session-1',
    originalImplementerInvocationId: 'implementer-invocation-1',
    reportingInvocationId: 'reporting-invocation-1',
    reportingHarnessRevisionId: 'reporting-revision-1',
    reportingRequestedAt: '2026-08-04T00:00:00Z',
    reportingPreparedAt: '2026-08-04T00:00:01Z',
  };
  if (state === 'in_progress') return outcome;
  Object.assign(outcome, {
    reportingHarnessBoundAt: '2026-08-04T00:00:02Z',
    reportingLaunchRequestedAt: '2026-08-04T00:00:03Z',
    reportingLaunchAcceptedAt: '2026-08-04T00:00:04Z',
    reportingReadyAt: '2026-08-04T00:00:05Z',
  });
  if (state === 'failed' || state === 'canceled') {
    outcome.terminalLifecycle = {
      status: state,
      observedAt: '2026-08-04T00:00:09Z',
    };
    return outcome;
  }
  Object.assign(outcome, {
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
  });
  return outcome;
}

function handlerReviewFixture(
  state: 'pending' | 'accepted' | 'returned' | 'failed',
): Record<string, unknown> {
  const review: Record<string, unknown> = {
    attemptId: 'attempt-1',
    reportingInvocationId: 'reporting-invocation-1',
    handlerSessionId: 'handler-session-1',
    originalHandlerInvocationId: 'handler-invocation-1',
    actionHandlerInvocationId: 'handler-action-1',
    reviewInvocationId: 'review-invocation-1',
    reviewHarnessRevisionId: 'review-revision-1',
    deliveryRequestedAt: '2026-08-04T00:00:12Z',
    deliveryPersistedAt: '2026-08-04T00:00:12Z',
    harnessBoundAt: '2026-08-04T00:00:13Z',
    launchRequestedAt: '2026-08-04T00:00:14Z',
    launchAcceptedAt: '2026-08-04T00:00:15Z',
    reviewReadyAt: '2026-08-04T00:00:16Z',
    delivered: {
      summaryClaim: 'Implemented the bounded change.',
      validationStatementClaim: 'Focused checks passed.',
      changedFiles: [
        {
          evidenceRef: 'evidence-1',
          displayName: 'src/feature.ts',
          changeKind: 'modified',
          contentFingerprint: 'content-1',
        },
      ],
      comparisonFingerprint: 'comparison-1',
      deliveredPayloadFingerprint: 'delivery-1',
    },
  };
  if (state === 'accepted' || state === 'returned') {
    Object.assign(review, {
      semanticJudgment: {
        variant: state === 'accepted' ? 'accept' : 'return',
        ...(state === 'returned'
          ? { reason: { code: 'review_failed', explanation: 'Evidence requires correction.' } }
          : {}),
        fingerprint: 'judgment-1',
        recordedAt: '2026-08-04T00:00:17Z',
      },
      lifecycle: { status: 'completed', observedAt: '2026-08-04T00:00:18Z' },
    });
  } else if (state === 'failed') {
    review.lifecycle = { status: 'failed', observedAt: '2026-08-04T00:00:18Z' };
  }
  return review;
}

function handlerDecisionFixture(variant: 'accepted' | 'returned'): Record<string, unknown> {
  return variant === 'accepted'
    ? {
        attemptId: 'attempt-1',
        reviewInvocationId: 'review-invocation-1',
        variant,
        fingerprint: 'decision-1',
        recordedAt: '2026-08-04T00:00:19Z',
        implementationAcceptedAt: '2026-08-04T00:00:19Z',
      }
    : {
        attemptId: 'attempt-1',
        reviewInvocationId: 'review-invocation-1',
        variant,
        fingerprint: 'decision-1',
        returnReason: { code: 'review_failed', explanation: 'Evidence requires correction.' },
        recordedAt: '2026-08-04T00:00:19Z',
        implementationReturnedAt: '2026-08-04T00:00:19Z',
        retryRequiredAt: '2026-08-04T00:00:19Z',
      };
}

function retryAttemptFixture(state: 'partial' | 'ready' | 'failed'): Record<string, unknown> {
  const retry: Record<string, unknown> = {
    ordinal: 1,
    originAttemptId: 'attempt-1',
    retryAttemptId: 'retry-attempt-1',
    implementerSessionId: 'retry-implementer-session-1',
    implementerInvocationId: 'retry-implementer-invocation-1',
    captureRequestedAt: '2026-08-04T00:00:20Z',
    candidatePinnedAt: '2026-08-04T00:00:21Z',
    authorizedAt: '2026-08-04T00:00:22Z',
    executionSupportGrantedAt: '2026-08-04T00:00:23Z',
    isolatedWorktreeReadyAt: '2026-08-04T00:00:24Z',
    implementerSessionCreatedAt: '2026-08-04T00:00:25Z',
    implementerInvocationPreparedAt: '2026-08-04T00:00:26Z',
    implementerHarnessBoundAt: '2026-08-04T00:00:27Z',
  };
  if (state === 'ready') {
    Object.assign(retry, {
      launchRequestedAt: '2026-08-04T00:00:29Z',
      launchAcceptedAt: '2026-08-04T00:00:30Z',
      providerActivationObservedAt: '2026-08-04T00:00:31Z',
      retryReadyAt: '2026-08-04T00:00:32Z',
    });
  } else if (state === 'failed') {
    Object.assign(retry, {
      launchRequestedAt: '2026-08-04T00:00:29Z',
      failureReason: 'retry_terminal_launch_failed',
    });
  }
  return retry;
}
