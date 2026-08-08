export const EPIC_INITIATION_CONFIRMATION_EVENT = 'orchestration://epic-initiation-confirmation';

export type EpicInitiationConfirmationState =
  'requested' | 'user_confirmed' | 'user_rejected' | 'applied' | 'persisted' | 'projected';

export type EpicInitiationRequestSource =
  | { readonly kind: 'button' }
  | {
      readonly kind: 'agent';
      readonly agentSessionId: string;
      readonly agentInvocationId: string;
    };

export interface EpicInitiationConfirmationRequest {
  readonly requestId: string;
  readonly source: EpicInitiationRequestSource;
  readonly epicPlanningDraftId: string;
  readonly state: 'requested';
}

export interface EpicInitiationResult {
  readonly initiationId: string;
  readonly epicId: string;
  readonly proposalRevisionId: string;
  readonly materialSnapshotHash: string;
  readonly idempotentReplay: boolean;
}

export interface EpicInitiationConfirmationEvent {
  readonly request: EpicInitiationConfirmationRequest;
  readonly state: EpicInitiationConfirmationState;
  readonly initiation?: EpicInitiationResult;
}

export interface EpicInitiationConfirmationResolution {
  readonly requestId: string;
  readonly state: 'projected';
  readonly initiation: EpicInitiationResult;
}

export type EpicInitiationConfirmationFailureKind =
  | 'rejected'
  | 'rejected_notification_failed'
  | 'confirmed_not_applied'
  | 'request_not_found'
  | 'timed_out'
  | 'stale_proposal'
  | 'canceled'
  | 'already_initiated'
  | 'persisted_reconciliation_required'
  | 'unavailable'
  | 'malformed_event';

export class EpicInitiationConfirmationError extends Error {
  constructor(readonly kind: EpicInitiationConfirmationFailureKind) {
    super(kind);
  }
}

export interface EpicInitiationConfirmationDetails {
  readonly title: string;
  readonly sprintTitles: readonly string[];
}

export interface EpicInitiationConfirmationClient {
  request(input: {
    epicPlanningDraftId: string;
    expectedRevisionToken: string;
    idempotencyKey: string;
    rootBranch: string;
  }): Promise<EpicInitiationConfirmationRequest>;
  resolve(
    requestId: string,
    decision: 'confirmed' | 'rejected',
  ): Promise<EpicInitiationConfirmationResolution>;
  subscribe(
    listener: (event: EpicInitiationConfirmationEvent) => void,
    onMalformed: () => void,
  ): Promise<() => void>;
  describe(request: EpicInitiationConfirmationRequest): Promise<EpicInitiationConfirmationDetails>;
}

export function decodeEpicInitiationConfirmationRequest(
  value: unknown,
): EpicInitiationConfirmationRequest {
  const request = object(value, 'request');
  keys(request, ['requestId', 'source', 'epicPlanningDraftId', 'state'], 'request');
  if (request.state !== 'requested') invalid('request state must be requested');
  return {
    requestId: text(request.requestId, 'requestId'),
    source: decodeSource(request.source),
    epicPlanningDraftId: text(request.epicPlanningDraftId, 'epicPlanningDraftId'),
    state: 'requested',
  };
}

export function decodeEpicInitiationConfirmationEvent(
  value: unknown,
): EpicInitiationConfirmationEvent {
  const event = object(value, 'event');
  keys(event, ['request', 'state', 'initiation'], 'event');
  const request = decodeEpicInitiationConfirmationRequest(event.request);
  const state = confirmationState(event.state);
  const initiation = event.initiation == null ? undefined : decodeInitiation(event.initiation);
  if ((state === 'applied' || state === 'persisted' || state === 'projected') !== !!initiation)
    invalid('effect states and initiation result do not match');
  return { request, state, ...(initiation ? { initiation } : {}) };
}

export function decodeEpicInitiationConfirmationResolution(
  value: unknown,
): EpicInitiationConfirmationResolution {
  const resolution = object(value, 'resolution');
  keys(resolution, ['requestId', 'state', 'initiation'], 'resolution');
  if (resolution.state !== 'projected' || resolution.initiation == null)
    invalid('resolution must be projected with initiation');
  return {
    requestId: text(resolution.requestId, 'requestId'),
    state: 'projected',
    initiation: decodeInitiation(resolution.initiation),
  };
}

export function confirmationFailureKind(error: unknown): EpicInitiationConfirmationFailureKind {
  const code =
    error && typeof error === 'object' && 'code' in error
      ? (error as { code?: unknown }).code
      : undefined;
  if (
    typeof code === 'string' &&
    [
      'rejected',
      'rejected_notification_failed',
      'confirmed_not_applied',
      'request_not_found',
      'timed_out',
      'stale_proposal',
      'canceled',
      'already_initiated',
      'persisted_reconciliation_required',
      'unavailable',
    ].includes(code)
  )
    return code as EpicInitiationConfirmationFailureKind;
  return 'unavailable';
}

export function confirmationErrorMessage(kind: EpicInitiationConfirmationFailureKind): string {
  switch (kind) {
    case 'request_not_found':
      return 'This confirmation request is stale or no longer available.';
    case 'timed_out':
      return 'This confirmation request timed out before it was resolved.';
    case 'stale_proposal':
      return 'The proposal changed before confirmation. Review the current proposal and try again.';
    case 'canceled':
      return 'This planning draft was canceled before confirmation.';
    case 'already_initiated':
      return 'This Epic was already initiated; the durable view must be refreshed.';
    case 'persisted_reconciliation_required':
      return 'Initiation persisted, but its projected transition needs reconciliation.';
    case 'confirmed_not_applied':
      return 'Confirmation was recorded, but initiation was not applied.';
    case 'rejected_notification_failed':
      return 'Rejection could not be delivered to the waiting request.';
    case 'malformed_event':
      return 'An invalid confirmation event was rejected.';
    case 'rejected':
      return 'Initiation was rejected.';
    case 'unavailable':
      return 'Epic initiation confirmation is currently unavailable.';
  }
}

function decodeSource(value: unknown): EpicInitiationRequestSource {
  const source = object(value, 'source');
  if (source.kind === 'button') {
    keys(source, ['kind'], 'button source');
    return { kind: 'button' };
  }
  if (source.kind === 'agent') {
    keys(source, ['kind', 'agent_session_id', 'agent_invocation_id'], 'agent source');
    return {
      kind: 'agent',
      agentSessionId: text(source.agent_session_id, 'agentSessionId'),
      agentInvocationId: text(source.agent_invocation_id, 'agentInvocationId'),
    };
  }
  invalid('unknown request source');
}
function decodeInitiation(value: unknown): EpicInitiationResult {
  const result = object(value, 'initiation');
  keys(
    result,
    ['initiationId', 'epicId', 'proposalRevisionId', 'materialSnapshotHash', 'idempotentReplay'],
    'initiation',
  );
  if (typeof result.idempotentReplay !== 'boolean') invalid('idempotentReplay must be boolean');
  return {
    initiationId: text(result.initiationId, 'initiationId'),
    epicId: text(result.epicId, 'epicId'),
    proposalRevisionId: text(result.proposalRevisionId, 'proposalRevisionId'),
    materialSnapshotHash: text(result.materialSnapshotHash, 'materialSnapshotHash'),
    idempotentReplay: result.idempotentReplay,
  };
}
function confirmationState(value: unknown): EpicInitiationConfirmationState {
  if (
    !['requested', 'user_confirmed', 'user_rejected', 'applied', 'persisted', 'projected'].includes(
      String(value),
    )
  )
    invalid('unknown confirmation state');
  return value as EpicInitiationConfirmationState;
}
function object(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    invalid(`${label} must be an object`);
  return value as Record<string, unknown>;
}
function text(value: unknown, label: string): string {
  if (typeof value !== 'string' || value.trim() === '') invalid(`${label} must be non-empty text`);
  return value;
}
function keys(value: Record<string, unknown>, allowed: readonly string[], label: string) {
  if (Object.keys(value).some((key) => !allowed.includes(key)))
    invalid(`${label} contains an unknown field`);
  if (allowed.filter((key) => key !== 'initiation').some((key) => !(key in value)))
    invalid(`${label} is missing a required field`);
}
function invalid(message: string): never {
  throw new Error(`Invalid Epic initiation confirmation: ${message}`);
}
