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
    expect(absentQuery.workUnits[0]!.implementerOutcome).toBeUndefined();

    const inProgress = implementerOutcomeNativeFixture();
    (inProgress.workUnits as Array<Record<string, unknown>>)[0]!.implementerOutcome =
      implementerOutcomeFixture('in_progress');
    const inProgressInput = nativeQueryProductCompositionInputV2(
      decodeOrchestrationNativeQueryV2(inProgress),
    );
    expect(inProgressInput.referenceIndex.workUnits[0]!.implementerOutcome).toMatchObject({
      reportingRequestedAt: '2026-08-04T00:00:00Z',
      reportingPreparedAt: '2026-08-04T00:00:01Z',
    });
    expect(inProgressInput.referenceIndex.workUnits[0]!.implementerOutcome).not.toHaveProperty(
      'submittedOutcome',
    );

    for (const status of ['failed', 'canceled'] as const) {
      const terminal = implementerOutcomeNativeFixture();
      (terminal.workUnits as Array<Record<string, unknown>>)[0]!.implementerOutcome =
        implementerOutcomeFixture(status);
      expect(
        decodeOrchestrationNativeQueryV2(terminal).workUnits[0]!.implementerOutcome
          ?.terminalLifecycle,
      ).toMatchObject({ status });
    }

    const reviewReady = implementerOutcomeNativeFixture();
    (reviewReady.workUnits as Array<Record<string, unknown>>)[0]!.implementerOutcome =
      implementerOutcomeFixture('review_ready');
    const readyQuery = decodeOrchestrationNativeQueryV2(reviewReady);
    const readyOutcome = readyQuery.workUnits[0]!.implementerOutcome!;
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
        .epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!.implementerOutcome,
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
    expect(pendingQuery.workUnits[0]!.handlerReview).toMatchObject({
      reviewReadyAt: '2026-08-04T00:00:11Z',
      delivered: { comparisonFingerprint: 'comparison-1' },
    });
    const pendingUnitModel = composeProductOrchestrationReadModels(
      nativeQueryProductCompositionInputV2(pendingQuery),
    ).epics[0]!.sprints[0]!.revisionViews[0]!.workUnits[0]!;
    expect(pendingUnitModel.handlerReview?.semanticJudgment).toBeUndefined();
    expect(pendingUnitModel.handlerDecision).toBeUndefined();

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
      expect(query.workUnits[0]!.handlerDecision?.variant).toBe(variant);
      expect(query.workUnits[0]!.handlerReview?.semanticJudgment?.variant).toBe(
        variant === 'accepted' ? 'accept' : 'return',
      );
      expect(query.workUnits[0]!.handlerReview?.lifecycle?.status).toBe('completed');
    }

    const failed = implementerOutcomeNativeFixture();
    const failedUnit = (failed.workUnits as Array<Record<string, unknown>>)[0]!;
    failedUnit.implementerOutcome = implementerOutcomeFixture('review_ready');
    failedUnit.handlerReview = handlerReviewFixture('failed');
    expect(decodeOrchestrationNativeQueryV2(failed).workUnits[0]!.handlerDecision).toBeUndefined();

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

  it('projects productive integration and settlement without populating legacy observed events', () => {
    const value = productiveIntegrationNativeFixture();
    const query = decodeOrchestrationNativeQueryV2(value);
    expect(query.workUnits[0]!.integration).toEqual({
      requestedAt: '2026-08-04T00:00:20Z',
      authorizedAt: '2026-08-04T00:00:20Z',
      progress: { phase: 'recording', recordedAt: '2026-08-04T00:00:24Z' },
      success: { recordedAt: '2026-08-04T00:00:25Z' },
      settlement: { settledAt: '2026-08-04T00:00:25Z' },
      prerequisiteContribution: {
        recordedAt: '2026-08-04T00:00:25Z',
        dependentCount: 1,
      },
    });

    const input = nativeQueryProductCompositionInputV2(query);
    expect(input.events.observedIntegrations).toEqual([]);
    expect(input.events.observedCompletions).toEqual([]);
    expect(input.events.observedHandoffs).toEqual([]);
    const unit = composeProductOrchestrationReadModels(input).epics[0]!.sprints[0]!
      .revisionViews[0]!.workUnits.find(({ workUnitId }) => workUnitId === 'unit-1')!;
    expect(unit.integration?.settlement?.settledAt).toBe('2026-08-04T00:00:25Z');
    expect(unit.observed.integrated).toBe(false);
    expect(unit.observed.responsibilityAccepted).toBe(false);
    expect(unit.presentationState).toBe('not_started');
  });

  it('keeps progressive and attention integration facts non-terminal', () => {
    const progressive = productiveIntegrationNativeFixture();
    const progressiveUnit = (progressive.workUnits as Array<Record<string, unknown>>)[0]!;
    progressiveUnit.integration = {
      requestedAt: '2026-08-04T00:00:20Z',
      authorizedAt: '2026-08-04T00:00:20Z',
      progress: { phase: 'applying', recordedAt: '2026-08-04T00:00:22Z' },
    };
    expect(
      decodeOrchestrationNativeQueryV2(progressive).workUnits[0]!.integration,
    ).not.toHaveProperty('success');

    const attention = productiveIntegrationNativeFixture();
    const attentionUnit = (attention.workUnits as Array<Record<string, unknown>>)[0]!;
    attentionUnit.integration = {
      requestedAt: '2026-08-04T00:00:20Z',
      authorizedAt: '2026-08-04T00:00:20Z',
      progress: { phase: 'preparing', recordedAt: '2026-08-04T00:00:21Z' },
      attention: {
        kind: 'conflict',
        safeCode: 'integration_conflict',
        recordedAt: '2026-08-04T00:00:22Z',
      },
    };
    expect(decodeOrchestrationNativeQueryV2(attention).workUnits[0]!.integration).toMatchObject({
      attention: { kind: 'conflict', safeCode: 'integration_conflict' },
    });
  });

  it.each([
    [
      'unknown field',
      (integration: Record<string, unknown>) =>
        (integration.privateRef = 'refs/heads/main'),
    ],
    [
      'authorization order',
      (integration: Record<string, unknown>) =>
        (integration.authorizedAt = '2026-08-04T00:00:19Z'),
    ],
    [
      'unknown phase',
      (integration: Record<string, unknown>) =>
        ((integration.progress as Record<string, unknown>).phase = 'ref_advanced'),
    ],
    [
      'attention terminal collision',
      (integration: Record<string, unknown>) =>
        (integration.attention = {
          kind: 'failure',
          safeCode: 'integration_failure',
          recordedAt: '2026-08-04T00:00:25Z',
        }),
    ],
    [
      'settlement without success',
      (integration: Record<string, unknown>) => delete integration.success,
    ],
    [
      'success without settlement',
      (integration: Record<string, unknown>) => delete integration.settlement,
    ],
    [
      'contribution before settlement',
      (integration: Record<string, unknown>) => {
        (integration.settlement as Record<string, unknown>).settledAt =
          '2026-08-04T00:00:27Z';
        (integration.prerequisiteContribution as Record<string, unknown>).recordedAt =
          '2026-08-04T00:00:26Z';
      },
    ],
    [
      'foreign dependent count',
      (integration: Record<string, unknown>) =>
        ((integration.prerequisiteContribution as Record<string, unknown>).dependentCount = 2),
    ],
  ] as const)('rejects productive integration %s', (_label, mutate) => {
    const value = productiveIntegrationNativeFixture();
    const unit = (value.workUnits as Array<Record<string, unknown>>)[0]!;
    mutate(unit.integration as Record<string, unknown>);
    expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow(
      'Invalid orchestration native query',
    );
  });

  it('rejects productive integration without the exact accepted Handler decision', () => {
    const value = productiveIntegrationNativeFixture();
    delete (value.workUnits as Array<Record<string, unknown>>)[0]!.handlerDecision;
    expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow(
      'Productive integration requires an accepted Handler decision',
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

function productiveIntegrationNativeFixture(): Record<string, unknown> {
  const value = implementerOutcomeNativeFixture();
  const units = value.workUnits as Array<Record<string, unknown>>;
  units[0]!.implementerOutcome = implementerOutcomeFixture('review_ready');
  units[0]!.handlerReview = handlerReviewFixture('accepted');
  units[0]!.handlerDecision = handlerDecisionFixture('accepted');
  units[0]!.integration = {
    requestedAt: '2026-08-04T00:00:20Z',
    authorizedAt: '2026-08-04T00:00:20Z',
    progress: { phase: 'recording', recordedAt: '2026-08-04T00:00:24Z' },
    success: { recordedAt: '2026-08-04T00:00:25Z' },
    settlement: { settledAt: '2026-08-04T00:00:25Z' },
    prerequisiteContribution: {
      recordedAt: '2026-08-04T00:00:25Z',
      dependentCount: 1,
    },
  };
  units.push({
    workUnitId: 'unit-2',
    materializationId: 'materialization-1',
    workSliceId: 'slice-1',
    acceptedRevisionId: 'accepted-revision-1',
    laneOrdinal: 1,
    laneTitle: 'Dependent responsibility',
    specification: 'Wait for the prerequisite contribution.',
  });
  const relationships = value.workUnitRelationships as Array<Record<string, unknown>>;
  relationships.push(
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
      relationshipId: 'dependency-1',
      materializationId: 'materialization-1',
      relationshipKind: 'depends_on',
      fromId: 'unit-2',
      toId: 'unit-1',
    },
  );
  return value;
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
        reviewInvocationId: 'review-invocation-1',
        variant,
        fingerprint: 'decision-1',
        recordedAt: '2026-08-04T00:00:19Z',
        implementationAcceptedAt: '2026-08-04T00:00:19Z',
      }
    : {
        reviewInvocationId: 'review-invocation-1',
        variant,
        fingerprint: 'decision-1',
        returnReason: { code: 'review_failed', explanation: 'Evidence requires correction.' },
        recordedAt: '2026-08-04T00:00:19Z',
        implementationReturnedAt: '2026-08-04T00:00:19Z',
        retryRequiredAt: '2026-08-04T00:00:19Z',
      };
}
