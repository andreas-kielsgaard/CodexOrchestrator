import type { EpicPlanProposalSnapshot } from './epicPlanProposal';
import { AGENT_CONTROL_CONTRACTS_V1 } from './agentControl';
import { ARTIFACT_ACCESS_CONTRACTS_V1 } from './artifactAccess';
import { ORCHESTRATION_EVENTS_V1 } from './orchestrationEvents';
import type {
  ProductReadCompositionInputV1,
  ProductWorkUnitActionContinuationV1,
  ProductWorkUnitHandlerActivationV1,
  ProductWorkUnitHandlerDecisionV1,
  ProductWorkUnitIncompleteDispositionV1,
  ProductWorkUnitIntegrationV1,
  ProductWorkUnitHandlerReviewV1,
  ProductWorkUnitImplementerActivationV1,
  ProductWorkUnitImplementerOutcomeV1,
  ProductWorkUnitRetryAttemptV1,
  ProductSprintRunnerHandbackDeliveryV1,
  ProductSprintRunnerHandbackBoundedDetailV1,
  ProductSprintRunnerHandbackUnknownMovementKindV1,
  ProductSprintRunnerHandbackDependencyOwnerClassificationV1,
  ProductSprintRunnerHandbackMovementV1,
  ProductEpicEscalationReceiverV1,
  ProductEpicEscalationDispositionV1,
} from './productReadModels';

export const ORCHESTRATION_NATIVE_QUERY_V2 = 'orchestration-native-query/v2' as const;

export interface OrchestrationNativeQueryV2 {
  readonly contractVersion: typeof ORCHESTRATION_NATIVE_QUERY_V2;
  readonly generatedAt: string;
  readonly planningDrafts: readonly NativePlanningDraftV1[];
  readonly agentSessionAssociations: readonly NativeAgentSessionAssociationV1[];
  readonly proposalRevisions: readonly NativeProposalRevisionV1[];
  readonly recordedProposalEvents: readonly NativeRecordedProposalEventV1[];
  readonly provenanceLinks: readonly NativeProvenanceLinkV1[];
  readonly initiationCommands: readonly NativeInitiationCommandV2[];
  readonly initiationResults: readonly NativeInitiationResultV2[];
  readonly initiationEvents: readonly NativeInitiationEventV2[];
  readonly initiationProvenance: readonly NativeInitiationProvenanceV2[];
  readonly materialSnapshots: readonly NativeMaterialSnapshotV2[];
  readonly initiatedEpics: readonly NativeInitiatedEpicV2[];
  readonly initiatedSprints: readonly NativeInitiatedSprintV2[];
  readonly fileReviewDocuments: readonly NativeFileReviewDocumentV1[];
  readonly workUnitMaterializations: readonly NativeWorkUnitMaterializationV1[];
  readonly workUnits: readonly NativeMaterializedWorkUnitV1[];
  readonly workUnitRelationships: readonly NativeWorkUnitRelationshipV1[];
  readonly dependencyActivationIntents: readonly NativeWorkUnitDependencyActivationIntentV1[];
  readonly workUnitExecutionStates: readonly NativeWorkUnitExecutionStateV1[];
  readonly workSliceExecutionGraphCompletions: readonly NativeWorkSliceExecutionGraphCompletionV1[];
  readonly workSliceExecutionSettlements: readonly NativeWorkSliceExecutionSettlementV1[];
  readonly workSlicePlanningPointExecutionSettlements: readonly NativeWorkSlicePlanningPointExecutionSettlementV1[];
  readonly workSliceExecutionAttentions: readonly NativeWorkSliceExecutionAttentionV1[];
}
export interface NativeWorkUnitExecutionStateV1 {
  readonly workUnitId: string;
  readonly materializationId: string;
  readonly acceptedRevisionId: string;
  readonly state: 'waiting_on_prerequisites' | 'ready' | 'active' | 'retry_authorized' | 'handed_back' | 'settled' | 'attention';
  readonly recordedAt: string;
}
export interface NativeWorkSliceExecutionGraphCompletionV1 { readonly materializationId: string; readonly acceptedRevisionId: string; readonly completedAt: string; }
export interface NativeWorkSliceExecutionSettlementV1 { readonly materializationId: string; readonly graphCompletionMaterializationId: string; readonly settledAt: string; }
export interface NativeWorkSlicePlanningPointExecutionSettlementV1 { readonly planningPointId: string; readonly materializationId: string; readonly workSliceExecutionMaterializationId: string; readonly settledAt: string; }
export interface NativeWorkSliceExecutionAttentionV1 { readonly materializationId: string; readonly recordedAt: string; }
export interface NativeWorkUnitDependencyActivationIntentV1 {
  readonly workUnitId: string;
  readonly materializationId: string;
  readonly acceptedRevisionId: string;
  readonly eligibilityState: 'blocked' | 'eligible';
  readonly blockedReason?: string;
  readonly eligibilityRecordedAt: string;
  readonly activationIntendedAt?: string;
}
export interface NativeWorkUnitMaterializationV1 {
  readonly materializationId: string;
  readonly planningPointId: string;
  readonly acceptedRevisionId: string;
  readonly epicId: string;
  readonly sprintId: string;
  readonly workSliceId: string;
  readonly authorizationRecordedAt: string;
  readonly attemptRecordedAt?: string;
  readonly workUnitsCreatedAt?: string;
  readonly relationshipsCompletedAt?: string;
  readonly settledAt?: string;
}
export interface NativeMaterializedWorkUnitV1 {
  readonly workUnitId: string;
  readonly materializationId: string;
  readonly workSliceId: string;
  readonly acceptedRevisionId: string;
  readonly laneOrdinal: number;
  readonly laneTitle: string;
  readonly specification: string;
  readonly handlerActivation?: NativeWorkUnitHandlerActivationV1;
  readonly actionContinuation?: NativeWorkUnitHandlerActionContinuationV1;
  readonly implementerActivation?: NativeWorkUnitImplementerActivationV1;
  readonly attemptHistory: readonly NativeWorkUnitAttemptHistoryV1[];
  readonly retryAttempts: readonly NativeWorkUnitRetryAttemptV1[];
  readonly integration?: NativeWorkUnitIntegrationV1;
}
export interface NativeWorkUnitAttemptHistoryV1 {
  readonly ordinal: number;
  readonly attemptId: string;
  readonly implementerOutcome?: NativeWorkUnitImplementerOutcomeV1;
  readonly handlerReview?: NativeWorkUnitHandlerReviewV1;
  readonly handlerDecision?: NativeWorkUnitHandlerDecisionV1;
  readonly incompleteDisposition?: NativeWorkUnitIncompleteDispositionV1;
}
export interface NativeWorkUnitHandlerActivationV1 {
  readonly attemptId: string;
  readonly handlerSessionId?: string;
  readonly handlerInvocationId?: string;
  readonly handlerHarnessRevisionId?: string;
  readonly eligibilityState: 'blocked' | 'eligible';
  readonly blockedReason?: string;
  readonly requestedAt?: string;
  readonly authorizedAt?: string;
  readonly attemptCreatedAt?: string;
  readonly executionSupportGrantedAt?: string;
  readonly isolatedWorktreeReadyAt?: string;
  readonly handlerSessionCreatedAt?: string;
  readonly handlerInvocationPreparedAt?: string;
  readonly handlerHarnessBoundAt?: string;
  readonly launchRequestedAt?: string;
  readonly launchAcceptedAt?: string;
  readonly providerActivationObservedAt?: string;
  readonly handlerReadyAt?: string;
  readonly failureReason?: string;
}
export interface NativeWorkUnitHandlerActionContinuationV1 {
  readonly attemptId: string;
  readonly handlerSessionId: string;
  readonly originalHandlerInvocationId: string;
  readonly actionInvocationId: string;
  readonly actionHarnessRevisionId: string;
  readonly requestedAt: string;
  readonly authorizedAt?: string;
  readonly invocationPreparedAt?: string;
  readonly harnessBoundAt?: string;
  readonly launchRequestedAt?: string;
  readonly launchAcceptedAt?: string;
  readonly providerActivationObservedAt?: string;
  readonly actionReadyAt?: string;
  readonly blockedReason?: string;
  readonly failureReason?: string;
}
export interface NativeWorkUnitImplementerActivationV1 {
  readonly attemptId: string;
  readonly handlerActionInvocationId: string;
  readonly implementerSessionId: string;
  readonly implementerInvocationId: string;
  readonly implementerHarnessRevisionId: string;
  readonly requestedAt: string;
  readonly authorizedAt?: string;
  readonly executionSupportGrantedAt?: string;
  readonly isolatedWorktreeReadyAt?: string;
  readonly implementerSessionCreatedAt?: string;
  readonly implementerInvocationPreparedAt?: string;
  readonly implementerHarnessBoundAt?: string;
  readonly launchRequestedAt?: string;
  readonly launchAcceptedAt?: string;
  readonly providerActivationObservedAt?: string;
  readonly implementerReadyAt?: string;
  readonly failureReason?: string;
}
export type NativeWorkUnitImplementerOutcomeV1 = ProductWorkUnitImplementerOutcomeV1;
export type NativeWorkUnitHandlerReviewV1 = ProductWorkUnitHandlerReviewV1;
export type NativeWorkUnitHandlerDecisionV1 = ProductWorkUnitHandlerDecisionV1;
export type NativeWorkUnitIncompleteDispositionV1 = ProductWorkUnitIncompleteDispositionV1;
export type NativeWorkUnitRetryAttemptV1 = ProductWorkUnitRetryAttemptV1;
export type NativeWorkUnitIntegrationV1 = ProductWorkUnitIntegrationV1;
export type NativeEpicEscalationReceiverV1 = ProductEpicEscalationReceiverV1;
export interface NativeWorkUnitRelationshipV1 {
  readonly relationshipId: string;
  readonly materializationId: string;
  readonly relationshipKind: 'planning_point' | 'sprint' | 'lane' | 'order' | 'depends_on';
  readonly fromId: string;
  readonly toId: string;
  readonly ordinal?: number;
}
export interface NativeFileReviewDocumentV1 {
  readonly documentRefId: string;
  readonly epicId: string;
  readonly sprintId: string;
  readonly provenanceId: string;
  readonly title: string;
  readonly summary?: string;
  readonly artifactId: string;
  readonly changedFiles: readonly {
    readonly changedFileReferenceId: string;
    readonly displayName: string;
    readonly changeKind: 'added' | 'modified' | 'deleted' | 'renamed';
    readonly previousDisplayName?: string;
  }[];
}
export interface NativeInitiatedEpicV2 {
  readonly initiationId: string;
  readonly epicPlanningDraftId: string;
  readonly proposalRevisionId: string;
  readonly materialSnapshotId: string;
  readonly epicId: string;
  readonly recordedAt: string;
  readonly commandId: string;
  readonly resultId: string;
  readonly eventId: string;
  readonly provenanceId: string;
}
export interface NativeInitiatedSprintV2 {
  readonly sprintId: string;
  readonly epicId: string;
  readonly ordinal: number;
  readonly title: string;
  readonly intendedMovement: string;
  readonly concernSummaries: readonly string[];
  readonly sprintPlanId: string;
  readonly sprintPlanRevisionId: string;
}
export interface NativeInitiationCommandV2 {
  readonly commandId: string;
  readonly epicPlanningDraftId: string;
  readonly expectedRevisionToken: string;
  readonly actorId: string;
  readonly idempotencyKey: string;
  readonly payloadFingerprint: string;
  readonly recordedAt: string;
}
export interface NativeInitiationResultV2 {
  readonly resultId: string;
  readonly commandId: string;
  readonly recordedAt: string;
}
export interface NativeInitiationEventV2 {
  readonly eventId: string;
  readonly commandId: string;
  readonly resultId: string;
  readonly recordedAt: string;
}
export interface NativeInitiationProvenanceV2 {
  readonly provenanceId: string;
  readonly commandId: string;
  readonly resultId: string;
  readonly eventId: string;
  readonly recordedAt: string;
}
export interface NativeMaterialSnapshotV2 {
  readonly materialSnapshotId: string;
  readonly epicPlanningDraftId: string;
  readonly proposalRevisionId: string;
  readonly version: 1;
  readonly proposal: NativeProposalRevisionV1['proposal'];
  readonly contentHash: string;
  readonly recordedAt: string;
}

export interface NativePlanningDraftV1 {
  readonly epicPlanningDraftId: string;
  readonly title?: string;
  readonly status: 'active' | 'canceled' | 'initiated';
  readonly createdAt: string;
  readonly updatedAt: string;
  readonly canceledAt?: string;
  readonly currentProposal:
    | { readonly status: 'empty' }
    | { readonly status: 'available'; readonly proposalRevisionId: string };
}
export interface NativeAgentSessionAssociationV1 {
  readonly agentSessionAssociationId: string;
  readonly epicPlanningDraftId: string;
  readonly agentSessionId: string;
  readonly actorId: string;
  readonly associatedAt: string;
}
export interface NativeProposalRevisionV1 {
  readonly proposalRevisionId: string;
  readonly epicPlanningDraftId: string;
  readonly parentProposalRevisionId?: string;
  readonly revisionToken: string;
  readonly proposal: {
    readonly suggestedEpicName?: string;
    readonly sprints: readonly {
      readonly title: string;
      readonly intendedMovement: string;
      readonly concernSummaries: readonly string[];
    }[];
  };
  readonly commandId: string;
  readonly provenanceId: string;
  readonly recordedAt: string;
}
export interface NativeRecordedProposalEventV1 {
  readonly proposalEventId: string;
  readonly epicPlanningDraftId: string;
  readonly proposalRevisionId: string;
  readonly commandId: string;
  readonly provenanceId: string;
  readonly eventKind: 'proposal_saved';
  readonly recordedAt: string;
}
export interface NativeProvenanceLinkV1 {
  readonly provenanceId: string;
  readonly sourceKind: 'managed_plan_builder';
  readonly recordedAt: string;
  readonly actorId: string;
  readonly agentSessionAssociationId: string;
  readonly capabilityProfileId: string;
  readonly causalCommandId: string;
  readonly causalResultId: string;
}

/** Decodes only the Rust-owned versioned contract; unknown shapes never become product facts. */
export function decodeOrchestrationNativeQueryV2(value: unknown): OrchestrationNativeQueryV2 {
  const root = object(value, 'native query');
  keys(
    root,
    [
      'contractVersion',
      'generatedAt',
      'planningDrafts',
      'agentSessionAssociations',
      'proposalRevisions',
      'recordedProposalEvents',
      'provenanceLinks',
      'initiationCommands',
      'initiationResults',
      'initiationEvents',
      'initiationProvenance',
      'materialSnapshots',
      'initiatedEpics',
      'initiatedSprints',
      'fileReviewDocuments',
      'workUnitMaterializations',
      'workUnits',
      'workUnitRelationships',
      'dependencyActivationIntents',
      'workUnitExecutionStates',
      'workSliceExecutionGraphCompletions',
      'workSliceExecutionSettlements',
      'workSlicePlanningPointExecutionSettlements',
      'workSliceExecutionAttentions',
    ],
    'native query',
  );
  if (root.contractVersion !== ORCHESTRATION_NATIVE_QUERY_V2) fail('unsupported contractVersion');
  const executionFields = ['workUnitExecutionStates', 'workSliceExecutionGraphCompletions', 'workSliceExecutionSettlements', 'workSlicePlanningPointExecutionSettlements', 'workSliceExecutionAttentions'] as const;
  const executionFieldCount = executionFields.filter((field) => root[field] !== undefined).length;
  if (executionFieldCount !== 0 && executionFieldCount !== executionFields.length)
    fail('productive execution projection bundle is incomplete');
  const query: OrchestrationNativeQueryV2 = {
    contractVersion: ORCHESTRATION_NATIVE_QUERY_V2,
    generatedAt: string(root.generatedAt, 'generatedAt'),
    planningDrafts: array(root.planningDrafts, 'planningDrafts').map(draft),
    agentSessionAssociations: array(root.agentSessionAssociations, 'agentSessionAssociations').map(
      association,
    ),
    proposalRevisions: array(root.proposalRevisions, 'proposalRevisions').map(revision),
    recordedProposalEvents: array(root.recordedProposalEvents, 'recordedProposalEvents').map(event),
    provenanceLinks: array(root.provenanceLinks, 'provenanceLinks').map(provenance),
    initiationCommands: array(root.initiationCommands, 'initiationCommands').map(initiationCommand),
    initiationResults: array(root.initiationResults, 'initiationResults').map(initiationResult),
    initiationEvents: array(root.initiationEvents, 'initiationEvents').map(initiationEvent),
    initiationProvenance: array(root.initiationProvenance, 'initiationProvenance').map(
      initiationProvenance,
    ),
    materialSnapshots: array(root.materialSnapshots, 'materialSnapshots').map(materialSnapshot),
    initiatedEpics: array(root.initiatedEpics, 'initiatedEpics').map(initiatedEpic),
    initiatedSprints: array(root.initiatedSprints, 'initiatedSprints').map(initiatedSprint),
    fileReviewDocuments:
      root.fileReviewDocuments === undefined
        ? []
        : array(root.fileReviewDocuments, 'fileReviewDocuments').map(fileReviewDocument),
    workUnitMaterializations:
      root.workUnitMaterializations === undefined
        ? []
        : array(root.workUnitMaterializations, 'workUnitMaterializations').map(
            workUnitMaterialization,
          ),
    workUnits:
      root.workUnits === undefined
        ? []
        : array(root.workUnits, 'workUnits').map(materializedWorkUnit),
    workUnitRelationships:
      root.workUnitRelationships === undefined
        ? []
        : array(root.workUnitRelationships, 'workUnitRelationships').map(workUnitRelationship),
    dependencyActivationIntents:
      root.dependencyActivationIntents === undefined
        ? []
        : array(root.dependencyActivationIntents, 'dependencyActivationIntents').map(
            dependencyActivationIntent,
          ),
    workUnitExecutionStates: root.workUnitExecutionStates === undefined ? [] : array(root.workUnitExecutionStates, 'workUnitExecutionStates').map(workUnitExecutionState),
    workSliceExecutionGraphCompletions: root.workSliceExecutionGraphCompletions === undefined ? [] : array(root.workSliceExecutionGraphCompletions, 'workSliceExecutionGraphCompletions').map(workSliceExecutionGraphCompletion),
    workSliceExecutionSettlements: root.workSliceExecutionSettlements === undefined ? [] : array(root.workSliceExecutionSettlements, 'workSliceExecutionSettlements').map(workSliceExecutionSettlement),
    workSlicePlanningPointExecutionSettlements: root.workSlicePlanningPointExecutionSettlements === undefined ? [] : array(root.workSlicePlanningPointExecutionSettlements, 'workSlicePlanningPointExecutionSettlements').map(workSlicePlanningPointExecutionSettlement),
    workSliceExecutionAttentions: root.workSliceExecutionAttentions === undefined ? [] : array(root.workSliceExecutionAttentions, 'workSliceExecutionAttentions').map(workSliceExecutionAttention),
  };
  validate(query);
  return query;
}

/** A proposal remains a proposal: this projector creates no Epic, Sprint, or acceptance fact. */
export function projectEpicPlanProposal(
  query: OrchestrationNativeQueryV2,
  draftId: string,
): EpicPlanProposalSnapshot {
  const draft = query.planningDrafts.find((item) => item.epicPlanningDraftId === draftId);
  if (!draft)
    return { kind: 'unavailable', reason: 'The requested Epic Planning Draft is not available.' };
  if (draft.currentProposal.status === 'empty')
    return { kind: 'unavailable', reason: 'No Epic Plan Proposal has been saved for this draft.' };
  const currentProposalRevisionId = draft.currentProposal.proposalRevisionId;
  const revision = query.proposalRevisions.find(
    (item) => item.proposalRevisionId === currentProposalRevisionId,
  );
  if (!revision)
    throw new Error('Invalid orchestration native query: current proposal revision is missing');
  return {
    kind: 'available',
    revision: { id: revision.proposalRevisionId, recordedAt: revision.recordedAt },
    ...(revision.proposal.suggestedEpicName
      ? { suggestedEpicName: revision.proposal.suggestedEpicName }
      : {}),
    sprints: revision.proposal.sprints,
  };
}

/** Turns only durable initiation facts into the neutral canonical composition envelope. */
export function nativeQueryProductCompositionInputV2(
  query: OrchestrationNativeQueryV2,
  transitionQuery?: import('./epicBootstrapTransition').EpicBootstrapTransitionQueryV2,
  sprintRunnerTransitionQuery?: import('./sprintRunnerTransition').SprintRunnerTransitionQueryV1,
): ProductReadCompositionInputV1 {
  const initiated = query.initiatedEpics;
  const initiatedPlanBuilders = initiated.map((epic) => {
    const association = query.agentSessionAssociations.find(
      (item) => item.epicPlanningDraftId === epic.epicPlanningDraftId,
    );
    const draft = query.planningDrafts.find(
      (item) => item.epicPlanningDraftId === epic.epicPlanningDraftId,
    );
    if (!association || !draft)
      throw new Error(
        'Invalid orchestration native query: initiated Epic planning binding is missing',
      );
    return { epic, association, draft };
  });
  const uniquePlanBuilderSessions = initiatedPlanBuilders.filter(
    (item, index, items) =>
      items.findIndex(
        ({ association }) => association.agentSessionId === item.association.agentSessionId,
      ) === index,
  );
  const source = (id: string) => ({
    status: 'available' as const,
    sourceKind: 'application_interpretation' as const,
    sourceReferences: [id],
  });
  const materializedUnits = query.workUnitMaterializations
    .filter((materialization) => materialization.settledAt !== undefined)
    .flatMap((materialization) =>
    query.workUnits
      .filter((unit) => unit.materializationId === materialization.materializationId)
      .sort((left, right) => left.laneOrdinal - right.laneOrdinal),
    );
  const scopeId = (unit: NativeMaterializedWorkUnitV1) =>
    `materialized-work-unit-scope:${unit.materializationId}:${unit.workUnitId}`;
  const materializationById = new Map(
    query.workUnitMaterializations.map((materialization) => [
      materialization.materializationId,
      materialization,
    ]),
  );
  const dependencyIntentByWorkUnitId = new Map(
    query.dependencyActivationIntents.map((intent) => [intent.workUnitId, intent]),
  );
  const executionStateByWorkUnitId = new Map(query.workUnitExecutionStates.map((state) => [state.workUnitId, state]));
  const executionByMaterializationId = new Map(query.workUnitMaterializations.map((materialization) => [materialization.materializationId, {
    graphCompletion: query.workSliceExecutionGraphCompletions.find((item) => item.materializationId === materialization.materializationId),
    settlement: query.workSliceExecutionSettlements.find((item) => item.materializationId === materialization.materializationId),
    planningPointSettlement: query.workSlicePlanningPointExecutionSettlements.find((item) => item.materializationId === materialization.materializationId),
    attention: query.workSliceExecutionAttentions.find((item) => item.materializationId === materialization.materializationId),
  }]));
  const events = {
    version: ORCHESTRATION_EVENTS_V1,
    epics: initiated.map((x) => ({ epicId: x.epicId })),
    sprints: query.initiatedSprints.map((x) => ({ sprintId: x.sprintId, epicId: x.epicId })),
    sprintPlans: query.initiatedSprints.map((x) => ({
      sprintPlanId: x.sprintPlanId,
      sprintId: x.sprintId,
    })),
    sprintPlanRevisions: query.initiatedSprints.map((x) => ({
      sprintPlanRevisionId: x.sprintPlanRevisionId,
      sprintPlanId: x.sprintPlanId,
      revision: 1,
    })),
    workUnits: materializedUnits.map((unit) => ({ workUnitId: unit.workUnitId })),
    workUnitScopes: materializedUnits.map((unit) => ({
      workUnitScopeId: scopeId(unit),
      sprintPlanRevisionId: query.initiatedSprints.find(
        (sprint) => sprint.sprintId === materializationById.get(unit.materializationId)?.sprintId,
      )!.sprintPlanRevisionId,
      workUnitId: unit.workUnitId,
      dependsOnWorkUnitScopeIds: query.workUnitRelationships
        .filter(
          (relationship) =>
            relationship.materializationId === unit.materializationId &&
            relationship.relationshipKind === 'depends_on' &&
            relationship.fromId === unit.workUnitId,
        )
        .map((relationship) =>
          scopeId(query.workUnits.find((candidate) => candidate.workUnitId === relationship.toId)!),
        ),
      gateIds: [],
    })),
    workSlicePlanningPoints: query.workUnitMaterializations.filter((materialization) => materialization.settledAt !== undefined).map((materialization) => ({
      workSlicePlanningPointId: materialization.planningPointId,
      sprintPlanId: query.initiatedSprints.find(
        (sprint) => sprint.sprintId === materialization.sprintId,
      )!.sprintPlanId,
      assessedSprintPlanRevisionIds: [
        query.initiatedSprints.find((sprint) => sprint.sprintId === materialization.sprintId)!
          .sprintPlanRevisionId,
      ],
    })),
    workUnitExecutions: [],
    attempts: [],
    agentSessions: uniquePlanBuilderSessions.map(({ association }) => ({
      agentSessionId: association.agentSessionId,
    })),
    // Plan Builder Sessions remain associated with their durable planning drafts. Initiation does
    // not relabel them as one of the five orchestration runtime roles.
    agentSessionReferences: [],
    gates: [],
    gateCriteriaRevisions: [],
    feedbackRecords: [],
    policyEligibilityFacts: [],
    executionRequests: [],
    observedLaunches: [],
    observedReturns: [],
    reviews: [],
    observedIntegrations: [],
    observedCompletions: [],
    continuationRequests: [],
    observedContinuations: [],
    observedHandoffs: [],
    internalArtifacts: query.fileReviewDocuments.map((x) => ({
      artifactId: x.artifactId,
      provenanceId: x.provenanceId,
    })),
    documentReferences: query.fileReviewDocuments.map((x) => ({
      documentRefId: x.documentRefId,
      artifactIds: [x.artifactId],
      provenanceId: x.provenanceId,
    })),
    provenance: [
      ...initiated.map((x) => ({
        provenanceId: x.provenanceId,
        sourceKind: 'application' as const,
        recordedAt: x.recordedAt,
        causalFactIds: [x.epicId],
      })),
      ...query.workUnitMaterializations.map((materialization) => ({
        provenanceId: materialization.materializationId,
        sourceKind: 'application' as const,
        recordedAt: materialization.authorizationRecordedAt,
        causalFactIds: [materialization.sprintId],
      })),
    ],
  };
  return {
    events,
    agentControl: {
      version: AGENT_CONTROL_CONTRACTS_V1,
      promptProvenance: [],
      continuationPolicies: [],
      continuationEligibilityEvaluations: [],
      commands: [],
      results: [],
    },
    artifactAccess: {
      version: ARTIFACT_ACCESS_CONTRACTS_V1,
      artifacts: query.fileReviewDocuments.map((x) => ({
        artifactId: x.artifactId as import('./artifactAccess').ArtifactId,
        kind: 'review_material' as const,
        provenanceReference: x.provenanceId,
      })),
      changedFileReferences: query.fileReviewDocuments.flatMap((x) => x.changedFiles),
      documents: query.fileReviewDocuments.map((x) => ({
        documentRefId: x.documentRefId as import('./artifactAccess').DocumentRefId,
        classification: 'changed_files' as const,
        title: x.title,
        ...(x.summary ? { summary: x.summary } : {}),
        artifactIds: [x.artifactId as import('./artifactAccess').ArtifactId],
        changedFileReferenceIds: x.changedFiles.map((f) => f.changedFileReferenceId),
        provenanceReference: x.provenanceId,
      })),
      requests: [],
      results: [],
    },
    referenceIndex: {
      epics: initiated.map((x) => ({
        epicId: x.epicId,
        title:
          query.proposalRevisions.find((r) => r.proposalRevisionId === x.proposalRevisionId)
            ?.proposal.suggestedEpicName ?? 'Initiated Epic',
        goal: 'Initiated from the durable Epic Plan Proposal.',
        source: source(x.provenanceId),
      })),
      epicOverviews: initiated.map((x) => ({
        epicId: x.epicId,
        currentMovement: {
          source: {
            status: 'unavailable' as const,
            reason: 'No Sprint execution has been initiated.',
          },
        },
        state: {
          source: {
            status: 'unavailable' as const,
            reason: 'No Epic lifecycle state has been observed.',
          },
        },
      })),
      sprints: query.initiatedSprints.map((x) => ({
        sprintId: x.sprintId,
        title: x.title,
        summary: x.intendedMovement,
        details: x.intendedMovement,
        source: source(initiated.find((e) => e.epicId === x.epicId)!.provenanceId),
        lifecycle: {
          source: source(initiated.find((e) => e.epicId === x.epicId)!.provenanceId),
          value: 'not_started' as const,
        },
        planningState: {
          source: source(initiated.find((e) => e.epicId === x.epicId)!.provenanceId),
          value: { kind: 'pre_start_forecast' as const },
        },
      })),
      sprintPlanRevisions: query.initiatedSprints.map((x) => ({
        sprintPlanRevisionId: x.sprintPlanRevisionId,
        summary: 'Preparatory initial Sprint Plan revision.',
        source: source(initiated.find((e) => e.epicId === x.epicId)!.provenanceId),
      })),
      workSlicePlanningPoints: query.workUnitMaterializations.filter((materialization) => materialization.settledAt !== undefined).map((materialization) => ({
        workSlicePlanningPointId: materialization.planningPointId,
        title: 'Accepted Work Slice',
        purpose: `Accepted immutable revision ${materialization.acceptedRevisionId}.`,
        source: source(materialization.materializationId),
      })),
      workUnits: materializedUnits.map((unit) => ({
        workUnitId: unit.workUnitId,
        title: unit.laneTitle,
        summary: unit.specification,
        details: `Accepted Work Slice revision ${unit.acceptedRevisionId}; lane ${unit.laneOrdinal + 1}.${handlerActivationDetail(unit.handlerActivation)}`,
        source: source(unit.materializationId),
        ...(dependencyIntentByWorkUnitId.has(unit.workUnitId)
          ? { dependencyActivationIntent: dependencyIntentByWorkUnitId.get(unit.workUnitId) }
          : {}),
        ...(executionStateByWorkUnitId.has(unit.workUnitId)
          ? { executionState: executionStateByWorkUnitId.get(unit.workUnitId) }
          : {}),
        ...(unit.handlerActivation
          ? { handlerActivation: handlerActivationPresentation(unit.handlerActivation) }
          : {}),
        ...(unit.actionContinuation
          ? { actionContinuation: actionContinuationPresentation(unit.actionContinuation) }
          : {}),
        ...(unit.implementerActivation
          ? { implementerActivation: implementerActivationPresentation(unit.implementerActivation) }
          : {}),
        attemptHistory: unit.attemptHistory.map((attempt) => ({
          ordinal: attempt.ordinal,
          attemptId: attempt.attemptId,
          ...(attempt.implementerOutcome
            ? { implementerOutcome: implementerOutcomePresentation(attempt.implementerOutcome) }
            : {}),
          ...(attempt.handlerReview ? { handlerReview: { ...attempt.handlerReview } } : {}),
          ...(attempt.handlerDecision ? { handlerDecision: { ...attempt.handlerDecision } } : {}),
          ...(attempt.incompleteDisposition
            ? { incompleteDisposition: { ...attempt.incompleteDisposition } }
            : {}),
        })),
        retryAttempts: unit.retryAttempts.map((retry) => ({ ...retry })),
        ...(unit.integration ? { integration: { ...unit.integration } } : {}),
      })),
      gates: [],
      concerns: [],
      agentSessions: uniquePlanBuilderSessions.map(({ epic, association, draft }) => ({
        agentSessionId: association.agentSessionId,
        title: draft.title ?? 'Epic Plan Builder',
        source: source(epic.provenanceId),
      })),
      artifactOwnership: query.fileReviewDocuments.map((x) => ({
        artifactId: x.artifactId,
        sprintId: x.sprintId,
        source: source(x.provenanceId),
      })),
      documentOwnership: query.fileReviewDocuments.map((x) => ({
        documentRefId: x.documentRefId,
        sprintId: x.sprintId,
        source: source(x.provenanceId),
      })),
      sprintWorkspacePresentation: {
        workSlicePlanningPointMembership: query.workUnitMaterializations.filter((materialization) => materialization.settledAt !== undefined).map((materialization) => ({
          workSlicePlanningPointId: materialization.planningPointId,
          sprintPlanRevisionId: query.initiatedSprints.find(
            (sprint) => sprint.sprintId === materialization.sprintId,
          )!.sprintPlanRevisionId,
          workUnitScopeIds: query.workUnits
            .filter((unit) => unit.materializationId === materialization.materializationId)
            .sort((left, right) => left.laneOrdinal - right.laneOrdinal)
            .map(scopeId),
          source: source(materialization.materializationId),
        })),
        gates: [],
        documents: query.fileReviewDocuments.map((x) => ({
          documentRefId: x.documentRefId,
          displayOrder: query.fileReviewDocuments
            .filter((item) => item.sprintId === x.sprintId)
            .indexOf(x),
          recordedAt: { source: source(x.provenanceId), value: query.generatedAt },
          displayCategory: { source: source(x.provenanceId), value: 'File review' },
          sprintPlanRevisionIds: [],
          workSlicePlanningPointIds: [],
          workUnitScopeIds: [],
        })),
      },
    },
    workUnitMaterializations: query.workUnitMaterializations.map((materialization) => ({
      materializationId: materialization.materializationId,
      planningPointId: materialization.planningPointId,
      acceptedRevisionId: materialization.acceptedRevisionId,
      sprintId: materialization.sprintId,
      stage: materializationStage(materialization),
      ...(executionByMaterializationId.get(materialization.materializationId) ? { execution: executionByMaterializationId.get(materialization.materializationId) } : {}),
      source: source(materialization.materializationId),
    })),
    ...(transitionQuery
      ? {
          bootstrapTransition: {
            query: transitionQuery,
            initiationIdsByEpic: Object.fromEntries(
              query.initiatedEpics.map((epic) => [epic.epicId, epic.initiationId]),
            ),
          },
        }
      : {}),
    ...(sprintRunnerTransitionQuery
      ? { sprintRunnerTransition: { query: sprintRunnerTransitionQuery } }
      : {}),
  };
}

function materializationStage(materialization: NativeWorkUnitMaterializationV1) {
  if (materialization.settledAt) return 'settled' as const;
  if (materialization.relationshipsCompletedAt) return 'relationships_complete' as const;
  if (materialization.workUnitsCreatedAt) return 'work_units_created' as const;
  if (materialization.attemptRecordedAt) return 'attempt_recorded' as const;
  return 'authorized' as const;
}

function handlerActivationDetail(activation: NativeWorkUnitHandlerActivationV1 | undefined) {
  if (!activation) return '';
  if (activation.eligibilityState === 'blocked')
    return ` Handler activation blocked: ${activation.blockedReason}.`;
  const providerObservation = activation.providerActivationObservedAt
    ? ' Provider activity observed separately; no provider lifecycle, outcome, or acceptance is implied.'
    : ' Provider activity is unobserved.';
  if (activation.failureReason)
    return ` Handler activation recorded a durable failure before application readiness.${providerObservation}`;
  if (activation.handlerReadyAt)
    return ` Handler launch accepted and application Handler readiness recorded.${providerObservation}`;
  if (activation.launchAcceptedAt)
    return ` Handler launch accepted; application Handler readiness is not yet recorded.${providerObservation}`;
  if (activation.launchRequestedAt)
    return ` Handler launch requested; acceptance is not yet recorded.${providerObservation}`;
  if (activation.handlerInvocationPreparedAt)
    return ` Handler invocation prepared; launch is not yet recorded.${providerObservation}`;
  return ` Handler activation is eligible but not yet prepared.${providerObservation}`;
}

function handlerActivationPresentation(
  activation: NativeWorkUnitHandlerActivationV1,
): ProductWorkUnitHandlerActivationV1 {
  if (activation.eligibilityState === 'blocked') {
    return { eligibilityState: 'blocked', blockedReason: activation.blockedReason! };
  }
  return {
    eligibilityState: 'eligible',
    stage: activation.handlerReadyAt
      ? 'handler_ready'
      : activation.failureReason
        ? 'failed'
        : activation.launchAcceptedAt
          ? 'launch_accepted'
          : activation.launchRequestedAt
            ? 'launch_requested'
            : activation.handlerInvocationPreparedAt
              ? 'invocation_prepared'
              : 'eligible_not_prepared',
    ...(activation.failureReason ? { failureReason: activation.failureReason } : {}),
    providerActivityObserved: Boolean(activation.providerActivationObservedAt),
  };
}

function actionContinuationPresentation(
  continuation: NativeWorkUnitHandlerActionContinuationV1,
): ProductWorkUnitActionContinuationV1 {
  return {
    stage: continuation.blockedReason
      ? 'blocked'
      : continuation.failureReason
        ? 'failed'
        : continuation.actionReadyAt
          ? 'action_ready'
          : continuation.launchAcceptedAt
            ? 'launch_accepted'
            : continuation.launchRequestedAt
              ? 'launch_requested'
              : continuation.harnessBoundAt
                ? 'harness_bound'
                : continuation.invocationPreparedAt
                  ? 'invocation_prepared'
                  : continuation.authorizedAt
                    ? 'authorized'
                    : 'requested',
    ...(continuation.blockedReason ? { blockedReason: continuation.blockedReason } : {}),
    ...(continuation.failureReason ? { failureReason: continuation.failureReason } : {}),
    providerActivityObserved: Boolean(continuation.providerActivationObservedAt),
  };
}

function implementerActivationPresentation(
  activation: NativeWorkUnitImplementerActivationV1,
): ProductWorkUnitImplementerActivationV1 {
  return {
    stage: activation.failureReason
      ? 'failed'
      : activation.implementerReadyAt
        ? 'implementer_ready'
        : activation.launchAcceptedAt
          ? 'launch_accepted'
          : activation.launchRequestedAt
            ? 'launch_requested'
            : activation.implementerHarnessBoundAt
              ? 'harness_bound'
              : activation.implementerInvocationPreparedAt
                ? 'invocation_prepared'
                : activation.implementerSessionCreatedAt
                  ? 'session_created'
                  : activation.isolatedWorktreeReadyAt
                    ? 'worktree_ready'
                    : activation.executionSupportGrantedAt
                      ? 'execution_support_granted'
                      : activation.authorizedAt
                        ? 'authorized'
                        : 'requested',
    ...(activation.failureReason ? { failureReason: activation.failureReason } : {}),
    providerActivityObserved: Boolean(activation.providerActivationObservedAt),
  };
}

function implementerOutcomePresentation(
  outcome: NativeWorkUnitImplementerOutcomeV1,
): ProductWorkUnitImplementerOutcomeV1 {
  return {
    ...outcome,
    ...(outcome.submittedOutcome ? { submittedOutcome: { ...outcome.submittedOutcome } } : {}),
    ...(outcome.evidence
      ? {
          evidence: {
            ...outcome.evidence,
            changedFiles: outcome.evidence.changedFiles.map((file) => ({ ...file })),
          },
        }
      : {}),
    ...(outcome.semanticCompletion
      ? { semanticCompletion: { ...outcome.semanticCompletion } }
      : {}),
    ...(outcome.terminalLifecycle ? { terminalLifecycle: { ...outcome.terminalLifecycle } } : {}),
  };
}

const draft = (value: unknown): NativePlanningDraftV1 => {
  const x = object(value, 'planning draft');
  keys(
    x,
    [
      'epicPlanningDraftId',
      'title',
      'status',
      'createdAt',
      'updatedAt',
      'canceledAt',
      'currentProposal',
    ],
    'planning draft',
  );
  if (x.status !== 'active' && x.status !== 'canceled' && x.status !== 'initiated')
    fail('invalid planning draft status');
  if (x.status === 'active' && x.canceledAt !== undefined)
    fail('active planning draft cannot have canceledAt');
  if (x.status === 'canceled' && x.canceledAt === undefined)
    fail('canceled planning draft requires canceledAt');
  if (x.status === 'initiated' && x.canceledAt !== undefined)
    fail('initiated planning draft cannot have canceledAt');
  const common = {
    epicPlanningDraftId: string(x.epicPlanningDraftId, 'epicPlanningDraftId'),
    ...(x.title === undefined ? {} : { title: boundedString(x.title, 240, 'draft title') }),
    status: x.status,
    createdAt: string(x.createdAt, 'createdAt'),
    updatedAt: string(x.updatedAt, 'updatedAt'),
    ...(x.canceledAt === undefined ? {} : { canceledAt: string(x.canceledAt, 'canceledAt') }),
  } as const;
  const current = object(x.currentProposal, 'currentProposal');
  if (current.status === 'empty') {
    keys(current, ['status'], 'currentProposal');
    return {
      ...common,
      currentProposal: { status: 'empty' },
    };
  }
  keys(current, ['status', 'proposalRevisionId'], 'currentProposal');
  if (current.status !== 'available') fail('invalid currentProposal status');
  return {
    ...common,
    currentProposal: {
      status: 'available',
      proposalRevisionId: string(current.proposalRevisionId, 'proposalRevisionId'),
    },
  };
};
const association = (value: unknown): NativeAgentSessionAssociationV1 => {
  const x = object(value, 'agentSessionAssociation');
  keys(
    x,
    [
      'agentSessionAssociationId',
      'epicPlanningDraftId',
      'agentSessionId',
      'actorId',
      'associatedAt',
    ],
    'agentSessionAssociation',
  );
  return {
    agentSessionAssociationId: string(x.agentSessionAssociationId, 'agentSessionAssociationId'),
    epicPlanningDraftId: string(x.epicPlanningDraftId, 'epicPlanningDraftId'),
    agentSessionId: string(x.agentSessionId, 'agentSessionId'),
    actorId: string(x.actorId, 'actorId'),
    associatedAt: string(x.associatedAt, 'associatedAt'),
  };
};
const revision = (value: unknown): NativeProposalRevisionV1 => {
  const x = object(value, 'proposalRevision');
  keys(
    x,
    [
      'proposalRevisionId',
      'epicPlanningDraftId',
      'parentProposalRevisionId',
      'revisionToken',
      'proposal',
      'commandId',
      'provenanceId',
      'recordedAt',
    ],
    'proposalRevision',
  );
  const proposal = object(x.proposal, 'proposal');
  keys(proposal, ['suggestedEpicName', 'sprints'], 'proposal');
  return {
    proposalRevisionId: string(x.proposalRevisionId, 'proposalRevisionId'),
    epicPlanningDraftId: string(x.epicPlanningDraftId, 'epicPlanningDraftId'),
    ...(x.parentProposalRevisionId === undefined
      ? {}
      : {
          parentProposalRevisionId: string(x.parentProposalRevisionId, 'parentProposalRevisionId'),
        }),
    revisionToken: string(x.revisionToken, 'revisionToken'),
    proposal: {
      ...(proposal.suggestedEpicName === undefined
        ? {}
        : { suggestedEpicName: string(proposal.suggestedEpicName, 'suggestedEpicName') }),
      sprints: array(proposal.sprints, 'sprints').map((item) => {
        const sprint = object(item, 'proposal sprint');
        keys(sprint, ['title', 'intendedMovement', 'concernSummaries'], 'proposal sprint');
        return {
          title: string(sprint.title, 'title'),
          intendedMovement: string(sprint.intendedMovement, 'intendedMovement'),
          concernSummaries: array(sprint.concernSummaries, 'concernSummaries').map((c) =>
            string(c, 'concernSummary'),
          ),
        };
      }),
    },
    commandId: string(x.commandId, 'commandId'),
    provenanceId: string(x.provenanceId, 'provenanceId'),
    recordedAt: string(x.recordedAt, 'recordedAt'),
  };
};
const event = (value: unknown): NativeRecordedProposalEventV1 => {
  const x = object(value, 'recordedProposalEvent');
  keys(
    x,
    [
      'proposalEventId',
      'epicPlanningDraftId',
      'proposalRevisionId',
      'commandId',
      'provenanceId',
      'eventKind',
      'recordedAt',
    ],
    'recordedProposalEvent',
  );
  if (x.eventKind !== 'proposal_saved') fail('invalid proposal event kind');
  return {
    proposalEventId: string(x.proposalEventId, 'proposalEventId'),
    epicPlanningDraftId: string(x.epicPlanningDraftId, 'epicPlanningDraftId'),
    proposalRevisionId: string(x.proposalRevisionId, 'proposalRevisionId'),
    commandId: string(x.commandId, 'commandId'),
    provenanceId: string(x.provenanceId, 'provenanceId'),
    eventKind: 'proposal_saved',
    recordedAt: string(x.recordedAt, 'recordedAt'),
  };
};
const provenance = (value: unknown): NativeProvenanceLinkV1 => {
  const x = object(value, 'provenanceLink');
  keys(
    x,
    [
      'provenanceId',
      'sourceKind',
      'recordedAt',
      'actorId',
      'agentSessionAssociationId',
      'capabilityProfileId',
      'causalCommandId',
      'causalResultId',
    ],
    'provenanceLink',
  );
  if (x.sourceKind !== 'managed_plan_builder') fail('invalid provenance source kind');
  return {
    provenanceId: string(x.provenanceId, 'provenanceId'),
    sourceKind: 'managed_plan_builder',
    recordedAt: string(x.recordedAt, 'recordedAt'),
    actorId: string(x.actorId, 'actorId'),
    agentSessionAssociationId: string(x.agentSessionAssociationId, 'agentSessionAssociationId'),
    capabilityProfileId: string(x.capabilityProfileId, 'capabilityProfileId'),
    causalCommandId: string(x.causalCommandId, 'causalCommandId'),
    causalResultId: string(x.causalResultId, 'causalResultId'),
  };
};
const initiationCommand = (value: unknown): NativeInitiationCommandV2 => {
  const x = object(value, 'initiation command');
  keys(
    x,
    [
      'commandId',
      'epicPlanningDraftId',
      'expectedRevisionToken',
      'actorId',
      'idempotencyKey',
      'payloadFingerprint',
      'recordedAt',
    ],
    'initiation command',
  );
  return {
    commandId: string(x.commandId, 'commandId'),
    epicPlanningDraftId: string(x.epicPlanningDraftId, 'epicPlanningDraftId'),
    expectedRevisionToken: string(x.expectedRevisionToken, 'expectedRevisionToken'),
    actorId: string(x.actorId, 'actorId'),
    idempotencyKey: string(x.idempotencyKey, 'idempotencyKey'),
    payloadFingerprint: string(x.payloadFingerprint, 'payloadFingerprint'),
    recordedAt: string(x.recordedAt, 'recordedAt'),
  };
};
const initiationResult = (value: unknown): NativeInitiationResultV2 => {
  const x = object(value, 'initiation result');
  keys(x, ['resultId', 'commandId', 'recordedAt'], 'initiation result');
  return {
    resultId: string(x.resultId, 'resultId'),
    commandId: string(x.commandId, 'commandId'),
    recordedAt: string(x.recordedAt, 'recordedAt'),
  };
};
const initiationEvent = (value: unknown): NativeInitiationEventV2 => {
  const x = object(value, 'initiation event');
  keys(x, ['eventId', 'commandId', 'resultId', 'recordedAt'], 'initiation event');
  return {
    eventId: string(x.eventId, 'eventId'),
    commandId: string(x.commandId, 'commandId'),
    resultId: string(x.resultId, 'resultId'),
    recordedAt: string(x.recordedAt, 'recordedAt'),
  };
};
const initiationProvenance = (value: unknown): NativeInitiationProvenanceV2 => {
  const x = object(value, 'initiation provenance');
  keys(
    x,
    ['provenanceId', 'commandId', 'resultId', 'eventId', 'recordedAt'],
    'initiation provenance',
  );
  return {
    provenanceId: string(x.provenanceId, 'provenanceId'),
    commandId: string(x.commandId, 'commandId'),
    resultId: string(x.resultId, 'resultId'),
    eventId: string(x.eventId, 'eventId'),
    recordedAt: string(x.recordedAt, 'recordedAt'),
  };
};
const materialSnapshot = (value: unknown): NativeMaterialSnapshotV2 => {
  const x = object(value, 'material snapshot');
  keys(
    x,
    [
      'materialSnapshotId',
      'epicPlanningDraftId',
      'proposalRevisionId',
      'version',
      'proposal',
      'contentHash',
      'recordedAt',
    ],
    'material snapshot',
  );
  if (x.version !== 1) fail('unsupported material snapshot version');
  const proposal = object(x.proposal, 'material snapshot proposal');
  keys(proposal, ['suggestedEpicName', 'sprints'], 'material snapshot proposal');
  const contentHash = string(x.contentHash, 'contentHash');
  if (!/^[a-f0-9]{64}$/.test(contentHash))
    fail('material snapshot contentHash must be SHA-256 hex');
  return {
    materialSnapshotId: string(x.materialSnapshotId, 'materialSnapshotId'),
    epicPlanningDraftId: string(x.epicPlanningDraftId, 'epicPlanningDraftId'),
    proposalRevisionId: string(x.proposalRevisionId, 'proposalRevisionId'),
    version: 1,
    proposal: {
      ...(proposal.suggestedEpicName === undefined
        ? {}
        : { suggestedEpicName: string(proposal.suggestedEpicName, 'suggestedEpicName') }),
      sprints: array(proposal.sprints, 'sprints').map((item) => {
        const sprint = object(item, 'material snapshot Sprint');
        keys(sprint, ['title', 'intendedMovement', 'concernSummaries'], 'material snapshot Sprint');
        return {
          title: string(sprint.title, 'title'),
          intendedMovement: string(sprint.intendedMovement, 'intendedMovement'),
          concernSummaries: array(sprint.concernSummaries, 'concernSummaries').map((summary) =>
            string(summary, 'concernSummary'),
          ),
        };
      }),
    },
    contentHash,
    recordedAt: string(x.recordedAt, 'recordedAt'),
  };
};
const initiatedEpic = (value: unknown): NativeInitiatedEpicV2 => {
  const x = object(value, 'initiated Epic');
  keys(
    x,
    [
      'initiationId',
      'epicPlanningDraftId',
      'proposalRevisionId',
      'materialSnapshotId',
      'epicId',
      'recordedAt',
      'commandId',
      'resultId',
      'eventId',
      'provenanceId',
    ],
    'initiated Epic',
  );
  return {
    initiationId: string(x.initiationId, 'initiationId'),
    epicPlanningDraftId: string(x.epicPlanningDraftId, 'epicPlanningDraftId'),
    proposalRevisionId: string(x.proposalRevisionId, 'proposalRevisionId'),
    materialSnapshotId: string(x.materialSnapshotId, 'materialSnapshotId'),
    epicId: string(x.epicId, 'epicId'),
    recordedAt: string(x.recordedAt, 'recordedAt'),
    commandId: string(x.commandId, 'commandId'),
    resultId: string(x.resultId, 'resultId'),
    eventId: string(x.eventId, 'eventId'),
    provenanceId: string(x.provenanceId, 'provenanceId'),
  };
};
const initiatedSprint = (value: unknown): NativeInitiatedSprintV2 => {
  const x = object(value, 'initiated Sprint');
  keys(
    x,
    [
      'sprintId',
      'epicId',
      'ordinal',
      'title',
      'intendedMovement',
      'concernSummaries',
      'sprintPlanId',
      'sprintPlanRevisionId',
    ],
    'initiated Sprint',
  );
  if (!Number.isSafeInteger(x.ordinal) || (x.ordinal as number) < 0) fail('invalid Sprint ordinal');
  return {
    sprintId: string(x.sprintId, 'sprintId'),
    epicId: string(x.epicId, 'epicId'),
    ordinal: x.ordinal as number,
    title: boundedString(x.title, 240, 'Sprint title'),
    intendedMovement: boundedString(x.intendedMovement, 4000, 'Sprint intended movement'),
    concernSummaries: array(x.concernSummaries, 'concernSummaries').map((x) =>
      boundedString(x, 2000, 'concern summary'),
    ),
    sprintPlanId: string(x.sprintPlanId, 'sprintPlanId'),
    sprintPlanRevisionId: string(x.sprintPlanRevisionId, 'sprintPlanRevisionId'),
  };
};
const workUnitMaterialization = (value: unknown): NativeWorkUnitMaterializationV1 => {
  const x = object(value, 'Work Unit materialization');
  keys(
    x,
    [
      'materializationId',
      'planningPointId',
      'acceptedRevisionId',
      'epicId',
      'sprintId',
      'workSliceId',
      'authorizationRecordedAt',
      'attemptRecordedAt',
      'workUnitsCreatedAt',
      'relationshipsCompletedAt',
      'settledAt',
    ],
    'Work Unit materialization',
  );
  const optionalTime = (key: keyof typeof x) =>
    x[key] === undefined ? undefined : string(x[key], key);
  return {
    materializationId: string(x.materializationId, 'materializationId'),
    planningPointId: string(x.planningPointId, 'planningPointId'),
    acceptedRevisionId: string(x.acceptedRevisionId, 'acceptedRevisionId'),
    epicId: string(x.epicId, 'epicId'),
    sprintId: string(x.sprintId, 'sprintId'),
    workSliceId: string(x.workSliceId, 'workSliceId'),
    authorizationRecordedAt: string(x.authorizationRecordedAt, 'authorizationRecordedAt'),
    ...(optionalTime('attemptRecordedAt')
      ? { attemptRecordedAt: optionalTime('attemptRecordedAt') }
      : {}),
    ...(optionalTime('workUnitsCreatedAt')
      ? { workUnitsCreatedAt: optionalTime('workUnitsCreatedAt') }
      : {}),
    ...(optionalTime('relationshipsCompletedAt')
      ? { relationshipsCompletedAt: optionalTime('relationshipsCompletedAt') }
      : {}),
    ...(optionalTime('settledAt') ? { settledAt: optionalTime('settledAt') } : {}),
  };
};
const workUnitExecutionState = (value: unknown): NativeWorkUnitExecutionStateV1 => {
  const x = object(value, 'Work Unit execution state');
  keys(x, ['workUnitId', 'materializationId', 'acceptedRevisionId', 'state', 'recordedAt'], 'Work Unit execution state');
  if (!['waiting_on_prerequisites', 'ready', 'active', 'retry_authorized', 'handed_back', 'settled', 'attention'].includes(x.state as string)) fail('invalid Work Unit execution state');
  return { workUnitId: string(x.workUnitId, 'workUnitId'), materializationId: string(x.materializationId, 'materializationId'), acceptedRevisionId: string(x.acceptedRevisionId, 'acceptedRevisionId'), state: x.state as NativeWorkUnitExecutionStateV1['state'], recordedAt: timestamp(x.recordedAt, 'Work Unit execution state recordedAt') };
};
const workSliceExecutionGraphCompletion = (value: unknown): NativeWorkSliceExecutionGraphCompletionV1 => {
  const x = object(value, 'Work Slice graph completion');
  keys(x, ['materializationId', 'acceptedRevisionId', 'completedAt'], 'Work Slice graph completion');
  return { materializationId: string(x.materializationId, 'materializationId'), acceptedRevisionId: string(x.acceptedRevisionId, 'acceptedRevisionId'), completedAt: timestamp(x.completedAt, 'Work Slice graph completion completedAt') };
};
const workSliceExecutionSettlement = (value: unknown): NativeWorkSliceExecutionSettlementV1 => {
  const x = object(value, 'Work Slice execution settlement');
  keys(x, ['materializationId', 'graphCompletionMaterializationId', 'settledAt'], 'Work Slice execution settlement');
  return { materializationId: string(x.materializationId, 'materializationId'), graphCompletionMaterializationId: string(x.graphCompletionMaterializationId, 'graphCompletionMaterializationId'), settledAt: timestamp(x.settledAt, 'Work Slice execution settlement settledAt') };
};
const workSlicePlanningPointExecutionSettlement = (value: unknown): NativeWorkSlicePlanningPointExecutionSettlementV1 => {
  const x = object(value, 'Work Slice planning-point execution settlement');
  keys(x, ['planningPointId', 'materializationId', 'workSliceExecutionMaterializationId', 'settledAt'], 'Work Slice planning-point execution settlement');
  return { planningPointId: string(x.planningPointId, 'planningPointId'), materializationId: string(x.materializationId, 'materializationId'), workSliceExecutionMaterializationId: string(x.workSliceExecutionMaterializationId, 'workSliceExecutionMaterializationId'), settledAt: timestamp(x.settledAt, 'Work Slice planning-point execution settlement settledAt') };
};
const workSliceExecutionAttention = (value: unknown): NativeWorkSliceExecutionAttentionV1 => {
  const x = object(value, 'Work Slice execution attention');
  keys(x, ['materializationId', 'recordedAt'], 'Work Slice execution attention');
  return { materializationId: string(x.materializationId, 'materializationId'), recordedAt: timestamp(x.recordedAt, 'Work Slice execution attention recordedAt') };
};
const materializedWorkUnit = (value: unknown): NativeMaterializedWorkUnitV1 => {
  const x = object(value, 'materialized Work Unit');
  keys(
    x,
    [
      'workUnitId',
      'materializationId',
      'workSliceId',
      'acceptedRevisionId',
      'laneOrdinal',
      'laneTitle',
      'specification',
      'handlerActivation',
      'actionContinuation',
      'implementerActivation',
      'attemptHistory',
      'retryAttempts',
      'integration',
    ],
    'materialized Work Unit',
  );
  if (!Number.isSafeInteger(x.laneOrdinal) || (x.laneOrdinal as number) < 0)
    fail('invalid Work Unit lane ordinal');
  return {
    workUnitId: string(x.workUnitId, 'workUnitId'),
    materializationId: string(x.materializationId, 'materializationId'),
    workSliceId: string(x.workSliceId, 'workSliceId'),
    acceptedRevisionId: string(x.acceptedRevisionId, 'acceptedRevisionId'),
    laneOrdinal: x.laneOrdinal as number,
    laneTitle: boundedString(x.laneTitle, 240, 'laneTitle'),
    specification: boundedString(x.specification, 4000, 'specification'),
    ...(x.handlerActivation === undefined
      ? {}
      : { handlerActivation: workUnitHandlerActivation(x.handlerActivation) }),
    ...(x.actionContinuation === undefined
      ? {}
      : { actionContinuation: workUnitActionContinuation(x.actionContinuation) }),
    ...(x.implementerActivation === undefined
      ? {}
      : { implementerActivation: workUnitImplementerActivation(x.implementerActivation) }),
    attemptHistory: array(x.attemptHistory, 'Work Unit attemptHistory').map(workUnitAttemptHistory),
    retryAttempts: array(x.retryAttempts, 'Work Unit retryAttempts').map(workUnitRetryAttempt),
    ...(x.integration === undefined ? {} : { integration: workUnitIntegration(x.integration) }),
  };
};
const workUnitAttemptHistory = (value: unknown): NativeWorkUnitAttemptHistoryV1 => {
  const x = object(value, 'Work Unit attempt history member');
  keys(
    x,
    ['ordinal', 'attemptId', 'implementerOutcome', 'handlerReview', 'handlerDecision', 'incompleteDisposition'],
    'Work Unit attempt history member',
  );
  if (!Number.isSafeInteger(x.ordinal) || (x.ordinal as number) < 0)
    fail('invalid Work Unit attempt history ordinal');
  return {
    ordinal: x.ordinal as number,
    attemptId: boundedString(x.attemptId, 240, 'attempt history attemptId'),
    ...(x.implementerOutcome === undefined
      ? {}
      : { implementerOutcome: workUnitImplementerOutcome(x.implementerOutcome) }),
    ...(x.handlerReview === undefined ? {} : { handlerReview: workUnitHandlerReview(x.handlerReview) }),
    ...(x.handlerDecision === undefined
      ? {}
      : { handlerDecision: workUnitHandlerDecision(x.handlerDecision) }),
    ...(x.incompleteDisposition === undefined
      ? {}
      : { incompleteDisposition: workUnitIncompleteDisposition(x.incompleteDisposition) }),
  };
};
const workUnitRetryAttempt = (value: unknown): NativeWorkUnitRetryAttemptV1 => {
  const x = object(value, 'Work Unit retry attempt');
  keys(
    x,
    [
      'ordinal',
      'originAttemptId',
      'retryAttemptId',
      'implementerSessionId',
      'implementerInvocationId',
      'captureRequestedAt',
      'candidatePinnedAt',
      'authorizedAt',
      'executionSupportGrantedAt',
      'isolatedWorktreeReadyAt',
      'implementerSessionCreatedAt',
      'implementerInvocationPreparedAt',
      'implementerHarnessBoundAt',
      'launchRequestedAt',
      'launchAcceptedAt',
      'providerActivationObservedAt',
      'retryReadyAt',
      'failureReason',
    ],
    'Work Unit retry attempt',
  );
  if (!Number.isSafeInteger(x.ordinal) || (x.ordinal as number) < 0)
    fail('Work Unit retry attempt ordinal must be a nonnegative integer');
  const optionalTime = (key: keyof typeof x) =>
    x[key] === undefined ? undefined : timestamp(x[key], key);
  const failureReason =
    x.failureReason === undefined
      ? undefined
      : boundedString(x.failureReason, 4000, 'retry failureReason');
  return {
    ordinal: x.ordinal as number,
    originAttemptId: boundedString(x.originAttemptId, 240, 'retry originAttemptId'),
    retryAttemptId: boundedString(x.retryAttemptId, 240, 'retry retryAttemptId'),
    implementerSessionId: boundedString(
      x.implementerSessionId,
      240,
      'retry implementerSessionId',
    ),
    implementerInvocationId: boundedString(
      x.implementerInvocationId,
      240,
      'retry implementerInvocationId',
    ),
    captureRequestedAt: timestamp(x.captureRequestedAt, 'retry captureRequestedAt'),
    ...(optionalTime('candidatePinnedAt')
      ? { candidatePinnedAt: optionalTime('candidatePinnedAt') }
      : {}),
    ...(optionalTime('authorizedAt') ? { authorizedAt: optionalTime('authorizedAt') } : {}),
    ...(optionalTime('executionSupportGrantedAt')
      ? { executionSupportGrantedAt: optionalTime('executionSupportGrantedAt') }
      : {}),
    ...(optionalTime('isolatedWorktreeReadyAt')
      ? { isolatedWorktreeReadyAt: optionalTime('isolatedWorktreeReadyAt') }
      : {}),
    ...(optionalTime('implementerSessionCreatedAt')
      ? { implementerSessionCreatedAt: optionalTime('implementerSessionCreatedAt') }
      : {}),
    ...(optionalTime('implementerInvocationPreparedAt')
      ? { implementerInvocationPreparedAt: optionalTime('implementerInvocationPreparedAt') }
      : {}),
    ...(optionalTime('implementerHarnessBoundAt')
      ? { implementerHarnessBoundAt: optionalTime('implementerHarnessBoundAt') }
      : {}),
    ...(optionalTime('launchRequestedAt')
      ? { launchRequestedAt: optionalTime('launchRequestedAt') }
      : {}),
    ...(optionalTime('launchAcceptedAt')
      ? { launchAcceptedAt: optionalTime('launchAcceptedAt') }
      : {}),
    ...(optionalTime('providerActivationObservedAt')
      ? { providerActivationObservedAt: optionalTime('providerActivationObservedAt') }
      : {}),
    ...(optionalTime('retryReadyAt') ? { retryReadyAt: optionalTime('retryReadyAt') } : {}),
    ...(failureReason ? { failureReason } : {}),
  };
};
const workUnitIntegration = (value: unknown): NativeWorkUnitIntegrationV1 => {
  const x = object(value, 'Work Unit integration');
  keys(x, ['requestedAt', 'authorizedAt', 'progress', 'attention', 'success', 'settlement', 'prerequisiteContribution'], 'Work Unit integration');
  const requestedAt = timestamp(x.requestedAt, 'Work Unit integration requestedAt');
  const authorizedAt = timestamp(x.authorizedAt, 'Work Unit integration authorizedAt');
  timestampAtOrAfter(requestedAt, authorizedAt, 'Work Unit integration authorization');
  const progress = x.progress === undefined ? undefined : (() => { const entry = object(x.progress, 'Work Unit integration progress'); keys(entry, ['phase', 'recordedAt'], 'Work Unit integration progress'); if (!['preparing', 'applying', 'recording'].includes(entry.phase as string)) fail('invalid Work Unit integration progress phase'); const recordedAt = timestamp(entry.recordedAt, 'Work Unit integration progress recordedAt'); timestampAtOrAfter(authorizedAt, recordedAt, 'Work Unit integration progress'); return { phase: entry.phase as 'preparing' | 'applying' | 'recording', recordedAt }; })();
  const attention = x.attention === undefined ? undefined : (() => { const entry = object(x.attention, 'Work Unit integration attention'); keys(entry, ['kind', 'safeCode', 'recordedAt'], 'Work Unit integration attention'); if (!['conflict', 'failure'].includes(entry.kind as string)) fail('invalid Work Unit integration attention kind'); const safeCode = entry.kind === 'conflict' ? 'integration_conflict' : 'integration_failure'; if (entry.safeCode !== safeCode) fail('invalid Work Unit integration attention code'); const recordedAt = timestamp(entry.recordedAt, 'Work Unit integration attention recordedAt'); timestampAtOrAfter(progress?.recordedAt ?? authorizedAt, recordedAt, 'Work Unit integration attention'); return { kind: entry.kind as 'conflict' | 'failure', safeCode: safeCode as 'integration_conflict' | 'integration_failure', recordedAt }; })();
  const success = x.success === undefined ? undefined : (() => { const entry = object(x.success, 'Work Unit integration success'); keys(entry, ['recordedAt'], 'Work Unit integration success'); const recordedAt = timestamp(entry.recordedAt, 'Work Unit integration success recordedAt'); timestampAtOrAfter(progress?.recordedAt ?? authorizedAt, recordedAt, 'Work Unit integration success'); return { recordedAt }; })();
  const settlement = x.settlement === undefined ? undefined : (() => { const entry = object(x.settlement, 'Work Unit settlement'); keys(entry, ['settledAt'], 'Work Unit settlement'); const settledAt = timestamp(entry.settledAt, 'Work Unit settlement settledAt'); if (!success) fail('Work Unit settlement requires integration success'); timestampAtOrAfter(success.recordedAt, settledAt, 'Work Unit settlement'); return { settledAt }; })();
  const prerequisiteContribution = x.prerequisiteContribution === undefined ? undefined : (() => { const entry = object(x.prerequisiteContribution, 'Work Unit prerequisite contribution'); keys(entry, ['recordedAt', 'dependentCount'], 'Work Unit prerequisite contribution'); if (!Number.isSafeInteger(entry.dependentCount) || (entry.dependentCount as number) < 0) fail('invalid Work Unit prerequisite contribution count'); if (!settlement) fail('Work Unit prerequisite contribution requires settlement'); const recordedAt = timestamp(entry.recordedAt, 'Work Unit prerequisite contribution recordedAt'); timestampAtOrAfter(settlement.settledAt, recordedAt, 'Work Unit prerequisite contribution'); return { recordedAt, dependentCount: entry.dependentCount as number }; })();
  if (attention && (success || settlement || prerequisiteContribution)) fail('Work Unit integration attention contradicts terminal facts');
  return { requestedAt, authorizedAt, ...(progress ? { progress } : {}), ...(attention ? { attention } : {}), ...(success ? { success } : {}), ...(settlement ? { settlement } : {}), ...(prerequisiteContribution ? { prerequisiteContribution } : {}) };
};
const dependencyActivationIntent = (value: unknown): NativeWorkUnitDependencyActivationIntentV1 => {
  const x = object(value, 'dependency activation intent');
  keys(x, ['workUnitId', 'materializationId', 'acceptedRevisionId', 'eligibilityState', 'blockedReason', 'eligibilityRecordedAt', 'activationIntendedAt'], 'dependency activation intent');
  const eligibilityState = x.eligibilityState === 'blocked' || x.eligibilityState === 'eligible'
    ? x.eligibilityState
    : fail('invalid dependency eligibility state');
  const blockedReason = x.blockedReason === undefined ? undefined : string(x.blockedReason, 'blockedReason');
  const activationIntendedAt = x.activationIntendedAt === undefined ? undefined : timestamp(x.activationIntendedAt, 'activationIntendedAt');
  if (eligibilityState === 'blocked' && !blockedReason)
    fail('blocked dependency intent requires a reason');
  if (eligibilityState === 'eligible' && blockedReason)
    fail('eligible dependency intent cannot have a blocked reason');
  return {
    workUnitId: string(x.workUnitId, 'workUnitId'),
    materializationId: string(x.materializationId, 'materializationId'),
    acceptedRevisionId: string(x.acceptedRevisionId, 'acceptedRevisionId'),
    eligibilityState,
    ...(blockedReason ? { blockedReason } : {}),
    eligibilityRecordedAt: timestamp(x.eligibilityRecordedAt, 'eligibilityRecordedAt'),
    ...(activationIntendedAt ? { activationIntendedAt } : {}),
  };
};
const workUnitHandlerActivation = (value: unknown): NativeWorkUnitHandlerActivationV1 => {
  const x = object(value, 'Work Unit Handler activation');
  keys(
    x,
    [
      'attemptId',
      'handlerSessionId',
        'handlerInvocationId',
        'handlerHarnessRevisionId',
      'eligibilityState',
      'blockedReason',
      'requestedAt',
      'authorizedAt',
      'attemptCreatedAt',
      'executionSupportGrantedAt',
      'isolatedWorktreeReadyAt',
      'handlerSessionCreatedAt',
      'handlerInvocationPreparedAt',
      'handlerHarnessBoundAt',
      'launchRequestedAt',
      'launchAcceptedAt',
      'providerActivationObservedAt',
      'handlerReadyAt',
      'failureReason',
    ],
    'Work Unit Handler activation',
  );
  const optional = (key: keyof typeof x) =>
    x[key] === undefined ? undefined : string(x[key], key);
  const eligibilityState: NativeWorkUnitHandlerActivationV1['eligibilityState'] =
    x.eligibilityState === 'blocked' || x.eligibilityState === 'eligible'
      ? x.eligibilityState
      : fail('invalid Handler eligibility state');
  const blockedReason = optional('blockedReason');
  const failureReason =
    x.failureReason === undefined
      ? undefined
      : boundedString(x.failureReason, 4000, 'Handler activation failureReason');
  if (eligibilityState === 'blocked' && !blockedReason)
    fail('blocked Handler activation requires a reason');
  if (eligibilityState === 'eligible' && blockedReason)
    fail('eligible Handler activation cannot have a blocked reason');
  const result = {
    attemptId: string(x.attemptId, 'attemptId'),
    ...(optional('handlerSessionId') ? { handlerSessionId: optional('handlerSessionId') } : {}),
    ...(optional('handlerInvocationId')
      ? { handlerInvocationId: optional('handlerInvocationId') }
      : {}),
    ...(optional('handlerHarnessRevisionId')
      ? { handlerHarnessRevisionId: optional('handlerHarnessRevisionId') }
      : {}),
    eligibilityState,
    ...(blockedReason ? { blockedReason } : {}),
    ...(optional('requestedAt') ? { requestedAt: optional('requestedAt') } : {}),
    ...(optional('authorizedAt') ? { authorizedAt: optional('authorizedAt') } : {}),
    ...(optional('attemptCreatedAt') ? { attemptCreatedAt: optional('attemptCreatedAt') } : {}),
    ...(optional('executionSupportGrantedAt')
      ? { executionSupportGrantedAt: optional('executionSupportGrantedAt') }
      : {}),
    ...(optional('isolatedWorktreeReadyAt')
      ? { isolatedWorktreeReadyAt: optional('isolatedWorktreeReadyAt') }
      : {}),
    ...(optional('handlerSessionCreatedAt')
      ? { handlerSessionCreatedAt: optional('handlerSessionCreatedAt') }
      : {}),
    ...(optional('handlerInvocationPreparedAt')
      ? { handlerInvocationPreparedAt: optional('handlerInvocationPreparedAt') }
      : {}),
    ...(optional('handlerHarnessBoundAt')
      ? { handlerHarnessBoundAt: optional('handlerHarnessBoundAt') }
      : {}),
    ...(optional('launchRequestedAt') ? { launchRequestedAt: optional('launchRequestedAt') } : {}),
    ...(optional('launchAcceptedAt') ? { launchAcceptedAt: optional('launchAcceptedAt') } : {}),
    ...(optional('providerActivationObservedAt')
      ? { providerActivationObservedAt: optional('providerActivationObservedAt') }
      : {}),
    ...(optional('handlerReadyAt') ? { handlerReadyAt: optional('handlerReadyAt') } : {}),
    ...(failureReason ? { failureReason } : {}),
  };
  if (!result.requestedAt) fail('Handler activation requires its request phase');
  phaseCoherence(
    result,
    [
      'requestedAt',
      'authorizedAt',
      'attemptCreatedAt',
      'executionSupportGrantedAt',
      'isolatedWorktreeReadyAt',
      'handlerSessionCreatedAt',
      'handlerInvocationPreparedAt',
      'handlerHarnessBoundAt',
      'launchRequestedAt',
      'launchAcceptedAt',
      'handlerReadyAt',
    ],
    'Handler activation',
  );
  if (result.providerActivationObservedAt && !result.launchRequestedAt)
    fail('Handler provider observation requires launch request');
  if (result.handlerReadyAt && !result.launchAcceptedAt)
    fail('Handler readiness requires launch acceptance');
  if (result.failureReason && result.handlerReadyAt)
    fail('failed Handler activation cannot be application-ready');
  if (
    result.eligibilityState === 'blocked' &&
    [
      result.authorizedAt,
      result.attemptCreatedAt,
      result.executionSupportGrantedAt,
      result.isolatedWorktreeReadyAt,
      result.handlerSessionCreatedAt,
      result.handlerInvocationPreparedAt,
      result.handlerHarnessBoundAt,
      result.launchRequestedAt,
      result.launchAcceptedAt,
      result.providerActivationObservedAt,
      result.handlerReadyAt,
    ].some(Boolean)
  )
    fail('blocked Handler activation cannot have authorized phases');
  return result;
};
const workUnitActionContinuation = (value: unknown): NativeWorkUnitHandlerActionContinuationV1 => {
  const x = object(value, 'Work Unit Handler action continuation');
  keys(
    x,
    [
      'attemptId',
      'handlerSessionId',
      'originalHandlerInvocationId',
      'actionInvocationId',
      'actionHarnessRevisionId',
      'requestedAt',
      'authorizedAt',
      'invocationPreparedAt',
      'harnessBoundAt',
      'launchRequestedAt',
      'launchAcceptedAt',
      'providerActivationObservedAt',
      'actionReadyAt',
      'blockedReason',
      'failureReason',
    ],
    'Work Unit Handler action continuation',
  );
  const optionalTime = (key: keyof typeof x) =>
    x[key] === undefined ? undefined : timestamp(x[key], key);
  const blockedReason =
    x.blockedReason === undefined
      ? undefined
      : boundedString(x.blockedReason, 4000, 'blockedReason');
  const failureReason =
    x.failureReason === undefined
      ? undefined
      : boundedString(x.failureReason, 4000, 'failureReason');
  const result = {
    attemptId: boundedString(x.attemptId, 240, 'action continuation attemptId'),
    handlerSessionId: boundedString(
      x.handlerSessionId,
      240,
      'action continuation handlerSessionId',
    ),
    originalHandlerInvocationId: boundedString(
      x.originalHandlerInvocationId,
      240,
      'originalHandlerInvocationId',
    ),
    actionInvocationId: boundedString(x.actionInvocationId, 240, 'actionInvocationId'),
    actionHarnessRevisionId: boundedString(
      x.actionHarnessRevisionId,
      240,
      'actionHarnessRevisionId',
    ),
    requestedAt: timestamp(x.requestedAt, 'action continuation requestedAt'),
    ...(optionalTime('authorizedAt') ? { authorizedAt: optionalTime('authorizedAt') } : {}),
    ...(optionalTime('invocationPreparedAt')
      ? { invocationPreparedAt: optionalTime('invocationPreparedAt') }
      : {}),
    ...(optionalTime('harnessBoundAt') ? { harnessBoundAt: optionalTime('harnessBoundAt') } : {}),
    ...(optionalTime('launchRequestedAt')
      ? { launchRequestedAt: optionalTime('launchRequestedAt') }
      : {}),
    ...(optionalTime('launchAcceptedAt')
      ? { launchAcceptedAt: optionalTime('launchAcceptedAt') }
      : {}),
    ...(optionalTime('providerActivationObservedAt')
      ? { providerActivationObservedAt: optionalTime('providerActivationObservedAt') }
      : {}),
    ...(optionalTime('actionReadyAt') ? { actionReadyAt: optionalTime('actionReadyAt') } : {}),
    ...(blockedReason ? { blockedReason } : {}),
    ...(failureReason ? { failureReason } : {}),
  };
  if (result.actionInvocationId === result.originalHandlerInvocationId)
    fail('Handler action invocation must differ from the original Handler invocation');
  phaseCoherence(
    result,
    [
      'requestedAt',
      'authorizedAt',
      'invocationPreparedAt',
      'harnessBoundAt',
      'launchRequestedAt',
      'launchAcceptedAt',
      'actionReadyAt',
    ],
    'Handler action continuation',
  );
  if (result.providerActivationObservedAt && !result.launchRequestedAt)
    fail('Handler action provider observation requires launch request');
  if (result.actionReadyAt && !result.launchAcceptedAt)
    fail('Handler action readiness requires launch acceptance');
  if (
    result.blockedReason &&
    [
      result.authorizedAt,
      result.invocationPreparedAt,
      result.harnessBoundAt,
      result.launchRequestedAt,
      result.launchAcceptedAt,
      result.providerActivationObservedAt,
      result.actionReadyAt,
    ].some(Boolean)
  )
    fail('blocked Handler action continuation cannot have authorized action phases');
  if (result.failureReason && result.actionReadyAt)
    fail('failed Handler action continuation cannot be application-ready');
  if (result.failureReason && result.blockedReason)
    fail('Handler action continuation cannot be both blocked and failed');
  return result;
};

const workUnitImplementerActivation = (value: unknown): NativeWorkUnitImplementerActivationV1 => {
  const x = object(value, 'Work Unit Implementer activation');
  keys(
    x,
    [
      'attemptId',
      'handlerActionInvocationId',
      'implementerSessionId',
      'implementerInvocationId',
      'implementerHarnessRevisionId',
      'requestedAt',
      'authorizedAt',
      'executionSupportGrantedAt',
      'isolatedWorktreeReadyAt',
      'implementerSessionCreatedAt',
      'implementerInvocationPreparedAt',
      'implementerHarnessBoundAt',
      'launchRequestedAt',
      'launchAcceptedAt',
      'providerActivationObservedAt',
      'implementerReadyAt',
      'failureReason',
    ],
    'Work Unit Implementer activation',
  );
  const optionalTime = (key: keyof typeof x) =>
    x[key] === undefined ? undefined : timestamp(x[key], key);
  const failureReason =
    x.failureReason === undefined
      ? undefined
      : boundedString(x.failureReason, 4000, 'failureReason');
  const result = {
    attemptId: boundedString(x.attemptId, 240, 'Implementer attemptId'),
    handlerActionInvocationId: boundedString(
      x.handlerActionInvocationId,
      240,
      'handlerActionInvocationId',
    ),
    implementerSessionId: boundedString(x.implementerSessionId, 240, 'implementerSessionId'),
    implementerInvocationId: boundedString(
      x.implementerInvocationId,
      240,
      'implementerInvocationId',
    ),
    implementerHarnessRevisionId: boundedString(
      x.implementerHarnessRevisionId,
      240,
      'implementerHarnessRevisionId',
    ),
    requestedAt: timestamp(x.requestedAt, 'Implementer requestedAt'),
    ...(optionalTime('authorizedAt') ? { authorizedAt: optionalTime('authorizedAt') } : {}),
    ...(optionalTime('executionSupportGrantedAt')
      ? { executionSupportGrantedAt: optionalTime('executionSupportGrantedAt') }
      : {}),
    ...(optionalTime('isolatedWorktreeReadyAt')
      ? { isolatedWorktreeReadyAt: optionalTime('isolatedWorktreeReadyAt') }
      : {}),
    ...(optionalTime('implementerSessionCreatedAt')
      ? { implementerSessionCreatedAt: optionalTime('implementerSessionCreatedAt') }
      : {}),
    ...(optionalTime('implementerInvocationPreparedAt')
      ? { implementerInvocationPreparedAt: optionalTime('implementerInvocationPreparedAt') }
      : {}),
    ...(optionalTime('implementerHarnessBoundAt')
      ? { implementerHarnessBoundAt: optionalTime('implementerHarnessBoundAt') }
      : {}),
    ...(optionalTime('launchRequestedAt')
      ? { launchRequestedAt: optionalTime('launchRequestedAt') }
      : {}),
    ...(optionalTime('launchAcceptedAt')
      ? { launchAcceptedAt: optionalTime('launchAcceptedAt') }
      : {}),
    ...(optionalTime('providerActivationObservedAt')
      ? { providerActivationObservedAt: optionalTime('providerActivationObservedAt') }
      : {}),
    ...(optionalTime('implementerReadyAt')
      ? { implementerReadyAt: optionalTime('implementerReadyAt') }
      : {}),
    ...(failureReason ? { failureReason } : {}),
  };
  phaseCoherence(
    result,
    [
      'requestedAt',
      'authorizedAt',
      'executionSupportGrantedAt',
      'isolatedWorktreeReadyAt',
      'implementerSessionCreatedAt',
      'implementerInvocationPreparedAt',
      'implementerHarnessBoundAt',
      'launchRequestedAt',
      'launchAcceptedAt',
      'implementerReadyAt',
    ],
    'Implementer activation',
  );
  if (result.providerActivationObservedAt && !result.launchRequestedAt)
    fail('Implementer provider observation requires launch request');
  if (result.implementerInvocationId === result.handlerActionInvocationId)
    fail('Implementer invocation must differ from the Handler action invocation');
  if (result.implementerReadyAt && !result.launchAcceptedAt)
    fail('Implementer readiness requires launch acceptance');
  if (result.failureReason && result.implementerReadyAt)
    fail('failed Implementer activation cannot be application-ready');
  return result;
};
const workUnitImplementerOutcome = (value: unknown): NativeWorkUnitImplementerOutcomeV1 => {
  const x = object(value, 'Work Unit Implementer outcome');
  keys(
    x,
    [
      'attemptId',
      'implementerSessionId',
      'originalImplementerInvocationId',
      'reportingInvocationId',
      'reportingHarnessRevisionId',
      'reportingRequestedAt',
      'reportingPreparedAt',
      'reportingHarnessBoundAt',
      'reportingLaunchRequestedAt',
      'reportingLaunchAcceptedAt',
      'reportingReadyAt',
      'submittedOutcome',
      'evidence',
      'semanticCompletion',
      'terminalLifecycle',
      'applicationAcceptedAt',
      'handlerReviewReadyAt',
      'failureReason',
    ],
    'Work Unit Implementer outcome',
  );
  const optionalTime = (key: keyof typeof x) =>
    x[key] === undefined ? undefined : timestamp(x[key], key);
  const submittedOutcome =
    x.submittedOutcome === undefined
      ? undefined
      : (() => {
          const submission = object(x.submittedOutcome, 'Implementer submitted outcome');
          keys(
            submission,
            [
              'variant',
              'summaryClaim',
              'validationStatementClaim',
              'semanticPayloadFingerprint',
              'submittedAt',
              'validationAt',
              'validationResult',
            ],
            'Implementer submitted outcome',
          );
          if (submission.variant !== 'review_pending') fail('invalid Implementer outcome variant');
          if (submission.validationResult !== 'valid')
            fail('invalid Implementer outcome validation result');
          return {
            variant: 'review_pending' as const,
            summaryClaim: boundedString(
              submission.summaryClaim,
              20_000,
              'Implementer summary claim',
            ),
            validationStatementClaim: boundedString(
              submission.validationStatementClaim,
              20_000,
              'Implementer validation statement claim',
            ),
            semanticPayloadFingerprint: boundedString(
              submission.semanticPayloadFingerprint,
              240,
              'Implementer semantic payload fingerprint',
            ),
            submittedAt: timestamp(submission.submittedAt, 'Implementer submittedAt'),
            validationAt: timestamp(submission.validationAt, 'Implementer validationAt'),
            validationResult: 'valid' as const,
          };
        })();
  const evidence =
    x.evidence === undefined
      ? undefined
      : (() => {
          const evidence = object(x.evidence, 'Implementer evidence');
          keys(
            evidence,
            ['changedFiles', 'comparisonFingerprint', 'readyAt'],
            'Implementer evidence',
          );
          const changedFiles = array(evidence.changedFiles, 'Implementer changed files').map(
            (value) => {
              const file = object(value, 'Implementer changed file');
              keys(
                file,
                ['evidenceRef', 'displayName', 'changeKind', 'contentFingerprint'],
                'Implementer changed file',
              );
              if (!['added', 'modified', 'deleted', 'renamed'].includes(file.changeKind as string))
                fail('invalid Implementer evidence change kind');
              return {
                evidenceRef: boundedString(file.evidenceRef, 240, 'Implementer evidenceRef'),
                displayName: boundedString(
                  file.displayName,
                  1000,
                  'Implementer evidence displayName',
                ),
                changeKind: file.changeKind as 'added' | 'modified' | 'deleted' | 'renamed',
                contentFingerprint: boundedString(
                  file.contentFingerprint,
                  240,
                  'Implementer evidence content fingerprint',
                ),
              };
            },
          );
          if (!changedFiles.length || changedFiles.length > 500)
            fail('Implementer evidence must contain a bounded changed-file manifest');
          unique(changedFiles, (file) => file.evidenceRef, 'Implementer evidence reference');
          return {
            changedFiles,
            comparisonFingerprint: boundedString(
              evidence.comparisonFingerprint,
              240,
              'Implementer comparison fingerprint',
            ),
            readyAt: timestamp(evidence.readyAt, 'Implementer evidence readyAt'),
          };
        })();
  const semanticCompletion =
    x.semanticCompletion === undefined
      ? undefined
      : (() => {
          const completion = object(x.semanticCompletion, 'Implementer semantic completion');
          keys(completion, ['invocationId', 'completedAt'], 'Implementer semantic completion');
          return {
            invocationId: boundedString(
              completion.invocationId,
              240,
              'semantic completion invocationId',
            ),
            completedAt: timestamp(completion.completedAt, 'semantic completion completedAt'),
          };
        })();
  const terminalLifecycle =
    x.terminalLifecycle === undefined
      ? undefined
      : (() => {
          const lifecycle = object(x.terminalLifecycle, 'Implementer terminal lifecycle');
          keys(lifecycle, ['status', 'observedAt'], 'Implementer terminal lifecycle');
          if (
            !['completed', 'failed', 'canceled', 'interrupted'].includes(lifecycle.status as string)
          )
            fail('invalid Implementer reporting lifecycle status');
          return {
            status: lifecycle.status as 'completed' | 'failed' | 'canceled' | 'interrupted',
            observedAt: timestamp(lifecycle.observedAt, 'terminal lifecycle observedAt'),
          };
        })();
  const result: NativeWorkUnitImplementerOutcomeV1 = {
    attemptId: boundedString(x.attemptId, 240, 'outcome attemptId'),
    implementerSessionId: boundedString(
      x.implementerSessionId,
      240,
      'outcome implementerSessionId',
    ),
    originalImplementerInvocationId: boundedString(
      x.originalImplementerInvocationId,
      240,
      'originalImplementerInvocationId',
    ),
    reportingInvocationId: boundedString(x.reportingInvocationId, 240, 'reportingInvocationId'),
    reportingHarnessRevisionId: boundedString(
      x.reportingHarnessRevisionId,
      240,
      'reportingHarnessRevisionId',
    ),
    reportingRequestedAt: timestamp(x.reportingRequestedAt, 'reportingRequestedAt'),
    ...(optionalTime('reportingPreparedAt')
      ? { reportingPreparedAt: optionalTime('reportingPreparedAt') }
      : {}),
    ...(optionalTime('reportingHarnessBoundAt')
      ? { reportingHarnessBoundAt: optionalTime('reportingHarnessBoundAt') }
      : {}),
    ...(optionalTime('reportingLaunchRequestedAt')
      ? { reportingLaunchRequestedAt: optionalTime('reportingLaunchRequestedAt') }
      : {}),
    ...(optionalTime('reportingLaunchAcceptedAt')
      ? { reportingLaunchAcceptedAt: optionalTime('reportingLaunchAcceptedAt') }
      : {}),
    ...(optionalTime('reportingReadyAt')
      ? { reportingReadyAt: optionalTime('reportingReadyAt') }
      : {}),
    ...(submittedOutcome ? { submittedOutcome } : {}),
    ...(evidence ? { evidence } : {}),
    ...(semanticCompletion ? { semanticCompletion } : {}),
    ...(terminalLifecycle ? { terminalLifecycle } : {}),
    ...(optionalTime('applicationAcceptedAt')
      ? { applicationAcceptedAt: optionalTime('applicationAcceptedAt') }
      : {}),
    ...(optionalTime('handlerReviewReadyAt')
      ? { handlerReviewReadyAt: optionalTime('handlerReviewReadyAt') }
      : {}),
    ...(x.failureReason === undefined
      ? {}
      : { failureReason: boundedString(x.failureReason, 4000, 'reporting failureReason') }),
  };
  phaseCoherence(
    result,
    [
      'reportingRequestedAt',
      'reportingPreparedAt',
      'reportingHarnessBoundAt',
      'reportingLaunchRequestedAt',
      'reportingLaunchAcceptedAt',
      'reportingReadyAt',
    ],
    'Implementer reporting',
  );
  if (result.submittedOutcome) {
    if (!result.reportingReadyAt)
      fail('Implementer outcome submission requires reporting readiness');
    timestampAtOrAfter(
      result.reportingReadyAt,
      result.submittedOutcome.submittedAt,
      'Implementer outcome submission',
    );
    timestampAtOrAfter(
      result.submittedOutcome.submittedAt,
      result.submittedOutcome.validationAt,
      'Implementer outcome validation',
    );
  }
  if (result.evidence) {
    if (!result.submittedOutcome) fail('Implementer evidence requires a validated submission');
    timestampAtOrAfter(
      result.submittedOutcome.validationAt,
      result.evidence.readyAt,
      'Implementer evidence readiness',
    );
  }
  if (result.semanticCompletion) {
    if (!result.evidence) fail('Implementer semantic completion requires evidence');
    if (result.semanticCompletion.invocationId !== result.reportingInvocationId)
      fail('Implementer semantic completion has a foreign invocation');
    timestampAtOrAfter(
      result.evidence.readyAt,
      result.semanticCompletion.completedAt,
      'Implementer semantic completion',
    );
  }
  if (result.terminalLifecycle) {
    if (!result.reportingReadyAt)
      fail('Implementer terminal lifecycle requires reporting readiness');
    timestampAtOrAfter(
      result.reportingReadyAt,
      result.terminalLifecycle.observedAt,
      'Implementer terminal lifecycle observation',
    );
    if (result.semanticCompletion)
      timestampAtOrAfter(
        result.semanticCompletion.completedAt,
        result.terminalLifecycle.observedAt,
        'Implementer terminal lifecycle observation',
      );
  }
  if (result.applicationAcceptedAt) {
    if (
      !result.submittedOutcome ||
      !result.evidence ||
      !result.semanticCompletion ||
      result.terminalLifecycle?.status !== 'completed'
    )
      fail(
        'Implementer application acceptance lacks exact semantic, evidence, or Completed prerequisites',
      );
    timestampAtOrAfter(
      result.terminalLifecycle.observedAt,
      result.applicationAcceptedAt,
      'Implementer application acceptance',
    );
  }
  if (result.handlerReviewReadyAt) {
    if (!result.applicationAcceptedAt)
      fail('Handler review readiness requires Implementer application acceptance');
    timestampAtOrAfter(
      result.applicationAcceptedAt,
      result.handlerReviewReadyAt,
      'Handler review readiness',
    );
  }
  return result;
};
const handlerReviewReason = (value: unknown, label: string) => {
  const x = object(value, label);
  keys(x, ['code', 'explanation'], label);
  const code = boundedString(x.code, 96, `${label} code`);
  if (!/^[A-Za-z0-9_-]+$/.test(code)) fail(`${label} code is not a bounded identifier`);
  return { code, explanation: boundedString(x.explanation, 2_000, `${label} explanation`) };
};
const workUnitHandlerReview = (value: unknown): NativeWorkUnitHandlerReviewV1 => {
  const x = object(value, 'Work Unit Handler review');
  keys(
    x,
    [
      'attemptId',
      'reportingInvocationId',
      'handlerSessionId',
      'originalHandlerInvocationId',
      'actionHandlerInvocationId',
      'reviewInvocationId',
      'reviewHarnessRevisionId',
      'deliveryRequestedAt',
      'deliveryPersistedAt',
      'harnessBoundAt',
      'launchRequestedAt',
      'launchAcceptedAt',
      'reviewReadyAt',
      'delivered',
      'semanticJudgment',
      'lifecycle',
      'conflict',
    ],
    'Work Unit Handler review',
  );
  const optionalTime = (key: keyof typeof x) =>
    x[key] === undefined ? undefined : timestamp(x[key], key);
  const deliveredValue = object(x.delivered, 'Handler review delivered evidence');
  keys(
    deliveredValue,
    [
      'summaryClaim',
      'validationStatementClaim',
      'changedFiles',
      'comparisonFingerprint',
      'deliveredPayloadFingerprint',
    ],
    'Handler review delivered evidence',
  );
  const changedFiles = array(
    deliveredValue.changedFiles,
    'Handler review delivered changed files',
  ).map((value) => {
    const file = object(value, 'Handler review changed file');
    keys(
      file,
      ['evidenceRef', 'displayName', 'changeKind', 'contentFingerprint'],
      'Handler review changed file',
    );
    if (!['added', 'modified', 'deleted', 'renamed'].includes(file.changeKind as string))
      fail('invalid Handler review evidence change kind');
    return {
      evidenceRef: boundedString(file.evidenceRef, 240, 'Handler review evidenceRef'),
      displayName: boundedString(file.displayName, 1_000, 'Handler review displayName'),
      changeKind: file.changeKind as 'added' | 'modified' | 'deleted' | 'renamed',
      contentFingerprint: boundedString(
        file.contentFingerprint,
        240,
        'Handler review content fingerprint',
      ),
    };
  });
  if (!changedFiles.length || changedFiles.length > 500)
    fail('Handler review evidence must contain a bounded changed-file manifest');
  unique(changedFiles, (file) => file.evidenceRef, 'Handler review evidence reference');
  const semanticJudgment =
    x.semanticJudgment === undefined
      ? undefined
      : (() => {
          const judgment = object(x.semanticJudgment, 'Handler review semantic judgment');
          keys(
            judgment,
            ['variant', 'reason', 'fingerprint', 'recordedAt'],
            'Handler review semantic judgment',
          );
          if (judgment.variant !== 'accept' && judgment.variant !== 'return')
            fail('invalid Handler review semantic judgment variant');
          const reason =
            judgment.reason === undefined
              ? undefined
              : handlerReviewReason(judgment.reason, 'Handler review semantic judgment reason');
          if (judgment.variant === 'return' && !reason)
            fail('returned Handler review judgment requires a reason');
          if (judgment.variant === 'accept' && reason)
            fail('accepted Handler review judgment cannot have a reason');
          return {
            variant: judgment.variant as 'accept' | 'return',
            ...(reason ? { reason } : {}),
            fingerprint: boundedString(
              judgment.fingerprint,
              240,
              'Handler review judgment fingerprint',
            ),
            recordedAt: timestamp(judgment.recordedAt, 'Handler review judgment recordedAt'),
          };
        })();
  const lifecycle =
    x.lifecycle === undefined
      ? undefined
      : (() => {
          const value = object(x.lifecycle, 'Handler review lifecycle');
          keys(value, ['status', 'observedAt'], 'Handler review lifecycle');
          if (!['completed', 'failed', 'canceled', 'interrupted'].includes(value.status as string))
            fail('invalid Handler review lifecycle status');
          return {
            status: value.status as 'completed' | 'failed' | 'canceled' | 'interrupted',
            observedAt: timestamp(value.observedAt, 'Handler review lifecycle observedAt'),
          };
        })();
  const conflict =
    x.conflict === undefined
      ? undefined
      : (() => {
          const value = object(x.conflict, 'Handler review conflict');
          keys(value, ['occurredAt', 'reason'], 'Handler review conflict');
          return {
            occurredAt: timestamp(value.occurredAt, 'Handler review conflict occurredAt'),
            reason: boundedString(value.reason, 4_000, 'Handler review conflict reason'),
          };
        })();
  const result: NativeWorkUnitHandlerReviewV1 = {
    attemptId: boundedString(x.attemptId, 240, 'Handler review attemptId'),
    reportingInvocationId: boundedString(
      x.reportingInvocationId,
      240,
      'Handler review reportingInvocationId',
    ),
    handlerSessionId: boundedString(x.handlerSessionId, 240, 'Handler review handlerSessionId'),
    originalHandlerInvocationId: boundedString(
      x.originalHandlerInvocationId,
      240,
      'Handler review originalHandlerInvocationId',
    ),
    actionHandlerInvocationId: boundedString(
      x.actionHandlerInvocationId,
      240,
      'Handler review actionHandlerInvocationId',
    ),
    reviewInvocationId: boundedString(x.reviewInvocationId, 240, 'Handler review reviewInvocationId'),
    reviewHarnessRevisionId: boundedString(
      x.reviewHarnessRevisionId,
      240,
      'Handler review reviewHarnessRevisionId',
    ),
    deliveryRequestedAt: timestamp(x.deliveryRequestedAt, 'Handler review deliveryRequestedAt'),
    ...(optionalTime('deliveryPersistedAt')
      ? { deliveryPersistedAt: optionalTime('deliveryPersistedAt') }
      : {}),
    ...(optionalTime('harnessBoundAt') ? { harnessBoundAt: optionalTime('harnessBoundAt') } : {}),
    ...(optionalTime('launchRequestedAt')
      ? { launchRequestedAt: optionalTime('launchRequestedAt') }
      : {}),
    ...(optionalTime('launchAcceptedAt')
      ? { launchAcceptedAt: optionalTime('launchAcceptedAt') }
      : {}),
    ...(optionalTime('reviewReadyAt') ? { reviewReadyAt: optionalTime('reviewReadyAt') } : {}),
    delivered: {
      summaryClaim: boundedString(
        deliveredValue.summaryClaim,
        20_000,
        'Handler review summary claim',
      ),
      validationStatementClaim: boundedString(
        deliveredValue.validationStatementClaim,
        20_000,
        'Handler review validation statement claim',
      ),
      changedFiles,
      comparisonFingerprint: boundedString(
        deliveredValue.comparisonFingerprint,
        240,
        'Handler review comparison fingerprint',
      ),
      deliveredPayloadFingerprint: boundedString(
        deliveredValue.deliveredPayloadFingerprint,
        240,
        'Handler review delivered payload fingerprint',
      ),
    },
    ...(semanticJudgment ? { semanticJudgment } : {}),
    ...(lifecycle ? { lifecycle } : {}),
    ...(conflict ? { conflict } : {}),
  };
  phaseCoherence(
    result,
    [
      'deliveryRequestedAt',
      'deliveryPersistedAt',
      'harnessBoundAt',
      'launchRequestedAt',
      'launchAcceptedAt',
      'reviewReadyAt',
    ],
    'Handler review',
  );
  if (result.semanticJudgment) {
    if (!result.reviewReadyAt)
      fail('Handler review judgment requires review readiness');
    timestampAtOrAfter(
      result.reviewReadyAt,
      result.semanticJudgment.recordedAt,
      'Handler review judgment',
    );
  }
  if (result.lifecycle) {
    if (!result.reviewReadyAt) fail('Handler review lifecycle requires review readiness');
    timestampAtOrAfter(result.reviewReadyAt, result.lifecycle.observedAt, 'Handler review lifecycle');
    if (result.semanticJudgment)
      timestampAtOrAfter(
        result.semanticJudgment.recordedAt,
        result.lifecycle.observedAt,
        'Handler review lifecycle',
      );
  }
  return result;
};
const workUnitHandlerDecision = (value: unknown): NativeWorkUnitHandlerDecisionV1 => {
  const x = object(value, 'Work Unit Handler decision');
  keys(
    x,
    [
      'attemptId',
      'reviewInvocationId',
      'variant',
      'fingerprint',
      'returnReason',
      'recordedAt',
      'implementationAcceptedAt',
      'implementationReturnedAt',
      'retryRequiredAt',
      'settlementReadyAt',
    ],
    'Work Unit Handler decision',
  );
  if (x.settlementReadyAt !== undefined) fail('Handler decision has forbidden settlement readiness');
  if (x.variant !== 'accepted' && x.variant !== 'returned') fail('invalid Handler decision variant');
  const returnReason =
    x.returnReason === undefined
      ? undefined
      : handlerReviewReason(x.returnReason, 'Handler decision return reason');
  const optionalTime = (key: keyof typeof x) =>
    x[key] === undefined ? undefined : timestamp(x[key], key);
  const result: NativeWorkUnitHandlerDecisionV1 = {
    attemptId: boundedString(x.attemptId, 240, 'Handler decision attemptId'),
    reviewInvocationId: boundedString(x.reviewInvocationId, 240, 'Handler decision reviewInvocationId'),
    variant: x.variant as 'accepted' | 'returned',
    fingerprint: boundedString(x.fingerprint, 240, 'Handler decision fingerprint'),
    ...(returnReason ? { returnReason } : {}),
    recordedAt: timestamp(x.recordedAt, 'Handler decision recordedAt'),
    ...(optionalTime('implementationAcceptedAt')
      ? { implementationAcceptedAt: optionalTime('implementationAcceptedAt') }
      : {}),
    ...(optionalTime('implementationReturnedAt')
      ? { implementationReturnedAt: optionalTime('implementationReturnedAt') }
      : {}),
    ...(optionalTime('retryRequiredAt') ? { retryRequiredAt: optionalTime('retryRequiredAt') } : {}),
  };
  if (result.variant === 'accepted') {
    if (returnReason || !result.implementationAcceptedAt || result.implementationReturnedAt || result.retryRequiredAt)
      fail('accepted Handler decision facts contradict their variant');
  } else if (!returnReason || !result.implementationReturnedAt || result.implementationAcceptedAt) {
    fail('returned Handler decision facts contradict their variant');
  }
  return result;
};
const workUnitIncompleteDisposition = (value: unknown): NativeWorkUnitIncompleteDispositionV1 => {
  const x = object(value, 'Work Unit incomplete disposition');
  keys(x, ['attemptId', 'reviewInvocationId', 'decisionFingerprint', 'classification', 'meaningfulProgress', 'recordedAt', 'nextAttemptAuthorizedAt', 'noProgressHandback'], 'Work Unit incomplete disposition');
  if (!['refinement_needed', 'functional_objective_not_satisfied', 'blocked'].includes(x.classification as string) || typeof x.meaningfulProgress !== 'boolean') fail('invalid Work Unit incomplete disposition');
  const nextAttemptAuthorizedAt = x.nextAttemptAuthorizedAt === undefined ? undefined : timestamp(x.nextAttemptAuthorizedAt, 'nextAttemptAuthorizedAt');
  const noProgressHandback = x.noProgressHandback === undefined ? undefined : (() => {
    const handback = object(x.noProgressHandback, 'no-progress Work Unit handback');
    keys(handback, ['handbackId', 'sourceAttemptId', 'sourceReviewInvocationId', 'contextFingerprint', 'persistedAt', 'deliveryIntendedAt', 'sprintRunnerReceiverActivatedAt', 'sprintRunnerReceiverDecisionAt', 'sprintRunnerDelivery', 'epicRunnerReceiver'], 'no-progress Work Unit handback');
    if (handback.sprintRunnerReceiverActivatedAt !== undefined || handback.sprintRunnerReceiverDecisionAt !== undefined) fail('no-progress Work Unit handback has forbidden receiver effects');
    const persistedAt = timestamp(handback.persistedAt, 'persistedAt');
    const deliveryIntendedAt = timestamp(handback.deliveryIntendedAt, 'deliveryIntendedAt');
    if (Date.parse(deliveryIntendedAt) < Date.parse(persistedAt))
      fail('no-progress handback delivery intent precedes persistence');
    const sprintRunnerDelivery =
      handback.sprintRunnerDelivery === undefined
        ? undefined
        : sprintRunnerHandbackDelivery(handback.sprintRunnerDelivery);
    const epicRunnerReceiver = handback.epicRunnerReceiver === undefined ? undefined : epicEscalationReceiver(handback.epicRunnerReceiver);
    if (sprintRunnerDelivery && Date.parse(sprintRunnerDelivery.deliveryRequestedAt) < Date.parse(deliveryIntendedAt))
      fail('Sprint Runner delivery request precedes delivery intent');
    return {
      handbackId: boundedString(handback.handbackId, 240, 'handbackId'),
      sourceAttemptId: boundedString(handback.sourceAttemptId, 240, 'sourceAttemptId'),
      sourceReviewInvocationId: boundedString(handback.sourceReviewInvocationId, 240, 'sourceReviewInvocationId'),
      contextFingerprint: boundedString(handback.contextFingerprint, 240, 'contextFingerprint'),
      persistedAt,
      deliveryIntendedAt,
      ...(sprintRunnerDelivery ? { sprintRunnerDelivery } : {}),
      ...(epicRunnerReceiver ? { epicRunnerReceiver } : {}),
    };
  })();
  if (x.meaningfulProgress ? !nextAttemptAuthorizedAt || noProgressHandback : nextAttemptAuthorizedAt !== undefined || !noProgressHandback) fail('incomplete disposition has incoherent later effects');
  return { attemptId: boundedString(x.attemptId, 240, 'incomplete disposition attemptId'), reviewInvocationId: boundedString(x.reviewInvocationId, 240, 'incomplete disposition reviewInvocationId'), decisionFingerprint: boundedString(x.decisionFingerprint, 240, 'incomplete disposition decisionFingerprint'), classification: x.classification as NativeWorkUnitIncompleteDispositionV1['classification'], meaningfulProgress: x.meaningfulProgress as boolean, recordedAt: timestamp(x.recordedAt, 'incomplete disposition recordedAt'), ...(nextAttemptAuthorizedAt ? { nextAttemptAuthorizedAt } : {}), ...(noProgressHandback ? { noProgressHandback } : {}) };
};

const epicEscalationReceiver = (value: unknown): NativeEpicEscalationReceiverV1 => {
  const x = object(value, 'Epic escalation receiver');
  keys(x, ['sprintId', 'epicId', 'deliveryRequestedAt', 'deliveryPersistedAt', 'harnessBoundAt', 'launchRequestedAt', 'launchAcceptedAt', 'providerActivationObservedAt', 'reassessmentLifecycleStatus', 'reassessmentLifecycleObservedAt', 'semanticReassessmentRecordedAt', 'disposition'], 'Epic escalation receiver');
  const time = (key: string) => x[key] === undefined ? undefined : timestamp(x[key], `Epic receiver ${key}`);
  const deliveryRequestedAt = timestamp(x.deliveryRequestedAt, 'Epic receiver deliveryRequestedAt');
  const deliveryPersistedAt = time('deliveryPersistedAt');
  const harnessBoundAt = time('harnessBoundAt');
  const launchRequestedAt = time('launchRequestedAt');
  const launchAcceptedAt = time('launchAcceptedAt');
  const providerActivationObservedAt = time('providerActivationObservedAt');
  const semanticReassessmentRecordedAt = time('semanticReassessmentRecordedAt');
  phaseCoherence({ deliveryRequestedAt, deliveryPersistedAt, harnessBoundAt, launchRequestedAt, launchAcceptedAt }, ['deliveryRequestedAt', 'deliveryPersistedAt', 'harnessBoundAt', 'launchRequestedAt', 'launchAcceptedAt'], 'Epic escalation receiver');
  if ((providerActivationObservedAt || semanticReassessmentRecordedAt) && !launchAcceptedAt) fail('Epic receiver later observation lacks launch acceptance');
  if (semanticReassessmentRecordedAt && x.disposition === undefined) { /* reassessment may end without a semantic disposition */ }
  const disposition = x.disposition === undefined ? undefined : epicEscalationDisposition(x.disposition);
  if (disposition && !semanticReassessmentRecordedAt) fail('Epic disposition lacks semantic reassessment');
  return { sprintId: boundedString(x.sprintId, 240, 'Epic receiver sprintId'), epicId: boundedString(x.epicId, 240, 'Epic receiver epicId'), deliveryRequestedAt, ...(deliveryPersistedAt ? { deliveryPersistedAt } : {}), ...(harnessBoundAt ? { harnessBoundAt } : {}), ...(launchRequestedAt ? { launchRequestedAt } : {}), ...(launchAcceptedAt ? { launchAcceptedAt } : {}), ...(providerActivationObservedAt ? { providerActivationObservedAt } : {}), ...(x.reassessmentLifecycleStatus === undefined ? {} : { reassessmentLifecycleStatus: boundedString(x.reassessmentLifecycleStatus, 96, 'Epic receiver lifecycle status') }), ...(x.reassessmentLifecycleObservedAt === undefined ? {} : { reassessmentLifecycleObservedAt: timestamp(x.reassessmentLifecycleObservedAt, 'Epic receiver lifecycle observedAt') }), ...(semanticReassessmentRecordedAt ? { semanticReassessmentRecordedAt } : {}), ...(disposition ? { disposition } : {}) };
};

const epicEscalationDisposition = (value: unknown): ProductEpicEscalationDispositionV1 => {
  const x = object(value, 'Epic escalation disposition');
  keys(x, ['movementKind', 'rationale', 'consideredIntent', 'downstreamRequest', 'humanExternalAttention'], 'Epic escalation disposition');
  const movementKind = boundedString(x.movementKind, 96, 'Epic disposition movementKind');
  if (!/^[A-Za-z0-9_.-]+$/.test(movementKind)) fail('invalid Epic disposition movementKind');
  const rationale = boundedString(x.rationale, 20_000, 'Epic disposition rationale');
  const downstreamRequest = x.downstreamRequest === undefined ? undefined : (() => {
    const request = object(x.downstreamRequest, 'Epic downstream request');
    keys(request, ['target', 'dependency', 'request', 'resumptionPath'], 'Epic downstream request');
    if (!['sprint_runner', 'existing_agent_achievable_dependency'].includes(request.target as string)) fail('invalid Epic downstream target');
    if (request.target === 'existing_agent_achievable_dependency' && request.dependency !== 'work_unit_handler') fail('invalid Epic known dependency');
    if (request.target === 'sprint_runner' && request.dependency !== undefined) fail('Sprint Runner request has dependency detail');
    return { target: request.target as 'sprint_runner' | 'existing_agent_achievable_dependency', ...(request.dependency === undefined ? {} : { dependency: 'work_unit_handler' as const }), request: boundedString(request.request, 20_000, 'Epic downstream request text'), resumptionPath: boundedString(request.resumptionPath, 20_000, 'Epic downstream resumption path') };
  })();
  const humanExternalAttention = x.humanExternalAttention === undefined ? undefined : (() => {
    const attention = object(x.humanExternalAttention, 'Epic human/external attention');
    keys(attention, ['reason', 'authorityNeeded', 'evidenceContext', 'resumptionPath'], 'Epic human/external attention');
    return { reason: boundedString(attention.reason, 20_000, 'Epic attention reason'), authorityNeeded: boundedString(attention.authorityNeeded, 20_000, 'Epic attention authority'), evidenceContext: boundedString(attention.evidenceContext, 20_000, 'Epic attention evidence'), resumptionPath: boundedString(attention.resumptionPath, 20_000, 'Epic attention resumption path') };
  })();
  if (downstreamRequest && humanExternalAttention) fail('Epic disposition mixes downstream request and attention');
  return { movementKind, rationale, ...(x.consideredIntent === undefined ? {} : { consideredIntent: boundedString(x.consideredIntent, 20_000, 'Epic considered intent') }), ...(downstreamRequest ? { downstreamRequest } : {}), ...(humanExternalAttention ? { humanExternalAttention } : {}) };
};

const sprintRunnerHandbackMovement = (value: unknown): ProductSprintRunnerHandbackMovementV1 => {
  const x = object(value, 'Sprint Runner Handback movement');
  keys(
    x,
    [
      'movementKind',
      'rationale',
      'eligibleWorkSummary',
      'dependencyOwner',
      'dependencyOwnerClassification',
      'enablingResult',
      'resumptionPath',
      'localExhaustionSummary',
      'boundedDetails',
    ],
    'Sprint Runner Handback movement',
  );
  const movementKind = string(x.movementKind, 'Handback movementKind');
  if (!/^[A-Za-z0-9_.-]+$/.test(movementKind) || movementKind.length > 96) fail('invalid Handback movementKind');
  const rationale = boundedString(x.rationale, 20_000, 'Handback rationale');
  const boundedDetails = x.boundedDetails === undefined ? undefined : array(x.boundedDetails, 'boundedDetails').map((detail, index): ProductSprintRunnerHandbackBoundedDetailV1 => {
    const item = object(detail, `boundedDetails[${index}]`);
    keys(item, ['label', 'value'], `boundedDetails[${index}]`);
    const label = boundedString(item.label, 96, `boundedDetails[${index}].label`);
    if (!['eligibleWorkSummary', 'dependencyOwner', 'dependencyOwnerClassification', 'enablingResult', 'resumptionPath', 'localExhaustionSummary'].includes(label)) fail('invalid bounded Handback detail label');
    return { label, value: boundedString(item.value, 20_000, `boundedDetails[${index}].value`) };
  });
  if (boundedDetails && new Set(boundedDetails.map((detail) => detail.label)).size !== boundedDetails.length)
    fail('duplicate bounded Handback detail label');
  if (movementKind === 'continue_eligible_work') {
    if (x.dependencyOwner !== undefined || x.dependencyOwnerClassification !== undefined || x.enablingResult !== undefined || x.resumptionPath !== undefined || x.localExhaustionSummary !== undefined || boundedDetails !== undefined)
      fail('alternate Handback movement has contradictory detail');
    return { movementKind, rationale, eligibleWorkSummary: boundedString(x.eligibleWorkSummary, 4000, 'eligibleWorkSummary') };
  }
  if (movementKind === 'wait_for_agent_dependency') {
    const classification = string(x.dependencyOwnerClassification, 'dependencyOwnerClassification');
    if (!['work_unit_handler', 'work_unit_implementer', 'work_slice_planner', 'sprint_runner'].includes(classification))
      fail('invalid dependencyOwnerClassification');
    const owner = boundedString(x.dependencyOwner, 4000, 'dependencyOwner');
    if (['human', 'external', 'approval', 'manual', 'user'].some((term) => owner.toLowerCase().includes(term)))
      fail('dependency owner is outside the agent-achievable boundary');
    if (x.eligibleWorkSummary !== undefined || x.localExhaustionSummary !== undefined || boundedDetails !== undefined)
      fail('dependency Handback movement has contradictory detail');
    return {
      movementKind,
      rationale,
      dependencyOwner: owner,
      dependencyOwnerClassification: classification as ProductSprintRunnerHandbackDependencyOwnerClassificationV1,
      enablingResult: boundedString(x.enablingResult, 4000, 'enablingResult'),
      resumptionPath: boundedString(x.resumptionPath, 4000, 'resumptionPath'),
    };
  }
  if (movementKind === 'local_exhaustion_escalate') {
    if (x.eligibleWorkSummary !== undefined || x.dependencyOwner !== undefined || x.dependencyOwnerClassification !== undefined || x.enablingResult !== undefined || x.resumptionPath !== undefined || boundedDetails !== undefined)
      fail('local exhaustion Handback movement has contradictory detail');
    return { movementKind, rationale, localExhaustionSummary: boundedString(x.localExhaustionSummary, 4000, 'localExhaustionSummary') };
  }
  const rawDetailValues = [
    ['eligibleWorkSummary', x.eligibleWorkSummary],
    ['dependencyOwner', x.dependencyOwner],
    ['dependencyOwnerClassification', x.dependencyOwnerClassification],
    ['enablingResult', x.enablingResult],
    ['resumptionPath', x.resumptionPath],
    ['localExhaustionSummary', x.localExhaustionSummary],
  ].filter(([, value]) => value !== undefined);
  if (x.dependencyOwnerClassification !== undefined && !['work_unit_handler', 'work_unit_implementer', 'work_slice_planner', 'sprint_runner'].includes(string(x.dependencyOwnerClassification, 'dependencyOwnerClassification')))
    fail('invalid bounded Handback detail classification');
  if (boundedDetails !== undefined && rawDetailValues.length > 0)
    fail('bounded Handback movement mixes projected and persisted detail shapes');
  const projectedDetails = boundedDetails ?? rawDetailValues.map(([label, value]) => ({ label: label as string, value: boundedString(value, 20_000, label as string) }));
  return {
    movementKind: movementKind as ProductSprintRunnerHandbackUnknownMovementKindV1,
    rationale,
    ...(projectedDetails.length > 0 ? { boundedDetails: projectedDetails } : {}),
  };
};

const sprintRunnerHandbackDelivery = (value: unknown): ProductSprintRunnerHandbackDeliveryV1 => {
  const x = object(value, 'Sprint Runner Handback delivery');
  keys(x, ['deliveryRequestedAt', 'deliveryPersistedAt', 'harnessBoundAt', 'launchRequestedAt', 'launchAcceptedAt', 'providerActivationObservedAt', 'semanticReassessmentRecordedAt', 'selectedMovementKind', 'selectedMovement', 'escalationIntentRecordedAt', 'escalationDeliveryRequestedAt'], 'Sprint Runner Handback delivery');
  const deliveryRequestedAt = timestamp(x.deliveryRequestedAt, 'deliveryRequestedAt');
  const optionalTime = (key: string) => x[key] === undefined ? undefined : timestamp(x[key], key);
  const deliveryPersistedAt = optionalTime('deliveryPersistedAt');
  const harnessBoundAt = optionalTime('harnessBoundAt');
  const launchRequestedAt = optionalTime('launchRequestedAt');
  const launchAcceptedAt = optionalTime('launchAcceptedAt');
  const providerActivationObservedAt = optionalTime('providerActivationObservedAt');
  const semanticReassessmentRecordedAt = optionalTime('semanticReassessmentRecordedAt');
  const escalationIntentRecordedAt = optionalTime('escalationIntentRecordedAt');
  const escalationDeliveryRequestedAt = optionalTime('escalationDeliveryRequestedAt');
  phaseCoherence({ deliveryRequestedAt, deliveryPersistedAt, harnessBoundAt, launchRequestedAt, launchAcceptedAt }, ['deliveryRequestedAt', 'deliveryPersistedAt', 'harnessBoundAt', 'launchRequestedAt', 'launchAcceptedAt'], 'Sprint Runner Handback delivery');
  if (providerActivationObservedAt) {
    if (!launchAcceptedAt) fail('Handback provider observation lacks launch acceptance');
    timestampAtOrAfter(launchAcceptedAt, providerActivationObservedAt, 'Handback provider observation');
  }
  if (semanticReassessmentRecordedAt) {
    if (!launchAcceptedAt) fail('Handback reassessment lacks launch acceptance');
    timestampAtOrAfter(launchAcceptedAt, semanticReassessmentRecordedAt, 'Handback semantic reassessment');
  }
  const selectedMovement = x.selectedMovement === undefined ? undefined : sprintRunnerHandbackMovement(x.selectedMovement);
  const selectedMovementKind = x.selectedMovementKind === undefined ? undefined : boundedString(x.selectedMovementKind, 96, 'selectedMovementKind');
  if ((selectedMovementKind === undefined) !== (selectedMovement === undefined))
    fail('selected Handback movement kind and detail must be paired');
  if (selectedMovement && selectedMovementKind !== selectedMovement.movementKind)
    fail('selected Handback movement kind does not match its detail');
  if (selectedMovementKind && !semanticReassessmentRecordedAt)
    fail('selected Handback movement lacks semantic reassessment');
  if (selectedMovementKind === 'wait_for_agent_dependency' && !selectedMovement)
    fail('dependency movement lacks structured route detail');
  if (escalationIntentRecordedAt || escalationDeliveryRequestedAt) {
    if (!selectedMovement || selectedMovement.movementKind !== 'local_exhaustion_escalate')
      fail('escalation delivery lacks selected local exhaustion movement');
    if (!semanticReassessmentRecordedAt) fail('escalation intent lacks semantic reassessment');
    if (!escalationIntentRecordedAt) fail('escalation delivery request lacks recorded intent');
    timestampAtOrAfter(semanticReassessmentRecordedAt, escalationIntentRecordedAt, 'Handback escalation intent');
    if (escalationDeliveryRequestedAt) {
      timestampAtOrAfter(escalationIntentRecordedAt, escalationDeliveryRequestedAt, 'Handback escalation delivery');
      timestampAtOrAfter(semanticReassessmentRecordedAt, escalationDeliveryRequestedAt, 'Handback escalation delivery');
    }
  }
  return {
    deliveryRequestedAt,
    ...(deliveryPersistedAt ? { deliveryPersistedAt } : {}),
    ...(harnessBoundAt ? { harnessBoundAt } : {}),
    ...(launchRequestedAt ? { launchRequestedAt } : {}),
    ...(launchAcceptedAt ? { launchAcceptedAt } : {}),
    ...(providerActivationObservedAt ? { providerActivationObservedAt } : {}),
    ...(semanticReassessmentRecordedAt ? { semanticReassessmentRecordedAt } : {}),
    ...(selectedMovementKind ? { selectedMovementKind } : {}),
    ...(selectedMovement ? { selectedMovement } : {}),
    ...(escalationIntentRecordedAt ? { escalationIntentRecordedAt } : {}),
    ...(escalationDeliveryRequestedAt ? { escalationDeliveryRequestedAt } : {}),
  };
};
const workUnitRelationship = (value: unknown): NativeWorkUnitRelationshipV1 => {
  const x = object(value, 'Work Unit relationship');
  keys(
    x,
    ['relationshipId', 'materializationId', 'relationshipKind', 'fromId', 'toId', 'ordinal'],
    'Work Unit relationship',
  );
  if (
    !['planning_point', 'sprint', 'lane', 'order', 'depends_on'].includes(
      x.relationshipKind as string,
    )
  )
    fail('invalid Work Unit relationship kind');
  if (x.ordinal !== undefined && (!Number.isSafeInteger(x.ordinal) || (x.ordinal as number) < 0))
    fail('invalid Work Unit relationship ordinal');
  return {
    relationshipId: string(x.relationshipId, 'relationshipId'),
    materializationId: string(x.materializationId, 'materializationId'),
    relationshipKind: x.relationshipKind as NativeWorkUnitRelationshipV1['relationshipKind'],
    fromId: string(x.fromId, 'fromId'),
    toId: string(x.toId, 'toId'),
    ...(x.ordinal === undefined ? {} : { ordinal: x.ordinal as number }),
  };
};
const fileReviewDocument = (value: unknown): NativeFileReviewDocumentV1 => {
  const x = object(value, 'file review document');
  keys(
    x,
    [
      'documentRefId',
      'epicId',
      'sprintId',
      'provenanceId',
      'title',
      'summary',
      'artifactId',
      'changedFiles',
    ],
    'file review document',
  );
  return {
    documentRefId: string(x.documentRefId, 'documentRefId'),
    epicId: string(x.epicId, 'epicId'),
    sprintId: string(x.sprintId, 'sprintId'),
    provenanceId: string(x.provenanceId, 'provenanceId'),
    title: boundedString(x.title, 240, 'Document title'),
    ...(x.summary === undefined
      ? {}
      : { summary: boundedString(x.summary, 4000, 'Document summary') }),
    artifactId: string(x.artifactId, 'artifactId'),
    changedFiles: array(x.changedFiles, 'changedFiles').map((value) => {
      const file = object(value, 'changed file');
      keys(
        file,
        ['changedFileReferenceId', 'displayName', 'changeKind', 'previousDisplayName'],
        'changed file',
      );
      if (!['added', 'modified', 'deleted', 'renamed'].includes(file.changeKind as string))
        fail('invalid changed file kind');
      const changeKind = file.changeKind as 'added' | 'modified' | 'deleted' | 'renamed';
      const previousDisplayName =
        file.previousDisplayName === undefined
          ? undefined
          : boundedString(file.previousDisplayName, 4000, 'previousDisplayName');
      if ((changeKind === 'renamed') !== (previousDisplayName !== undefined))
        fail('invalid renamed previous display name');
      return {
        changedFileReferenceId: string(file.changedFileReferenceId, 'changedFileReferenceId'),
        displayName: boundedString(file.displayName, 4000, 'displayName'),
        changeKind,
        ...(previousDisplayName === undefined ? {} : { previousDisplayName }),
      };
    }),
  };
};
function validate(query: OrchestrationNativeQueryV2) {
  unique(query.planningDrafts, (x) => x.epicPlanningDraftId, 'planning draft ID');
  unique(query.agentSessionAssociations, (x) => x.agentSessionAssociationId, 'association ID');
  unique(query.proposalRevisions, (x) => x.proposalRevisionId, 'proposal revision ID');
  unique(query.recordedProposalEvents, (x) => x.proposalEventId, 'proposal event ID');
  unique(query.provenanceLinks, (x) => x.provenanceId, 'provenance ID');
  unique(query.initiationCommands, (x) => x.commandId, 'initiation command ID');
  unique(query.initiationResults, (x) => x.resultId, 'initiation result ID');
  unique(query.initiationEvents, (x) => x.eventId, 'initiation event ID');
  unique(query.initiationProvenance, (x) => x.provenanceId, 'initiation provenance ID');
  unique(query.materialSnapshots, (x) => x.materialSnapshotId, 'material snapshot ID');
  unique(query.initiatedEpics, (x) => x.initiationId, 'initiation ID');
  unique(query.initiatedEpics, (x) => x.epicId, 'initiated Epic ID');
  unique(query.initiatedEpics, (x) => x.epicPlanningDraftId, 'initiated Epic planning draft ID');
  unique(query.initiatedSprints, (x) => x.sprintId, 'initiated Sprint ID');
  const drafts = new Set(query.planningDrafts.map((x) => x.epicPlanningDraftId));
  const revisions = new Map(query.proposalRevisions.map((x) => [x.proposalRevisionId, x]));
  const associations = new Map(
    query.agentSessionAssociations.map((x) => [x.agentSessionAssociationId, x]),
  );
  const provenance = new Map(query.provenanceLinks.map((x) => [x.provenanceId, x]));
  const initiationCommands = new Map(query.initiationCommands.map((x) => [x.commandId, x]));
  const initiationResults = new Map(query.initiationResults.map((x) => [x.resultId, x]));
  const initiationEvents = new Map(query.initiationEvents.map((x) => [x.eventId, x]));
  const initiationProvenance = new Map(query.initiationProvenance.map((x) => [x.provenanceId, x]));
  const snapshots = new Map(query.materialSnapshots.map((x) => [x.materialSnapshotId, x]));
  query.planningDrafts.forEach((x) => {
    if (
      x.currentProposal.status === 'available' &&
      revisions.get(x.currentProposal.proposalRevisionId)?.epicPlanningDraftId !==
        x.epicPlanningDraftId
    )
      fail('current proposal does not belong to draft');
  });
  query.proposalRevisions.forEach((x) => {
    if (!drafts.has(x.epicPlanningDraftId)) fail('proposal revision references unknown draft');
    if (
      x.parentProposalRevisionId &&
      revisions.get(x.parentProposalRevisionId)?.epicPlanningDraftId !== x.epicPlanningDraftId
    )
      fail('proposal parent does not belong to draft');
    const link = provenance.get(x.provenanceId);
    if (!link || link.causalCommandId !== x.commandId)
      fail('proposal revision provenance does not name its causal command');
    if (
      associations.get(link.agentSessionAssociationId)?.epicPlanningDraftId !==
      x.epicPlanningDraftId
    )
      fail('proposal revision provenance is not associated with its draft');
  });
  query.agentSessionAssociations.forEach((x) => {
    if (!drafts.has(x.epicPlanningDraftId)) fail('association references unknown draft');
  });
  query.recordedProposalEvents.forEach((x) => {
    const revision = revisions.get(x.proposalRevisionId);
    if (
      !revision ||
      !drafts.has(x.epicPlanningDraftId) ||
      revision.epicPlanningDraftId !== x.epicPlanningDraftId
    )
      fail('proposal event references unknown draft or revision');
    if (revision.commandId !== x.commandId || revision.provenanceId !== x.provenanceId)
      fail('proposal event does not match its revision command or provenance');
    if (provenance.get(x.provenanceId)?.causalCommandId !== x.commandId)
      fail('proposal event provenance does not name its causal command');
  });
  query.provenanceLinks.forEach((x) => {
    if (!associations.has(x.agentSessionAssociationId))
      fail('provenance references unknown association');
  });
  query.proposalRevisions.forEach((x) => validateProposal(x.proposal));
  query.initiationResults.forEach((result) => {
    if (!initiationCommands.has(result.commandId))
      fail('initiation result references unknown command');
  });
  query.initiationEvents.forEach((event) => {
    if (initiationResults.get(event.resultId)?.commandId !== event.commandId)
      fail('initiation event does not match its result command');
  });
  query.initiationProvenance.forEach((item) => {
    const event = initiationEvents.get(item.eventId);
    if (!event || event.commandId !== item.commandId || event.resultId !== item.resultId)
      fail('initiation provenance does not match command result and event');
  });
  query.materialSnapshots.forEach((snapshot) => {
    const revision = revisions.get(snapshot.proposalRevisionId);
    if (!revision || revision.epicPlanningDraftId !== snapshot.epicPlanningDraftId)
      fail('material snapshot has invalid proposal correlation');
    validateProposal(snapshot.proposal);
    if (JSON.stringify(snapshot.proposal) !== JSON.stringify(revision.proposal))
      fail('material snapshot differs from consumed proposal revision');
  });
  query.initiatedEpics.forEach((epic) => {
    const command = initiationCommands.get(epic.commandId);
    const result = initiationResults.get(epic.resultId);
    const event = initiationEvents.get(epic.eventId);
    const itemProvenance = initiationProvenance.get(epic.provenanceId);
    const snapshot = snapshots.get(epic.materialSnapshotId);
    const revision = revisions.get(epic.proposalRevisionId);
    if (
      !command ||
      !result ||
      !event ||
      !itemProvenance ||
      !snapshot ||
      !revision ||
      !drafts.has(epic.epicPlanningDraftId)
    )
      fail('initiated Epic references a missing initiation fact');
    if (
      command.epicPlanningDraftId !== epic.epicPlanningDraftId ||
      command.expectedRevisionToken !== revision.revisionToken ||
      result.commandId !== epic.commandId ||
      event.commandId !== epic.commandId ||
      event.resultId !== epic.resultId ||
      itemProvenance.commandId !== epic.commandId ||
      itemProvenance.resultId !== epic.resultId ||
      itemProvenance.eventId !== epic.eventId ||
      snapshot.epicPlanningDraftId !== epic.epicPlanningDraftId ||
      snapshot.proposalRevisionId !== epic.proposalRevisionId ||
      revision.epicPlanningDraftId !== epic.epicPlanningDraftId
    )
      fail('initiated Epic has invalid command result event provenance or snapshot correlation');
    const draft = query.planningDrafts.find(
      (item) => item.epicPlanningDraftId === epic.epicPlanningDraftId,
    );
    if (
      draft?.currentProposal.status !== 'available' ||
      draft.currentProposal.proposalRevisionId !== epic.proposalRevisionId
    )
      fail('initiated Epic does not consume its draft current proposal');
  });
  query.planningDrafts.forEach((draft) => {
    const hasInitiation = query.initiatedEpics.some(
      (epic) => epic.epicPlanningDraftId === draft.epicPlanningDraftId,
    );
    if ((draft.status === 'initiated') !== hasInitiation)
      fail('planning draft initiation status does not match durable initiation facts');
  });
  query.initiatedEpics.forEach((epic) => {
    const snapshot = snapshots.get(epic.materialSnapshotId)!;
    const sprints = query.initiatedSprints.filter((sprint) => sprint.epicId === epic.epicId);
    if (
      sprints.length !== snapshot.proposal.sprints.length ||
      sprints.some(
        (sprint, ordinal) =>
          sprint.ordinal !== ordinal ||
          JSON.stringify({
            title: sprint.title,
            intendedMovement: sprint.intendedMovement,
            concernSummaries: sprint.concernSummaries,
          }) !== JSON.stringify(snapshot.proposal.sprints[ordinal]),
      )
    )
      fail('initiated Sprints do not exactly match the ordered material snapshot');
  });
  query.initiatedSprints.forEach((sprint) => {
    if (!query.initiatedEpics.some((epic) => epic.epicId === sprint.epicId))
      fail('initiated Sprint references unknown Epic');
  });
  const initiated = query.initiatedEpics;
  requireConsumed(
    query.initiationCommands,
    initiated.map((epic) => epic.commandId),
    (item) => item.commandId,
    'initiation command',
  );
  requireConsumed(
    query.initiationResults,
    initiated.map((epic) => epic.resultId),
    (item) => item.resultId,
    'initiation result',
  );
  requireConsumed(
    query.initiationEvents,
    initiated.map((epic) => epic.eventId),
    (item) => item.eventId,
    'initiation event',
  );
  requireConsumed(
    query.initiationProvenance,
    initiated.map((epic) => epic.provenanceId),
    (item) => item.provenanceId,
    'initiation provenance',
  );
  requireConsumed(
    query.materialSnapshots,
    initiated.map((epic) => epic.materialSnapshotId),
    (item) => item.materialSnapshotId,
    'initiation material snapshot',
  );
  unique(
    query.workUnitMaterializations,
    (x) => x.materializationId,
    'Work Unit materialization ID',
  );
  unique(
    query.workUnitMaterializations,
    (x) => x.planningPointId,
    'Work Unit materialization planning point',
  );
  unique(query.workUnits, (x) => x.workUnitId, 'materialized Work Unit ID');
  unique(query.workUnitExecutionStates, (x) => x.workUnitId, 'Work Unit execution state Work Unit ID');
  unique(query.workSliceExecutionGraphCompletions, (x) => x.materializationId, 'Work Slice graph completion materialization ID');
  unique(query.workSliceExecutionSettlements, (x) => x.materializationId, 'Work Slice execution settlement materialization ID');
  unique(query.workSlicePlanningPointExecutionSettlements, (x) => x.planningPointId, 'Work Slice planning-point execution settlement planning point ID');
  unique(query.workSliceExecutionAttentions, (x) => x.materializationId, 'Work Slice execution attention materialization ID');
  unique(query.workUnitRelationships, (x) => x.relationshipId, 'Work Unit relationship ID');
  unique(query.dependencyActivationIntents, (x) => x.workUnitId, 'dependency activation intent Work Unit ID');
  const materializations = new Map(
    query.workUnitMaterializations.map((x) => [x.materializationId, x]),
  );
  const unitsById = new Map(query.workUnits.map((unit) => [unit.workUnitId, unit]));
  const stateByUnitId = new Map(query.workUnitExecutionStates.map((state) => [state.workUnitId, state]));
  const completionByMaterializationId = new Map(query.workSliceExecutionGraphCompletions.map((item) => [item.materializationId, item]));
  const settlementByMaterializationId = new Map(query.workSliceExecutionSettlements.map((item) => [item.materializationId, item]));
  const attentionByMaterializationId = new Map(query.workSliceExecutionAttentions.map((item) => [item.materializationId, item]));
  query.dependencyActivationIntents.forEach((intent) => {
    const unit = unitsById.get(intent.workUnitId);
    const materialization = materializations.get(intent.materializationId);
    if (!unit || !materialization || unit.materializationId !== intent.materializationId || unit.acceptedRevisionId !== intent.acceptedRevisionId || materialization.acceptedRevisionId !== intent.acceptedRevisionId)
      fail('dependency activation intent has invalid Work Unit/materialization/revision correlation');
  });
  query.workUnitMaterializations.forEach((materialization) => {
    if (!query.initiatedEpics.some((epic) => epic.epicId === materialization.epicId))
      fail('Work Unit materialization references unknown Epic');
    if (
      !query.initiatedSprints.some(
        (sprint) =>
          sprint.sprintId === materialization.sprintId && sprint.epicId === materialization.epicId,
      )
    )
      fail('Work Unit materialization references unknown Sprint');
    if (materialization.attemptRecordedAt && !materialization.authorizationRecordedAt)
      fail('Work Unit materialization attempt requires authorization');
    if (materialization.workUnitsCreatedAt && !materialization.attemptRecordedAt)
      fail('Work Unit materialization Work Units require an attempt');
    if (materialization.relationshipsCompletedAt && !materialization.workUnitsCreatedAt)
      fail('Work Unit materialization relationships require Work Units');
    if (materialization.settledAt && !materialization.relationshipsCompletedAt)
      fail('Work Unit materialization settlement requires relationships');
    const units = query.workUnits.filter(
      (unit) => unit.materializationId === materialization.materializationId,
    );
    const relationships = query.workUnitRelationships.filter(
      (item) => item.materializationId === materialization.materializationId,
    );
    if (!materialization.workUnitsCreatedAt && units.length)
      fail('Work Units exist before their materialization stage');
    if (!materialization.relationshipsCompletedAt && relationships.length)
      fail('relationships exist before their materialization stage');
    if (materialization.workUnitsCreatedAt && !units.length)
      fail('Work Unit materialization created stage has no Work Units');
    if (materialization.relationshipsCompletedAt)
      validateMaterializationRelationships(materialization, units, relationships);
    units.forEach((unit) => {
      if (
        unit.workSliceId !== materialization.workSliceId ||
        unit.acceptedRevisionId !== materialization.acceptedRevisionId
      )
        fail('materialized Work Unit does not match its materialization');
      validateActivationCorrelations(unit);
    });
    if (query.workUnitExecutionStates.length && units.some((unit) => !stateByUnitId.has(unit.workUnitId))) fail('productive execution state is incomplete');
    const completion = completionByMaterializationId.get(materialization.materializationId);
    const attention = attentionByMaterializationId.get(materialization.materializationId);
    if (completion) {
      if (completion.acceptedRevisionId !== materialization.acceptedRevisionId || attention || units.some((unit) => stateByUnitId.get(unit.workUnitId)?.state !== 'settled')) fail('Work Slice graph completion is incoherent');
      const settlement = settlementByMaterializationId.get(materialization.materializationId);
      if (settlement && (settlement.graphCompletionMaterializationId !== materialization.materializationId || Date.parse(settlement.settledAt) < Date.parse(completion.completedAt))) fail('Work Slice execution settlement is incoherent');
    } else if (settlementByMaterializationId.has(materialization.materializationId)) fail('Work Slice execution settlement lacks graph completion');
  });
  query.workUnitExecutionStates.forEach((state) => { const unit = unitsById.get(state.workUnitId); if (!unit || unit.materializationId !== state.materializationId || unit.acceptedRevisionId !== state.acceptedRevisionId) fail('Work Unit execution state has invalid correlation'); });
  query.workSliceExecutionAttentions.forEach((attention) => { if (!materializations.has(attention.materializationId)) fail('Work Slice execution attention references unknown materialization'); });
  query.workSlicePlanningPointExecutionSettlements.forEach((item) => { const materialization = materializations.get(item.materializationId); const settlement = settlementByMaterializationId.get(item.materializationId); if (!materialization || item.planningPointId !== materialization.planningPointId || item.workSliceExecutionMaterializationId !== item.materializationId || !settlement || Date.parse(item.settledAt) < Date.parse(settlement.settledAt)) fail('Work Slice planning-point execution settlement is incoherent'); });
  if (query.workUnits.some((unit) => !materializations.has(unit.materializationId)))
    fail('materialized Work Unit references unknown materialization');
  if (query.workUnitRelationships.some((item) => !materializations.has(item.materializationId)))
    fail('Work Unit relationship references unknown materialization');
}
function validateActivationCorrelations(unit: NativeMaterializedWorkUnitV1) {
  const handler = unit.handlerActivation;
  const continuation = unit.actionContinuation;
  const implementer = unit.implementerActivation;
  const history = unit.attemptHistory;
  const originHistory = history.find((member) => member.ordinal === 0);
  const outcome = originHistory?.implementerOutcome;
  const retries = unit.retryAttempts;
  if (unit.integration) {
    const terminal = history.at(-1);
    if (!terminal?.handlerDecision || terminal.handlerDecision.variant !== 'accepted' || terminal.incompleteDisposition)
      fail('productive integration requires the final attempt to be accepted');
  }
  if (continuation) {
    if (!handler || handler.attemptId !== continuation.attemptId)
      fail('Handler action continuation does not share the Handler attempt');
    if (handler.eligibilityState !== 'eligible')
      fail('Handler action continuation requires an eligible Handler activation');
    if (
      handler.handlerSessionId !== continuation.handlerSessionId ||
      handler.handlerInvocationId !== continuation.originalHandlerInvocationId
    )
      fail(
        'Handler action continuation does not match the original Handler Session and invocation',
      );
  }
  if (implementer) {
    if (!handler || handler.attemptId !== implementer.attemptId)
      fail('Implementer activation does not share the Handler attempt');
    if (!continuation || continuation.actionInvocationId !== implementer.handlerActionInvocationId)
      fail('Implementer activation does not match the Handler action invocation');
    if (continuation.blockedReason)
      fail('blocked Handler action continuation cannot have an Implementer activation');
  }
  if (outcome) {
    if (
      !implementer ||
      implementer.attemptId !== outcome.attemptId ||
      implementer.implementerSessionId !== outcome.implementerSessionId ||
      implementer.implementerInvocationId !== outcome.originalImplementerInvocationId
    )
      fail('Implementer outcome does not match the exact Implementer attempt and Session');
    if (!implementer.launchAcceptedAt || !implementer.implementerReadyAt)
      fail('Implementer outcome requires application-ready Implementer activation');
    if (
      outcome.reportingInvocationId === outcome.originalImplementerInvocationId ||
      outcome.reportingInvocationId === continuation?.actionInvocationId ||
      outcome.reportingInvocationId === continuation?.originalHandlerInvocationId
    )
      fail('Implementer outcome reuses an earlier invocation identity');
    if (outcome.reportingHarnessRevisionId === implementer.implementerHarnessRevisionId)
      fail('Implementer outcome reuses the original Implementer Harness revision');
  }
  const retryOrdinals = new Set<number>();
  const retryAttemptIds = new Set<string>();
  for (const retry of retries) {
    if (!retryOrdinals.add(retry.ordinal) || !retryAttemptIds.add(retry.retryAttemptId))
      fail('retry attempts must have unique ordinals and identities');
    const origin = history.find((member) => member.attemptId === retry.originAttemptId);
    const originOutcome = origin?.implementerOutcome;
    const predecessor = history.find((member) => member.ordinal === retry.ordinal - 1);
    const disposition = predecessor?.incompleteDisposition;
    const legacyOrdinalOne =
      origin?.ordinal === 0 &&
      retry.ordinal === 1 &&
      origin.incompleteDisposition === undefined &&
      origin.handlerDecision?.variant === 'returned' &&
      origin.handlerDecision.retryRequiredAt !== undefined;
    if (!origin || !predecessor || predecessor.attemptId !== origin.attemptId || !originOutcome || retry.ordinal !== origin.ordinal + 1)
      fail('retry attempt does not match its exact predecessor history member');
    if (!legacyOrdinalOne && (!disposition || !disposition.meaningfulProgress || !disposition.nextAttemptAuthorizedAt))
      fail('retry attempt requires a returned Handler decision or exact meaningful-progress authorization from its predecessor');
    if (
      retry.implementerSessionId === originOutcome.implementerSessionId ||
      retry.implementerInvocationId === originOutcome.originalImplementerInvocationId
    )
      fail('retry attempt must use distinct Implementer Session and invocation identities');
    timestampAtOrAfter(
      legacyOrdinalOne
        ? origin.handlerDecision!.retryRequiredAt!
        : disposition!.nextAttemptAuthorizedAt!,
      retry.captureRequestedAt,
      'retry capture request',
    );
    phaseCoherence(
      retry,
      [
        'captureRequestedAt', 'candidatePinnedAt', 'authorizedAt',
        'executionSupportGrantedAt', 'isolatedWorktreeReadyAt', 'implementerSessionCreatedAt',
        'implementerInvocationPreparedAt', 'implementerHarnessBoundAt', 'launchRequestedAt',
        'launchAcceptedAt', 'retryReadyAt',
      ],
      'retry Implementer activation',
    );
    if (retry.providerActivationObservedAt) {
      if (!retry.launchRequestedAt)
        fail('retry provider observation lacks launch request');
      timestampAtOrAfter(retry.launchRequestedAt, retry.providerActivationObservedAt, 'retry provider observation');
      if (retry.launchAcceptedAt)
        timestampAtOrAfter(retry.launchAcceptedAt, retry.providerActivationObservedAt, 'retry provider observation');
    }
    if (retry.retryReadyAt && !retry.launchAcceptedAt)
      fail('retry readiness lacks launch acceptance');
    if (retry.failureReason && retry.retryReadyAt)
      fail('retry failure cannot be application-ready');
    const retryHistory = history.find((member) => member.ordinal === retry.ordinal);
    if (retryHistory && retryHistory.attemptId !== retry.retryAttemptId)
      fail('retry activation does not match its attempt-history identity');
    if (
      retryHistory?.implementerOutcome &&
      (retryHistory.implementerOutcome.implementerSessionId !== retry.implementerSessionId ||
        retryHistory.implementerOutcome.originalImplementerInvocationId !== retry.implementerInvocationId)
    )
      fail('retry attempt history does not match its exact Session and invocation');
    if (!retryHistory && history.some((member) => member.attemptId === retry.retryAttemptId))
      fail('retry activation reuses a foreign attempt-history identity');
  }
  let expectedOrdinal = 0;
  const attemptIds = new Set<string>();
  for (const member of history) {
    if (member.ordinal !== expectedOrdinal || attemptIds.has(member.attemptId))
      fail('Work Unit attempt history must be strictly ordered without gaps and use unique identities');
    expectedOrdinal += 1;
    attemptIds.add(member.attemptId);
    const memberOutcome = member.implementerOutcome;
    if (!memberOutcome || memberOutcome.attemptId !== member.attemptId)
      fail('Work Unit attempt history member lacks its exact Implementer outcome');
    const memberReview = member.handlerReview;
    if (memberReview) {
      if (
        memberReview.attemptId !== member.attemptId ||
        memberReview.reportingInvocationId !== memberOutcome.reportingInvocationId ||
        memberReview.handlerSessionId !== handler?.handlerSessionId ||
        memberReview.originalHandlerInvocationId !== handler?.handlerInvocationId ||
        memberReview.actionHandlerInvocationId !== continuation?.actionInvocationId
      )
        fail('Handler review does not match its attempt and application-owned Handler authority');
      if (!handler || handler.eligibilityState !== 'eligible')
        fail('Handler review requires an eligible Handler activation');
      if (!continuation || continuation.blockedReason)
        fail('Handler review requires an unblocked Handler action continuation');
      if (!memberOutcome.submittedOutcome || !memberOutcome.evidence)
        fail('Handler review requires the accepted Implementer claims and evidence');
      if (
        memberReview.delivered.summaryClaim !== memberOutcome.submittedOutcome.summaryClaim ||
        memberReview.delivered.validationStatementClaim !== memberOutcome.submittedOutcome.validationStatementClaim ||
        memberReview.delivered.comparisonFingerprint !== memberOutcome.evidence.comparisonFingerprint ||
        JSON.stringify(memberReview.delivered.changedFiles) !== JSON.stringify(memberOutcome.evidence.changedFiles)
      )
        fail('Handler review delivered evidence differs from the Implementer outcome');
      if (memberReview.semanticJudgment && !memberReview.launchAcceptedAt)
        fail('Handler review judgment requires launch acceptance');
    }
    const memberDecision = member.handlerDecision;
    if (memberDecision) {
      if (
        !memberReview ||
        memberDecision.reviewInvocationId !== memberReview.reviewInvocationId ||
        !memberReview.semanticJudgment ||
        memberReview.lifecycle?.status !== 'completed'
      )
        fail('Handler decision lacks exact completed review correlation');
      const expectedVariant = memberReview.semanticJudgment.variant === 'accept' ? 'accepted' : 'returned';
      if (memberDecision.variant !== expectedVariant)
        fail('Handler decision contradicts semantic judgment');
      if (memberDecision.variant === 'returned') {
        if (JSON.stringify(memberDecision.returnReason) !== JSON.stringify(memberReview.semanticJudgment.reason))
          fail('Handler decision return reason differs from semantic judgment');
      } else if (memberDecision.returnReason) {
        fail('accepted Handler decision cannot have a return reason');
      }
      timestampAtOrAfter(memberReview.lifecycle.observedAt, memberDecision.recordedAt, 'Handler decision');
    }
    const memberDisposition = member.incompleteDisposition;
    if (memberDisposition) {
      if (!memberReview || !memberDecision || memberDecision.variant !== 'returned')
        fail('incomplete disposition requires a returned Handler decision');
      if (
        memberDisposition.attemptId !== member.attemptId ||
        memberDisposition.reviewInvocationId !== memberReview.reviewInvocationId ||
        memberDisposition.decisionFingerprint !== memberDecision.fingerprint
      )
        fail('incomplete disposition does not match its attempt, review, and decision');
      const handback = memberDisposition.noProgressHandback;
      if (memberDisposition.meaningfulProgress) {
        if (!memberDisposition.nextAttemptAuthorizedAt || handback)
          fail('meaningful-progress disposition has incoherent later effects');
      } else {
        if (memberDisposition.nextAttemptAuthorizedAt || !handback)
          fail('no-progress disposition has incoherent authorization or handback');
        if (
          handback.sourceAttemptId !== member.attemptId ||
          handback.sourceReviewInvocationId !== memberReview.reviewInvocationId ||
          handback.sprintRunnerReceiverActivatedAt !== undefined ||
          handback.sprintRunnerReceiverDecisionAt !== undefined
        )
          fail('no-progress handback has foreign or forbidden receiver effects');
      }
    }
  }
}
function validateMaterializationRelationships(
  materialization: NativeWorkUnitMaterializationV1,
  units: readonly NativeMaterializedWorkUnitV1[],
  relationships: readonly NativeWorkUnitRelationshipV1[],
) {
  unique(
    units,
    (unit) => `${unit.materializationId}:${unit.laneOrdinal}`,
    'materialized Work Unit lane ordinal',
  );
  unique(
    relationships,
    (relationship) =>
      `${relationship.materializationId}:${relationship.relationshipKind}:${relationship.fromId}:${relationship.toId}`,
    'Work Unit relationship',
  );
  const unitIds = new Set(units.map((unit) => unit.workUnitId));
  const exact = (
    kind: NativeWorkUnitRelationshipV1['relationshipKind'],
    fromId: string,
    toId: string,
    ordinal?: number,
  ) =>
    relationships.filter(
      (item) =>
        item.relationshipKind === kind &&
        item.fromId === fromId &&
        item.toId === toId &&
        item.ordinal === ordinal,
    );
  if (
    exact('planning_point', materialization.planningPointId, materialization.workSliceId).length !==
    1
  )
    fail('materialization requires exactly one planning-point relationship');
  if (exact('sprint', materialization.sprintId, materialization.workSliceId).length !== 1)
    fail('materialization requires exactly one Sprint relationship');
  for (const unit of units) {
    if (
      exact('lane', materialization.workSliceId, unit.workUnitId, unit.laneOrdinal).length !== 1 ||
      exact('order', materialization.workSliceId, unit.workUnitId, unit.laneOrdinal).length !== 1
    )
      fail('materialization Work Unit requires matching lane and order relationships');
  }
  for (const relationship of relationships) {
    if (
      (relationship.relationshipKind === 'lane' || relationship.relationshipKind === 'order') &&
      (!unitIds.has(relationship.toId) || relationship.fromId !== materialization.workSliceId)
    )
      fail('materialization lane/order relationship is incoherent');
    if (
      relationship.relationshipKind === 'depends_on' &&
      (!unitIds.has(relationship.fromId) ||
        !unitIds.has(relationship.toId) ||
        relationship.fromId === relationship.toId ||
        relationship.ordinal !== undefined)
    )
      fail('materialization dependency relationship is incoherent');
    if (
      (relationship.relationshipKind === 'planning_point' ||
        relationship.relationshipKind === 'sprint') &&
      relationship.ordinal !== undefined
    )
      fail('materialization ownership relationship cannot have an ordinal');
    if (
      relationship.relationshipKind === 'planning_point' &&
      (relationship.fromId !== materialization.planningPointId ||
        relationship.toId !== materialization.workSliceId)
    )
      fail('materialization planning-point relationship is incoherent');
    if (
      relationship.relationshipKind === 'sprint' &&
      (relationship.fromId !== materialization.sprintId ||
        relationship.toId !== materialization.workSliceId)
    )
      fail('materialization Sprint relationship is incoherent');
  }
}
function requireConsumed<T>(
  items: readonly T[],
  consumedIds: readonly string[],
  id: (item: T) => string,
  label: string,
) {
  const consumed = new Set(consumedIds);
  if (consumed.size !== consumedIds.length || items.length !== consumed.size)
    fail(`${label} does not contribute to exactly one initiated Epic`);
  if (items.some((item) => !consumed.has(id(item))))
    fail(`${label} does not contribute to an initiated Epic`);
}
function validateProposal(proposal: NativeProposalRevisionV1['proposal']) {
  if (proposal.sprints.length === 0 || proposal.sprints.length > 20)
    fail('proposal must contain 1 to 20 Sprints');
  if (proposal.suggestedEpicName !== undefined)
    boundedText(proposal.suggestedEpicName, 240, 'suggestedEpicName');
  proposal.sprints.forEach((sprint) => {
    boundedText(sprint.title, 240, 'Sprint title');
    boundedText(sprint.intendedMovement, 4_000, 'Sprint intended movement');
    if (sprint.concernSummaries.length > 20) fail('Sprint has too many concern summaries');
    sprint.concernSummaries.forEach((summary) => boundedText(summary, 2_000, 'concern summary'));
  });
}
function boundedText(value: string, maxBytes: number, label: string) {
  if (value.trim().length === 0 || new TextEncoder().encode(value).length > maxBytes)
    fail(`${label} must be non-blank and within its size limit`);
}
function boundedString(value: unknown, maxBytes: number, label: string): string {
  const result = string(value, label);
  boundedText(result, maxBytes, label);
  return result;
}
function unique<T>(items: readonly T[], id: (item: T) => string, label: string) {
  if (new Set(items.map(id)).size !== items.length) fail(`duplicate ${label}`);
}
function object(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    fail(`${label} must be an object`);
  return value as Record<string, unknown>;
}
function array(value: unknown, label: string): readonly unknown[] {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}
function string(value: unknown, label: string): string {
  if (typeof value !== 'string' || value.length === 0) fail(`${label} must be a non-empty string`);
  return value;
}
function timestamp(value: unknown, label: string): string {
  const result = boundedString(value, 80, label);
  if (!/^\d{4}-\d{2}-\d{2}T/.test(result) || Number.isNaN(Date.parse(result)))
    fail(`${label} must be an ISO timestamp`);
  return result;
}
function phaseCoherence(value: object, keys: readonly string[], label: string) {
  let previous: number | undefined;
  let priorPresent = true;
  for (const [index, key] of keys.entries()) {
    const current = (value as Record<string, unknown>)[key];
    if (current === undefined) {
      priorPresent = false;
      continue;
    }
    if (index > 0 && !priorPresent) fail(`${label} has a phase without its prerequisite`);
    const parsed = Date.parse(current as string);
    if (previous !== undefined && parsed < previous)
      fail(`${label} phase timestamps are not ordered`);
    previous = parsed;
  }
}
function timestampAtOrAfter(prerequisite: string, value: string, label: string) {
  if (Date.parse(value) < Date.parse(prerequisite)) fail(`${label} precedes its prerequisite`);
}
function keys(value: Record<string, unknown>, allowed: readonly string[], label: string) {
  if (Object.keys(value).some((key) => !allowed.includes(key)))
    fail(`${label} contains an unknown field`);
}
function fail(message: string): never {
  throw new Error(`Invalid orchestration native query: ${message}`);
}
