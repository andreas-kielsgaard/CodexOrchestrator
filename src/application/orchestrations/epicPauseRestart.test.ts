import { describe, expect, it } from 'vitest';
import { decodeEpicPauseRestartOutcome, decodeEpicPauseRestartQuery } from './epicPauseRestart';

const outcome = { actionId: 'action-1', kind: 'pause', status: 'partial', targetCount: 2, launchedCount: 1 };

describe('Epic Pause/Restart native contracts', () => {
  it('decodes only complete authoritative control state', () => {
    expect(decodeEpicPauseRestartQuery({ epicId: 'epic-1', pause: { availability: 'available', reason: 'ready', current: outcome }, restart: { availability: 'unavailable', reason: 'no interruption', current: null } })).toMatchObject({ epicId: 'epic-1', pause: { current: outcome } });
  });
  it('rejects malformed outcomes and impossible counts', () => {
    expect(() => decodeEpicPauseRestartOutcome({ ...outcome, extra: true })).toThrow();
    expect(() => decodeEpicPauseRestartOutcome({ ...outcome, launchedCount: 3 })).toThrow();
  });
});
