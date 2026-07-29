import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { AgentSessionClient } from '../application/agentSessions';
import { sessionDetails } from '../features/agentSessions/testFixtures';
import type { OrchestrationApplicationClient } from '../application/orchestrations';
import { App } from './App';
import { createRecordedDevelopmentApplicationComposition } from '../dev/orchestrationSection/recordedOrchestrationClient';

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

  it('opens one product-owned Session binding in standalone view and returns to its typed product view', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    const loadedSessionIds: string[] = [];
    const tracingClient: AgentSessionClient = {
      ...composition.agentSessionClient,
      loadSession: async (request) => {
        loadedSessionIds.push(request.sessionId);
        return composition.agentSessionClient.loadSession(request);
      },
    };
    render(
      <App
        {...composition}
        agentSessionClient={tracingClient}
        orchestrationAgentSessionComposition={{ client: tracingClient }}
      />,
    );

    fireEvent.click(await screen.findByRole('button', { name: /Open Codex Epic Runner/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Open in Agent Sessions' }));

    expect(
      await screen.findByRole('heading', { name: 'Orientation discovery handler' }),
    ).toBeVisible();
    expect(screen.queryByRole('main', { name: 'Epic detail' })).toBeNull();
    await waitFor(() =>
      expect(
        screen.getByRole('treeitem', { name: /Orientation discovery handler/ }),
      ).toHaveAttribute('aria-selected', 'true'),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Go to Epic' }));
    expect(await screen.findByRole('main', { name: 'Epic detail' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    expect(
      await screen.findByRole('heading', { name: 'Orientation discovery handler' }),
    ).toBeVisible();
    expect(screen.getByRole('treeitem', { name: /Orientation discovery handler/ })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    expect(loadedSessionIds.length).toBeGreaterThanOrEqual(2);
    expect(new Set(loadedSessionIds)).toEqual(
      new Set(['recorded-epic-runner-manual-continuation-ready']),
    );
  });

  it('routes a Work Unit Session to the exact typed Work Unit detail', async () => {
    render(<App {...createRecordedDevelopmentApplicationComposition()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    const tree = await screen.findByRole('tree', { name: 'Session hierarchy' });
    fireEvent.click(
      within(tree).getByRole('treeitem', {
        name: 'Codex Epic Runner workspace development',
      }),
    );
    fireEvent.click(
      within(tree).getByRole('treeitem', { name: 'Sprint Control Surface Discovery' }),
    );
    fireEvent.click(within(tree).getByRole('treeitem', { name: 'Execution' }));
    fireEvent.click(
      within(tree).getByRole('treeitem', { name: 'Plan and Work Unit detail surfaces' }),
    );
    fireEvent.click(within(tree).getByRole('treeitem', { name: /Recorded WU-ECS2E worker/ }));

    expect(await screen.findByRole('button', { name: 'Go to Work Unit' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Go to Work Unit' }));
    expect(await screen.findByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
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
