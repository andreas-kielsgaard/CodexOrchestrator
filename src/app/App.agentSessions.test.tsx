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
    expect(screen.queryByRole('button', { name: /Worktree Review/ })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    expect(await screen.findByText('Start with a message')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestration' }));
    expect(screen.getByRole('main', { name: 'Orchestration' })).toBeVisible();
  });

  it('shows the development worktree review launcher as one peer surface when supplied', async () => {
    render(
      <App
        agentSessionClient={emptyAgentClient()}
        orchestrationClient={emptyOrchestrationClient()}
        humanReviewLauncherView={<main aria-label="Worktree review launcher">Launcher proof</main>}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: /Worktree Review/ }));

    expect(await screen.findByRole('main', { name: 'Worktree review launcher' })).toBeVisible();
    expect(screen.getByText('Launcher proof')).toBeInTheDocument();
  });

  it('accepts only the application-owned non-activating launcher proof route', async () => {
    render(
      <App
        agentSessionClient={emptyAgentClient()}
        orchestrationClient={emptyOrchestrationClient()}
        humanReviewLauncherView={<main aria-label="Worktree review launcher">Launcher proof</main>}
        humanReviewLauncherNavigation={async () => 'worktree-review'}
      />,
    );

    expect(await screen.findByRole('main', { name: 'Worktree review launcher' })).toBeVisible();
    expect(screen.getByRole('button', { name: /Worktree Review/ })).toHaveAttribute(
      'aria-current',
      'page',
    );
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
