import type { EpicPlanProposalSnapshot } from './epicPlanProposal';
import { AGENT_CONTROL_CONTRACTS_V1 } from './agentControl';
import { ARTIFACT_ACCESS_CONTRACTS_V1 } from './artifactAccess';
import { ORCHESTRATION_EVENTS_V1 } from './orchestrationEvents';
import type {
  ProductReadCompositionInputV1,
  ProductWorkUnitActionContinuationV1,
  ProductWorkUnitHandlerActivationV1,
  ProductWorkUnitImplementerActivationV1,
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
}
export interface NativeWorkUnitHandlerActivationV1 {
  readonly attemptId: string;
  readonly handlerSessionId?: string;
  readonly handlerInvocationId?: string;
  readonly handlerHarnessRevisionId?: string;
  readonly handlerHarnessConfigurationDigest?: string;
  readonly handlerHarnessRepositoryCommitRef?: string;
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
}
export interface NativeWorkUnitHandlerActionContinuationV1 {
  readonly attemptId: string;
  readonly handlerSessionId: string;
  readonly originalHandlerInvocationId: string;
  readonly actionInvocationId: string;
  readonly actionHarnessRevisionId: string;
  readonly actionHarnessConfigurationDigest: string;
  readonly actionHarnessRepositoryCommitRef: string;
  readonly requestedAt: string;
  readonly authorizedAt?: string;
  readonly invocationPreparedAt?: string;
  readonly harnessBoundAt?: string;
  readonly launchRequestedAt?: string;
  readonly launchAcceptedAt?: string;
  readonly providerActivationObservedAt?: string;
  readonly actionReadyAt?: string;
  readonly blockedReason?: string;
}
export interface NativeWorkUnitImplementerActivationV1 {
  readonly attemptId: string;
  readonly handlerActionInvocationId: string;
  readonly implementerSessionId: string;
  readonly implementerInvocationId: string;
  readonly implementerHarnessRevisionId: string;
  readonly implementerHarnessConfigurationDigest: string;
  readonly implementerHarnessRepositoryCommitRef: string;
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
    ],
    'native query',
  );
  if (root.contractVersion !== ORCHESTRATION_NATIVE_QUERY_V2) fail('unsupported contractVersion');
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
  const settledMaterializations = query.workUnitMaterializations.filter(
    (materialization) => materialization.settledAt !== undefined,
  );
  const materializedUnits = settledMaterializations.flatMap((materialization) =>
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
    workSlicePlanningPoints: settledMaterializations.map((materialization) => ({
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
      workSlicePlanningPoints: settledMaterializations.map((materialization) => ({
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
        ...(unit.handlerActivation
          ? { handlerActivation: handlerActivationPresentation(unit.handlerActivation) }
          : {}),
        ...(unit.actionContinuation
          ? { actionContinuation: actionContinuationPresentation(unit.actionContinuation) }
          : {}),
        ...(unit.implementerActivation
          ? { implementerActivation: implementerActivationPresentation(unit.implementerActivation) }
          : {}),
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
        workSlicePlanningPointMembership: settledMaterializations.map((materialization) => ({
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
      : activation.launchAcceptedAt
        ? 'launch_accepted'
        : activation.launchRequestedAt
          ? 'launch_requested'
          : activation.handlerInvocationPreparedAt
            ? 'invocation_prepared'
            : 'eligible_not_prepared',
    providerActivityObserved: Boolean(activation.providerActivationObservedAt),
  };
}

function actionContinuationPresentation(
  continuation: NativeWorkUnitHandlerActionContinuationV1,
): ProductWorkUnitActionContinuationV1 {
  return {
    stage: continuation.blockedReason
      ? 'blocked'
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
      'handlerHarnessConfigurationDigest',
      'handlerHarnessRepositoryCommitRef',
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
    ...(optional('handlerHarnessConfigurationDigest')
      ? { handlerHarnessConfigurationDigest: optional('handlerHarnessConfigurationDigest') }
      : {}),
    ...(optional('handlerHarnessRepositoryCommitRef')
      ? { handlerHarnessRepositoryCommitRef: optional('handlerHarnessRepositoryCommitRef') }
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
  };
  if (result.handlerReadyAt && !result.launchAcceptedAt)
    fail('Handler readiness requires launch acceptance');
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
      'actionHarnessConfigurationDigest',
      'actionHarnessRepositoryCommitRef',
      'requestedAt',
      'authorizedAt',
      'invocationPreparedAt',
      'harnessBoundAt',
      'launchRequestedAt',
      'launchAcceptedAt',
      'providerActivationObservedAt',
      'actionReadyAt',
      'blockedReason',
    ],
    'Work Unit Handler action continuation',
  );
  const optionalTime = (key: keyof typeof x) =>
    x[key] === undefined ? undefined : timestamp(x[key], key);
  const blockedReason =
    x.blockedReason === undefined
      ? undefined
      : boundedString(x.blockedReason, 4000, 'blockedReason');
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
    actionHarnessConfigurationDigest: boundedString(
      x.actionHarnessConfigurationDigest,
      240,
      'actionHarnessConfigurationDigest',
    ),
    actionHarnessRepositoryCommitRef: boundedString(
      x.actionHarnessRepositoryCommitRef,
      240,
      'actionHarnessRepositoryCommitRef',
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
  if (result.blockedReason && (result.authorizedAt || result.invocationPreparedAt))
    fail('blocked Handler action continuation cannot have authorized action phases');
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
      'implementerHarnessConfigurationDigest',
      'implementerHarnessRepositoryCommitRef',
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
    implementerHarnessConfigurationDigest: boundedString(
      x.implementerHarnessConfigurationDigest,
      240,
      'implementerHarnessConfigurationDigest',
    ),
    implementerHarnessRepositoryCommitRef: boundedString(
      x.implementerHarnessRepositoryCommitRef,
      240,
      'implementerHarnessRepositoryCommitRef',
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
  if (result.implementerReadyAt && !result.launchAcceptedAt)
    fail('Implementer readiness requires launch acceptance');
  if (result.failureReason && result.implementerReadyAt)
    fail('failed Implementer activation cannot be application-ready');
  return result;
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
  unique(query.workUnitRelationships, (x) => x.relationshipId, 'Work Unit relationship ID');
  const materializations = new Map(
    query.workUnitMaterializations.map((x) => [x.materializationId, x]),
  );
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
  });
  if (query.workUnits.some((unit) => !materializations.has(unit.materializationId)))
    fail('materialized Work Unit references unknown materialization');
  if (query.workUnitRelationships.some((item) => !materializations.has(item.materializationId)))
    fail('Work Unit relationship references unknown materialization');
}
function validateActivationCorrelations(unit: NativeMaterializedWorkUnitV1) {
  const handler = unit.handlerActivation;
  const continuation = unit.actionContinuation;
  const implementer = unit.implementerActivation;
  if (continuation) {
    if (!handler || handler.attemptId !== continuation.attemptId)
      fail('Handler action continuation does not share the Handler attempt');
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
function keys(value: Record<string, unknown>, allowed: readonly string[], label: string) {
  if (Object.keys(value).some((key) => !allowed.includes(key)))
    fail(`${label} contains an unknown field`);
}
function fail(message: string): never {
  throw new Error(`Invalid orchestration native query: ${message}`);
}
