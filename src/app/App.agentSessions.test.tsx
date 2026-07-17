import { fireEvent, render, screen } from '@testing-library/react';
import type { AgentSessionClient } from '../application/agentSessions';
import { sessionDetails } from '../features/agentSessions/testFixtures';
import type { OrchestrationApplicationClient } from '../application/orchestrations';
import type { WorktreeRuntimeExplorationSource } from '../application/worktreeRuntime';
import { WorktreeRuntimeExplorationView } from '../features/worktreeRuntime/WorktreeRuntimeExplorationView';
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
    expect(screen.queryByRole('button', { name: /Worktree Runtime/ })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    expect(await screen.findByText('Start with a message')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestration' }));
    expect(screen.getByRole('main', { name: 'Orchestration' })).toBeVisible();
  });

  it('shows the development worktree runtime as one peer surface when its source is supplied', async () => {
    render(
      <App
        agentSessionClient={emptyAgentClient()}
        orchestrationClient={emptyOrchestrationClient()}
        worktreeRuntimeExplorationView={
          <WorktreeRuntimeExplorationView source={worktreeRuntimeSource()} />
        }
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: /Worktree Runtime/ }));

    expect(await screen.findByRole('main', { name: 'Worktree runtime' })).toBeVisible();
    expect(screen.getByText('proof-a')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Projected versus actual' })).toBeVisible();
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

function worktreeRuntimeSource(): WorktreeRuntimeExplorationSource {
  return {
    load: async () => ({
      label: 'Live instance metadata',
      notice: 'Development-only evidence.',
      checkedAt: '2026-07-17T08:00:00.000Z',
      identity: {
        instanceId: 'proof-a',
        sessionId: 'session-a',
        worktreePath: 'C:\\worktree-a',
        gitCommit: 'c25239f',
        sourceFingerprint: 'source-a',
        tauriIdentifier: 'dev.codex-orchestrator.worktree.a',
      },
      materials: [],
      lifecycle: [
        {
          stage: 'Running',
          state: 'Healthy owner match',
          detail: 'Observed.',
          evidence: 'observed',
        },
      ],
      unsupported: ['No product registry.'],
      reviewPoints: ['Choose the credential policy.'],
    }),
  };
}
