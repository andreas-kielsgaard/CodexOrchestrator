import { createAgentSessionHistorySource } from './tauriAgentSessionHistorySource';
import type { AgentSessionUpdateListener } from '../../application/agentSessions';
import { runtimeEvent, sessionDetails } from '../../features/agentSessions/testFixtures';

describe('AgentSessionHistorySource', () => {
  it('tracks omitted wire telemetry without publishing or losing sequence-gap detection', async () => {
    const initial = sessionDetails('running', [runtimeEvent(1, 'agent_message', 'Hello')]);
    initial.invocations[0] = { ...initial.invocations[0], lastEventSequence: 10 };
    let updates: AgentSessionUpdateListener | undefined;
    const load = vi.fn(async () => initial);
    const source = createAgentSessionHistorySource({
      load,
      subscribe: async (listener) => {
        updates = listener;
        return () => {};
      },
      schedule: (publish) => publish(),
    });
    const changed = vi.fn();
    source.subscribe('session-1', changed);
    await source.ensure('session-1');
    changed.mockClear();
    const delta = runtimeEvent(11, 'unknown', null);
    delta.normalized = null;
    delta.rawPayload = {
      method: 'item/commandExecution/outputDelta',
      params: { delta: 'fragment' },
    };
    updates?.({
      kind: 'event_persisted',
      sessionId: 'session-1',
      invocationId: 'invocation-1',
      event: delta,
    });
    expect(changed).not.toHaveBeenCalled();
    updates?.({
      kind: 'event_persisted',
      sessionId: 'session-1',
      invocationId: 'invocation-1',
      event: runtimeEvent(12, 'agent_message', 'Next'),
    });
    expect(load).toHaveBeenCalledTimes(1);
    expect(source.getSnapshot('session-1').details?.invocations[0].events).toHaveLength(2);
    updates?.({
      kind: 'event_persisted',
      sessionId: 'session-1',
      invocationId: 'invocation-1',
      event: runtimeEvent(14, 'agent_message', 'Gap'),
    });
    expect(load).toHaveBeenCalledTimes(2);
  });
  it('loads once and frame-batches a contiguous event burst without reloading history', async () => {
    const initial = sessionDetails('running');
    const load = vi.fn(async () => initial);
    const frames: (() => void)[] = [];
    let updates: AgentSessionUpdateListener | undefined;
    const source = createAgentSessionHistorySource({
      load,
      subscribe: async (listener) => {
        updates = listener;
        return () => undefined;
      },
      schedule: (publish) => frames.push(publish),
    });
    const changed = vi.fn();
    const unsubscribe = source.subscribe('session-1', changed);
    await source.ensure('session-1');
    await Promise.resolve();

    const stable = source.getSnapshot('session-1');
    expect(source.getSnapshot('session-1')).toBe(stable);
    for (let sequence = 1; sequence <= 80; sequence += 1) {
      updates?.({
        kind: 'event_persisted',
        sessionId: 'session-1',
        invocationId: 'invocation-1',
        event: runtimeEvent(sequence, 'processing_update', `Update ${sequence}`),
      });
    }

    expect(load).toHaveBeenCalledTimes(1);
    expect(frames).toHaveLength(1);
    frames.shift()?.();
    expect(source.getSnapshot('session-1').details?.invocations[0].events).toHaveLength(80);
    expect(source.getSnapshot('session-1').changes).toHaveLength(80);
    unsubscribe();
  });

  it('flushes terminal state immediately and coalesces a sequence-gap recovery load', async () => {
    const initial = sessionDetails('running');
    let finishRecovery: ((value: typeof initial) => void) | undefined;
    const load = vi
      .fn<() => Promise<typeof initial>>()
      .mockResolvedValueOnce(initial)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishRecovery = resolve;
          }),
      );
    let updates: AgentSessionUpdateListener | undefined;
    const source = createAgentSessionHistorySource({
      load,
      subscribe: async (listener) => {
        updates = listener;
        return () => undefined;
      },
      schedule: () => undefined,
    });
    source.subscribe('session-1', () => undefined);
    await source.ensure('session-1');
    await Promise.resolve();

    const completed = sessionDetails('completed').invocations[0].invocation;
    updates?.({
      kind: 'invocation_terminal',
      sessionId: 'session-1',
      invocationId: 'invocation-1',
      invocation: completed,
    });
    expect(source.getSnapshot('session-1').details?.invocations[0].invocation.status).toBe(
      'completed',
    );

    updates?.({
      kind: 'event_persisted',
      sessionId: 'session-1',
      invocationId: 'invocation-1',
      event: runtimeEvent(2, 'processing_update', 'Gap'),
    });
    updates?.({
      kind: 'event_persisted',
      sessionId: 'session-1',
      invocationId: 'invocation-1',
      event: runtimeEvent(3, 'processing_update', 'Same recovery'),
    });
    expect(load).toHaveBeenCalledTimes(2);
    finishRecovery?.(sessionDetails('completed', [runtimeEvent(1, 'processing_update', 'Done')]));
    await Promise.resolve();
  });
});

it('keeps discarded messages absent when an initial stale load and late notifications arrive', async () => {
  const initial = sessionDetails('pending');
  let finish!: (value: typeof initial) => void;
  let updates!: AgentSessionUpdateListener;
  const source = createAgentSessionHistorySource({
    load: () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
    subscribe: async (listener) => {
      updates = listener;
      return () => {};
    },
    schedule: (publish) => publish(),
  });
  source.subscribe('session-1', () => {});
  const pending = source.ensure('session-1');
  await vi.waitFor(() => expect(finish).toBeDefined());
  updates({
    kind: 'queue_updated',
    sessionId: 'session-1',
    removedInvocationIds: ['invocation-1'],
  });
  finish(initial);
  await pending;
  expect(source.getSnapshot('session-1').details?.invocations).toEqual([]);
  updates({
    kind: 'invocation_terminal',
    sessionId: 'session-1',
    invocationId: 'invocation-1',
    invocation: sessionDetails('canceled').invocations[0].invocation,
  });
  expect(source.getSnapshot('session-1').details?.invocations).toEqual([]);
});

it('publishes queue outcomes immediately without waiting for a foreground animation frame', async () => {
  const initial = sessionDetails();
  let updates!: AgentSessionUpdateListener;
  const frames: (() => void)[] = [];
  const source = createAgentSessionHistorySource({
    load: async () => initial,
    subscribe: async (listener) => {
      updates = listener;
      return () => {};
    },
    schedule: (publish) => frames.push(publish),
  });
  source.subscribe('session-1', () => {});
  await source.ensure('session-1');
  const timeline = {
    positions: [{ ordinal: 0, invocationId: null, operationId: 'completed-switch' }],
    transitions: [],
    providers: [],
  };
  updates({ kind: 'queue_updated', sessionId: 'session-1', timeline });
  expect(source.getSnapshot('session-1').details?.timeline).toEqual(timeline);
  expect(frames).toHaveLength(0);
});
