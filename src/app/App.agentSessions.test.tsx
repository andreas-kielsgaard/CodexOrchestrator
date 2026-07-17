import { fireEvent, render, screen } from '@testing-library/react';
import type { AgentSessionClient } from '../application/agentSessions';
import { sessionDetails } from '../features/agentSessions/testFixtures';
import type { OrchestrationApplicationClient } from '../application/orchestrations';
import { App } from './App';

describe('App application surfaces', () => {
  it('switches between peer Orchestration and Agent Sessions capability surfaces', async () => {
    render(
      <App
        agentSessionClient={emptyAgentClient()}
        orchestrationClient={emptyOrchestrationClient()}
      />,
    );

    expect(screen.getByRole('navigation', { name: 'Application surfaces' })).toBeVisible();
    expect(screen.getByRole('main', { name: 'Orchestration' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    expect(await screen.findByText('Start with a message')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestration' }));
    expect(screen.getByRole('main', { name: 'Orchestration' })).toBeVisible();
  });

  it('presents an injected development view as an optional peer surface', async () => {
    render(
      <App
        agentSessionClient={emptyAgentClient()}
        orchestrationClient={pendingOrchestrationClient()}
        initialSurface="development"
        developmentSurface={{
          label: 'Development proof',
          render: () => <main aria-label="Development proof">Injected development view</main>,
        }}
      />,
    );

    expect(screen.getByRole('button', { name: 'Development proof' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(screen.getByRole('main', { name: 'Development proof' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestration' }));
    expect(await screen.findByRole('main', { name: 'Orchestration' })).toBeVisible();
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

function emptyOrchestrationClient(): OrchestrationApplicationClient {
  return { load: async () => ({ kind: 'empty', reason: 'No orchestration records.' }) };
}

function pendingOrchestrationClient(): OrchestrationApplicationClient {
  return { load: () => new Promise(() => undefined) };
}
