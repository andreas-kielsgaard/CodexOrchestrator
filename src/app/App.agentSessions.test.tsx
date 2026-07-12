import { render, screen } from '@testing-library/react';
import type { AgentSessionClient } from '../application/agentSessions';
import { sessionDetails } from '../features/agentSessions/testFixtures';
import { App } from './App';

describe('App Agent Session shell', () => {
  it('mounts Agent Sessions as the only application surface', async () => {
    render(<App agentSessionClient={emptyAgentClient()} />);

    expect(await screen.findByText('Start with a message')).toBeInTheDocument();
    expect(
      screen.queryByRole('navigation', { name: 'Application surfaces' }),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Legacy Tasks' })).not.toBeInTheDocument();
  });
});

function emptyAgentClient(): AgentSessionClient {
  return {
    createSession: async () => sessionDetails().session,
    listSessions: async () => [],
    loadSession: async () => sessionDetails(),
    reloadSession: async () => sessionDetails(),
    subscribeUpdates: async () => () => undefined,
    sendMessage: async () => ({ sessionId: 'session-1', invocationId: 'invocation-1' }),
    cancelInvocation: async () => sessionDetails('canceled').invocations[0].invocation,
    disconnectUpdates: async () => undefined,
  };
}
