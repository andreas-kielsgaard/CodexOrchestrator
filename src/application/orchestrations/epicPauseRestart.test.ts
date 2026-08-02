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

const fullObservation = () => ({
  launchAcceptedAt: '2026-08-02T00:00:00Z',
  externalContext: {
    externalContextId: 'context-1',
    correlation: { eventId: 'event-1', sequence: 1, recordedAt: '2026-08-02T00:00:01Z' },
  },
  providerActivity: { eventId: 'event-2', sequence: 2, recordedAt: '2026-08-02T00:00:02Z' },
  providerTerminal: {
    status: 'completed',
    correlation: { eventId: 'event-3', sequence: 3, recordedAt: '2026-08-02T00:00:03Z' },
  },
  processTerminal: {
    status: 'completed',
    completedAt: '2026-08-02T00:00:04Z',
    exitCode: 0,
    signal: null,
  },
  mcpToolActivities: [
    {
      activity: {
        phase: 'completed',
        itemId: 'item-1',
        server: 'server-1',
        tool: 'tool-1',
        status: 'ok',
        resultClassification: 'succeeded',
      },
      correlation: { eventId: 'event-4', sequence: 4, recordedAt: '2026-08-02T00:00:05Z' },
    },
  ],
  mcpToolActivityPartial: false,
});

function outcomeWithObservation(observation: unknown) {
  return {
    ...outcome,
    targetCount: 1,
    targets: [
      {
        sessionId: 'session-1',
        sourceInvocationId: 'source-1',
        cancelRequestedAt: null,
        interruptionStatus: 'canceled',
        interruptionObservedAt: null,
        sourceObservation: observation,
        controlInvocation: null,
        failure: null,
      },
    ],
  };
}

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

  it('decodes every nested runtime-observation field from the shared contract', () => {
    expect(
      decodeEpicPauseRestartOutcome(outcomeWithObservation(fullObservation())).targets[0]
        .sourceObservation,
    ).toMatchObject({
      externalContext: { correlation: { sequence: 1 } },
      providerTerminal: { status: 'completed' },
      processTerminal: { exitCode: 0 },
      mcpToolActivities: [{ activity: { resultClassification: 'succeeded' } }],
    });
  });

  it('accepts a signed i32 process exit code', () => {
    const observed = decodeEpicPauseRestartOutcome(
      outcomeWithObservation({
        ...fullObservation(),
        processTerminal: { ...fullObservation().processTerminal, exitCode: -9 },
      }),
    );
    expect(observed.targets[0].sourceObservation?.processTerminal?.exitCode).toBe(-9);
  });

  it.each([
    ['an incomplete provider activity correlation', { ...fullObservation(), providerActivity: {} }],
    ['a scalar process terminal', { ...fullObservation(), processTerminal: 'completed' }],
    [
      'a running process terminal',
      {
        ...fullObservation(),
        processTerminal: { ...fullObservation().processTerminal, status: 'running' },
      },
    ],
    [
      'a pending process terminal',
      {
        ...fullObservation(),
        processTerminal: { ...fullObservation().processTerminal, status: 'pending' },
      },
    ],
    [
      'an out-of-range positive process exit code',
      {
        ...fullObservation(),
        processTerminal: { ...fullObservation().processTerminal, exitCode: 2147483648 },
      },
    ],
    [
      'an out-of-range negative process exit code',
      {
        ...fullObservation(),
        processTerminal: { ...fullObservation().processTerminal, exitCode: -2147483649 },
      },
    ],
    [
      'a fractional process exit code',
      {
        ...fullObservation(),
        processTerminal: { ...fullObservation().processTerminal, exitCode: 1.5 },
      },
    ],
    [
      'an unknown provider terminal enum',
      {
        ...fullObservation(),
        providerTerminal: { ...fullObservation().providerTerminal, status: 'unknown' },
      },
    ],
    [
      'an external-context unknown field',
      {
        ...fullObservation(),
        externalContext: { ...fullObservation().externalContext, extra: true },
      },
    ],
    [
      'an invalid MCP phase',
      {
        ...fullObservation(),
        mcpToolActivities: [
          {
            ...fullObservation().mcpToolActivities[0],
            activity: { ...fullObservation().mcpToolActivities[0].activity, phase: 'invalid' },
          },
        ],
      },
    ],
    [
      'an MCP correlation with a non-integer sequence',
      {
        ...fullObservation(),
        mcpToolActivities: [
          {
            ...fullObservation().mcpToolActivities[0],
            correlation: { ...fullObservation().mcpToolActivities[0].correlation, sequence: '4' },
          },
        ],
      },
    ],
  ])('rejects %s', (_label, observation) => {
    expect(() => decodeEpicPauseRestartOutcome(outcomeWithObservation(observation))).toThrow();
  });
});
