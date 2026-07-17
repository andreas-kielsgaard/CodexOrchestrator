export type EpicInitiationCapability =
  | {
      readonly status: 'blocked';
      readonly reason: string;
    }
  | {
      readonly status: 'already_initiated';
      readonly reason: string;
    }
  | {
      readonly status: 'ready';
      readonly request: {
        readonly epicPlanningDraftId: string;
        readonly expectedRevisionToken: string;
        readonly idempotencyKey: string;
      };
    };

export type EpicInitiationFailureKind =
  'stale_proposal' | 'canceled' | 'already_initiated' | 'unavailable';

/** Safe application error shape for the semantic command adapter. */
export class EpicInitiationError extends Error {
  constructor(readonly kind: EpicInitiationFailureKind) {
    super(kind);
  }
}

export function epicInitiationErrorMessage(error: unknown): string {
  if (!(error instanceof EpicInitiationError))
    return 'Epic initiation could not reach the durable service. Try again.';
  switch (error.kind) {
    case 'stale_proposal':
      return 'The proposal changed. Review the current proposal, then try initiation again.';
    case 'canceled':
      return 'This Epic Planning Draft was canceled and cannot be initiated.';
    case 'already_initiated':
      return 'This Epic has already been initiated. Refreshing the durable overview.';
    case 'unavailable':
      return 'Epic initiation is currently unavailable. Try again.';
  }
}

export const unavailableEpicInitiationCapability: EpicInitiationCapability = {
  status: 'blocked',
  reason: 'Select an active Epic Planning Draft with a current proposal before initiation.',
};

/** Durable authority and exact request input; the shared application controller owns transport. */
export function createEpicInitiationCapability(
  query: import('./nativeQuery').OrchestrationNativeQueryV2,
  draftId: string,
): EpicInitiationCapability {
  if (query.initiatedEpics.some((item) => item.epicPlanningDraftId === draftId))
    return {
      status: 'already_initiated',
      reason: 'This Epic Planning Draft has already been initiated.',
    };
  const draft = query.planningDrafts.find((item) => item.epicPlanningDraftId === draftId);
  if (draft?.status === 'canceled')
    return {
      status: 'blocked',
      reason: 'This Epic Planning Draft was canceled and cannot be initiated.',
    };
  if (!draft || draft.status !== 'active' || draft.currentProposal.status !== 'available')
    return {
      status: 'blocked',
      reason: 'A current active Epic Plan Proposal is required before initiation.',
    };
  const proposalRevisionId = draft.currentProposal.proposalRevisionId;
  const revision = query.proposalRevisions.find(
    (item) => item.proposalRevisionId === proposalRevisionId,
  );
  if (!revision)
    return { status: 'blocked', reason: 'The current Epic Plan Proposal cannot be verified.' };
  const idempotencyKey = `initiate:${draftId}:${revision.proposalRevisionId}`;
  return {
    status: 'ready',
    request: {
      epicPlanningDraftId: draftId,
      expectedRevisionToken: revision.revisionToken,
      idempotencyKey,
    },
  };
}
