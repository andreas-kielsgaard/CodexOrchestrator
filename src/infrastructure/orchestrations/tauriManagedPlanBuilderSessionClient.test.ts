import type { AgentSessionClient } from '../../application/agentSessions';
import { createTauriManagedPlanBuilderSessionClient } from './tauriManagedPlanBuilderSessionClient';

describe('managed Plan Builder Agent Session adapter', () => {
  it('routes first and resume sends through the managed command while retaining shared reads', async () => {
    const calls: { command: string; args?: Record<string, unknown> }[] = [];
    const shared = agentClient();
    const client = createTauriManagedPlanBuilderSessionClient(
      shared,
      async <T>(command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return { sessionId: 'acknowledged-session', invocationId: 'invocation-1' } as T;
      },
    );

    await client.sendMessage({ submittedText: 'first', title: 'stable first title' });
    await client.sendMessage({ sessionId: 'acknowledged-session', submittedText: 'resume' });
    await client.loadSession({ sessionId: 'acknowledged-session' });

    expect(calls).toEqual([
      {
        command: 'send_managed_plan_builder_message',
        args: { input: { submittedText: 'first', title: 'stable first title' } },
      },
      {
        command: 'send_managed_plan_builder_message',
        args: { input: { sessionId: 'acknowledged-session', submittedText: 'resume' } },
      },
    ]);
    expect(shared.loadCalls).toEqual(['acknowledged-session']);
  });
});

function agentClient(): AgentSessionClient & { loadCalls: string[] } {
  const loadCalls: string[] = [];
  return {
    loadCalls,
    createSession: async () => session(),
    listSessions: async () => [],
    loadSession: async ({ sessionId }) => {
      loadCalls.push(sessionId);
      return { session: session(), invocations: [] };
    },
    reloadSession: async () => ({ session: session(), invocations: [] }),
    subscribeUpdates: async () => () => undefined,
    sendMessage: async () => ({ sessionId: 'wrong-generic-command', invocationId: 'wrong' }),
    cancelInvocation: async () => ({ id: 'i', sessionId: 's' }) as never,
    disconnectUpdates: async () => undefined,
  };
}

function session() {
  return {
    id: 'acknowledged-session',
    title: 'stable first title',
    availability: 'available' as const,
    runtimeBinding: { externalContextId: null, runtimeVersion: null },
    workingDirectory: null,
    requestedOptions: { model: null, sandbox: null },
    createdAt: '2026-07-15T00:00:00Z',
    updatedAt: '2026-07-15T00:00:00Z',
  };
}
