import type {
  AgentSessionDetailsDto,
  AgentSessionUpdateDto,
} from '../../application/agentSessions';
import {
  createRecordedAgentSessionClient,
  createRecordedAgentSessionStore,
} from './recordedAgentSessionClient';
import { recordedAgentSessionScenarios } from './scenarios';

describe('recorded Agent Session client', () => {
  it('advances live processing deterministically and preserves correlation and ordering', async () => {
    const client = createRecordedAgentSessionClient({
      scenario: recordedAgentSessionScenarios.liveProcessing,
    });
    const updates: AgentSessionUpdateDto[] = [];
    await client.subscribeUpdates((update) => updates.push(update));
    const details = await client.loadSession({ sessionId: 'live-session' });
    expect(details.invocations[0].invocation.status).toBe('pending');

    expect(client.advanceAll()).toBe(5);
    expect(updates).toHaveLength(5);
    expect(updates.map((update) => update.invocationId)).toEqual(Array(5).fill('live-invocation'));
    expect(
      updates
        .filter((update) => update.kind === 'event_persisted')
        .map((update) => update.event.sequence),
    ).toEqual([1, 2, 3, 4]);
    const completed = await client.reloadSession({ sessionId: 'live-session' });
    expect(completed.invocations[0].invocation.status).toBe('completed');
    expect(completed.invocations[0].events.map((item) => item.id)).toEqual([
      'live-1',
      'live-2',
      'live-3',
      'live-4',
    ]);
  });

  it('reconstructs durable history through a second client without replaying events', async () => {
    const store = createRecordedAgentSessionStore();
    const first = createRecordedAgentSessionClient({
      store,
      scenario: recordedAgentSessionScenarios.liveProcessing,
    });
    await first.advanceAll();
    const second = createRecordedAgentSessionClient({ store });
    expect(await second.listSessions()).toHaveLength(1);
    const details = await second.loadSession({ sessionId: 'live-session' });
    expect(details.invocations[0].events.map((item) => item.id)).toEqual([
      'live-1',
      'live-2',
      'live-3',
      'live-4',
    ]);
    expect(second.emittedUpdates).toEqual([]);
  });

  it('supports send, reload, cancellation, unsubscribe, and explicit operation errors', async () => {
    const client = createRecordedAgentSessionClient();
    const received: AgentSessionUpdateDto[] = [];
    const unsubscribe = await client.subscribeUpdates((update) => received.push(update));
    const sent = await client.sendMessage({ submittedText: 'Work' });
    expect(sent.sessionId).toBeTruthy();
    await expect(
      client.cancelInvocation({ invocationId: sent.invocationId }),
    ).resolves.toMatchObject({ status: 'canceled' });
    unsubscribe();
    expect(received).toHaveLength(1);
    expect(await client.reloadSession({ sessionId: sent.sessionId })).toMatchObject({
      invocations: [{ invocation: { status: 'canceled' } }],
    });

    const errors = recordedAgentSessionScenarios.errors;
    await expect(
      createRecordedAgentSessionClient({ scenario: errors }).subscribeUpdates(() => undefined),
    ).rejects.toThrow('subscription');
    const loaded = createRecordedAgentSessionClient({ scenario: errors });
    await expect(loaded.loadSession({ sessionId: 'missing' })).rejects.toThrow('load');
    await expect(loaded.reloadSession({ sessionId: 'missing' })).rejects.toThrow('reload');
    await expect(loaded.sendMessage({ submittedText: 'fail' })).rejects.toThrow('send');
    await expect(loaded.cancelInvocation({ invocationId: 'missing' })).rejects.toThrow(
      'cancellation',
    );
  });

  it('keeps raw payload opaque and diagnostics correlated', async () => {
    const client = createRecordedAgentSessionClient({
      scenario: recordedAgentSessionScenarios.diagnostics,
    });
    await client.advanceAll();
    const details = await client.loadSession({ sessionId: 'diagnostic-session' });
    expect(details.invocations[0].events[0].rawPayload).toEqual({ future: 'opaque' });
    expect(details.invocations[0].events[1].rawPayload).toBe('raw stderr');
    expect(details.invocations[0].invocation.diagnostics[0].code).toBe('FUTURE_EVENT');
  });

  it('returns defensive snapshots and does not mutate scenario definitions', async () => {
    const scenario = recordedAgentSessionScenarios.liveProcessing;
    const client = createRecordedAgentSessionClient({ scenario });
    const details = await client.loadSession({ sessionId: 'live-session' });
    details.session.title = 'mutated';
    details.invocations[0].events.push(
      {} as AgentSessionDetailsDto['invocations'][number]['events'][number],
    );
    expect((await client.loadSession({ sessionId: 'live-session' })).session.title).toBe(
      'live-session',
    );
    expect(scenario.steps[0].kind).toBe('event');
  });
});
