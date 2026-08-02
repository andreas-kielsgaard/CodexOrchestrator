import { describe, expect, it } from 'vitest';
import { decodeEpicPauseRestartOutcome, decodeEpicPauseRestartQuery } from './epicPauseRestart';

const outcome = {
  actionId: 'action-1',
  kind: 'pause',
  status: 'partial',
  targetCount: 0,
  launchedCount: 0,
  targets: [],
};

describe('Epic Pause/Restart native contracts', () => {
  it('decodes only complete authoritative control state', () => {
    expect(
      decodeEpicPauseRestartQuery({
        epicId: 'epic-1',
        pause: { availability: 'available', reason: 'ready', current: outcome },
        restart: { availability: 'unavailable', reason: 'no interruption', current: null },
      }),
    ).toMatchObject({ epicId: 'epic-1', pause: { current: outcome } });
  });
  it.each([
    ['a non-record query', null],
    ['a query with a missing epic id', { pause: {}, restart: {} }],
    ['a query with an unknown field', { epicId: 'epic-1', pause: {}, restart: {}, extra: true }],
    [
      'a query control with an unknown field',
      {
        epicId: 'epic-1',
        pause: { availability: 'available', reason: 'ready', extra: true },
        restart: { availability: 'available', reason: 'ready' },
      },
    ],
  ])('rejects %s', (_label, value) => {
    expect(() => decodeEpicPauseRestartQuery(value)).toThrow();
  });
  it.each([
    ['a missing field', { ...outcome, launchedCount: undefined }],
    ['an unknown field', { ...outcome, extra: true }],
    ['a non-record value', []],
    ['a negative target count', { ...outcome, targetCount: -1 }],
    ['launched targets exceeding targets', { ...outcome, launchedCount: 3 }],
  ])('rejects %s', (_label, value) => {
    expect(() => decodeEpicPauseRestartOutcome(value)).toThrow();
  });
  it('keeps only each target’s correlated durable observation, including partial history', () => {
    const observation = {
      launchAcceptedAt: null,
      externalContext: null,
      providerActivity: null,
      providerTerminal: null,
      processTerminal: null,
      mcpToolActivities: [],
      mcpToolActivityPartial: true,
    };
    const value = {
      ...outcome,
      targetCount: 1,
      targets: [
        {
          sessionId: 'session-1',
          sourceInvocationId: 'source-1',
          cancelRequestedAt: '2026-08-02T00:00:00Z',
          interruptionStatus: 'canceled',
          interruptionObservedAt: '2026-08-02T00:00:01Z',
          sourceObservation: observation,
          controlInvocation: {
            invocationId: 'control-1',
            persistedAt: '2026-08-02T00:00:02Z',
            launchAcceptedAt: null,
            observation,
          },
          failure: null,
        },
      ],
    };
    expect(decodeEpicPauseRestartOutcome(value).targets[0]).toMatchObject({
      sourceInvocationId: 'source-1',
      controlInvocation: {
        invocationId: 'control-1',
        observation: { mcpToolActivityPartial: true },
      },
    });
  });
});
