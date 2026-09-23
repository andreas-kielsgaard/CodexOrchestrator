import { createAgentSessionHistorySource } from './tauriAgentSessionHistorySource';
import type { AgentSessionUpdateListener } from '../../application/agentSessions';
import { runtimeEvent, sessionDetails } from '../../features/agentSessions/testFixtures';

describe('AgentSessionHistorySource', () => {
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
