import { describe, expect, it } from 'vitest';
import {
  confirmationErrorMessage,
  decodeEpicInitiationConfirmationEvent,
  decodeEpicInitiationConfirmationRequest,
  decodeEpicInitiationConfirmationResolution,
} from './epicInitiationConfirmation';

const request = {
  requestId: 'request-1',
  source: { kind: 'agent', agent_session_id: 'session-1', agent_invocation_id: 'invocation-1' },
  epicPlanningDraftId: 'draft-1',
  state: 'requested',
};
const initiation = {
  initiationId: 'initiation-1',
  epicId: 'epic-1',
  proposalRevisionId: 'revision-1',
  materialSnapshotHash: 'hash-1',
  idempotentReplay: false,
};

describe('Epic initiation confirmation contracts', () => {
  it('decodes button and agent sources without guessing field names', () => {
    expect(decodeEpicInitiationConfirmationRequest(request).source).toEqual({
      kind: 'agent',
      agentSessionId: 'session-1',
      agentInvocationId: 'invocation-1',
    });
    expect(
      decodeEpicInitiationConfirmationRequest({ ...request, source: { kind: 'button' } }).source,
    ).toEqual({ kind: 'button' });
  });
  it('requires initiation facts only for applied, persisted, and projected states', () => {
    expect(decodeEpicInitiationConfirmationEvent({ request, state: 'requested' }).state).toBe(
      'requested',
    );
    expect(
      decodeEpicInitiationConfirmationEvent({ request, state: 'projected', initiation }).initiation,
    ).toEqual(initiation);
    expect(() => decodeEpicInitiationConfirmationEvent({ request, state: 'projected' })).toThrow(
      'do not match',
    );
    expect(() => decodeEpicInitiationConfirmationEvent({ request, state: 'mystery' })).toThrow(
      'unknown confirmation state',
    );
  });
  it('rejects malformed, unknown, and incomplete resolutions', () => {
    expect(
      decodeEpicInitiationConfirmationResolution({
        requestId: 'request-1',
        state: 'projected',
        initiation,
      }),
    ).toMatchObject({ state: 'projected' });
    expect(() =>
      decodeEpicInitiationConfirmationResolution({
        requestId: 'request-1',
        state: 'requested',
        initiation,
      }),
    ).toThrow('must be projected');
    expect(() => decodeEpicInitiationConfirmationRequest({ ...request, extra: true })).toThrow(
      'unknown field',
    );
  });
  it('maps stale, missing, and timeout failures without exposing native details', () => {
    expect(confirmationErrorMessage('request_not_found')).toContain('stale');
    expect(confirmationErrorMessage('stale_proposal')).toContain('proposal changed');
    expect(confirmationErrorMessage('timed_out')).toContain('timed out');
  });
});
