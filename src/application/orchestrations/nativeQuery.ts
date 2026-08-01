import type { EpicPlanProposalSnapshot } from './epicPlanProposal';
import { AGENT_CONTROL_CONTRACTS_V1 } from './agentControl';
import { ARTIFACT_ACCESS_CONTRACTS_V1 } from './artifactAccess';
import { ORCHESTRATION_EVENTS_V1 } from './orchestrationEvents';
import type { ProductReadCompositionInputV1 } from './productReadModels';

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
}
export interface NativeFileReviewDocumentV1 { readonly documentRefId: string; readonly epicId: string; readonly sprintId: string; readonly provenanceId: string; readonly title: string; readonly summary?: string; readonly artifactId: string; readonly changedFiles: readonly { readonly changedFileReferenceId: string; readonly displayName: string; readonly changeKind: 'added' | 'modified' | 'deleted' | 'renamed'; readonly previousDisplayName?: string }[]; }
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
    fileReviewDocuments: root.fileReviewDocuments === undefined ? [] : array(root.fileReviewDocuments, 'fileReviewDocuments').map(fileReviewDocument),
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
    workUnits: [],
    workUnitScopes: [],
    workSlicePlanningPoints: [],
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
    internalArtifacts: query.fileReviewDocuments.map((x) => ({ artifactId: x.artifactId, provenanceId: x.provenanceId })),
    documentReferences: query.fileReviewDocuments.map((x) => ({ documentRefId: x.documentRefId, artifactIds: [x.artifactId], provenanceId: x.provenanceId })),
    provenance: initiated.map((x) => ({
      provenanceId: x.provenanceId,
      sourceKind: 'application' as const,
      recordedAt: x.recordedAt,
      causalFactIds: [x.epicId],
    })),
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
      artifacts: query.fileReviewDocuments.map((x) => ({ artifactId: x.artifactId as import('./artifactAccess').ArtifactId, kind: 'review_material' as const, provenanceReference: x.provenanceId })),
      changedFileReferences: query.fileReviewDocuments.flatMap((x) => x.changedFiles),
      documents: query.fileReviewDocuments.map((x) => ({ documentRefId: x.documentRefId as import('./artifactAccess').DocumentRefId, classification: 'changed_files' as const, title: x.title, ...(x.summary ? { summary: x.summary } : {}), artifactIds: [x.artifactId as import('./artifactAccess').ArtifactId], changedFileReferenceIds: x.changedFiles.map((f) => f.changedFileReferenceId), provenanceReference: x.provenanceId })),
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
      workSlicePlanningPoints: [],
      workUnits: [],
      gates: [],
      concerns: [],
      agentSessions: uniquePlanBuilderSessions.map(({ epic, association, draft }) => ({
        agentSessionId: association.agentSessionId,
        title: draft.title ?? 'Epic Plan Builder',
        source: source(epic.provenanceId),
      })),
      artifactOwnership: query.fileReviewDocuments.map((x) => ({ artifactId: x.artifactId, sprintId: x.sprintId, source: source(x.provenanceId) })),
      documentOwnership: query.fileReviewDocuments.map((x) => ({ documentRefId: x.documentRefId, sprintId: x.sprintId, source: source(x.provenanceId) })),
      sprintWorkspacePresentation: {
        workSlicePlanningPointMembership: [], gates: [],
        documents: query.fileReviewDocuments.map((x) => ({ documentRefId: x.documentRefId, displayOrder: query.fileReviewDocuments.filter((item) => item.sprintId === x.sprintId).indexOf(x), recordedAt: { source: source(x.provenanceId), value: query.generatedAt }, displayCategory: { source: source(x.provenanceId), value: 'File review' }, sprintPlanRevisionIds: [], workSlicePlanningPointIds: [], workUnitScopeIds: [] })),
      },
    },
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
const fileReviewDocument = (value: unknown): NativeFileReviewDocumentV1 => {
  const x = object(value, 'file review document');
  keys(x, ['documentRefId','epicId','sprintId','provenanceId','title','summary','artifactId','changedFiles'], 'file review document');
  return { documentRefId: string(x.documentRefId, 'documentRefId'), epicId: string(x.epicId, 'epicId'), sprintId: string(x.sprintId, 'sprintId'), provenanceId: string(x.provenanceId, 'provenanceId'), title: boundedString(x.title, 240, 'Document title'), ...(x.summary === undefined ? {} : { summary: boundedString(x.summary, 4000, 'Document summary') }), artifactId: string(x.artifactId, 'artifactId'), changedFiles: array(x.changedFiles, 'changedFiles').map((value) => { const file = object(value, 'changed file'); keys(file, ['changedFileReferenceId','displayName','changeKind','previousDisplayName'], 'changed file'); if (!['added','modified','deleted','renamed'].includes(file.changeKind as string)) fail('invalid changed file kind'); const changeKind = file.changeKind as 'added' | 'modified' | 'deleted' | 'renamed'; const previousDisplayName = file.previousDisplayName === undefined ? undefined : boundedString(file.previousDisplayName, 4000, 'previousDisplayName'); if ((changeKind === 'renamed') !== (previousDisplayName !== undefined)) fail('invalid renamed previous display name'); return { changedFileReferenceId: string(file.changedFileReferenceId, 'changedFileReferenceId'), displayName: boundedString(file.displayName, 4000, 'displayName'), changeKind, ...(previousDisplayName === undefined ? {} : { previousDisplayName }) }; }) };
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
function keys(value: Record<string, unknown>, allowed: readonly string[], label: string) {
  if (Object.keys(value).some((key) => !allowed.includes(key)))
    fail(`${label} contains an unknown field`);
}
function fail(message: string): never {
  throw new Error(`Invalid orchestration native query: ${message}`);
}
