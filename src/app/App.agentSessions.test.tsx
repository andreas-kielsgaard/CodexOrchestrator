import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { AgentSessionClient } from '../application/agentSessions';
import { sessionDetails } from '../features/agentSessions/testFixtures';
import type { OrchestrationApplicationClient } from '../application/orchestrations';
import {
  createRecordedFileReviewApplicationComposition,
  createRecordedFileReviewSource,
} from '../dev/fileReview/recordedFileReviewClient';
import type {
  ContextualFileReviewClient,
  ContextualFileReviewResult,
} from '../application/contextualFileReview';
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
    expect(screen.queryByRole('button', { name: 'Files & diffs' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Worktree Review Dev' })).toBeNull();
  });

  it('adds the development-only file review tab only when its client is injected', async () => {
    render(<App {...createRecordedFileReviewApplicationComposition()} />);

    expect(await screen.findByRole('main', { name: 'Files and diffs' })).toBeVisible();
    expect(await screen.findByText('5 changed files')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Files & diffs' })).toHaveAttribute(
      'aria-current',
      'page',
    );

    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    await waitFor(() => expect(screen.getByText('Start with a message')).toBeVisible());

    fireEvent.click(screen.getByRole('button', { name: 'Files & diffs' }));
    expect(screen.getByRole('main', { name: 'Files and diffs' })).toBeVisible();
    expect(await screen.findByText('5 changed files')).toBeVisible();
  });

  it('adds Worktree Review only through the injected development composition', async () => {
    render(
      <App
        agentSessionClient={emptyAgentClient()}
        orchestrationClient={emptyOrchestrationClient()}
        humanReviewLauncherView={<main aria-label="Retained worktree builds">Launcher</main>}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Worktree Review Dev' }));
    expect(screen.getByRole('main', { name: 'Retained worktree builds' })).toBeVisible();

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

  it('round-trips the WU-ECS2E handler without manufacturing a Reviewer Session', async () => {
    const composition = createRecordedDevelopmentApplicationComposition({
      includeWorkUnitReview: true,
    });
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
    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    const tree = await screen.findByRole('tree', { name: 'Epics session hierarchy' });
    await expandTreeItem(tree, /Codex Epic Runner workspace development/);
    await expandTreeItem(tree, 'Sprint Control Surface Discovery');
    await expandTreeItem(tree, 'Integrated detail surfaces');
    await expandTreeItem(tree, 'Plan and Work Unit detail surfaces');
    const handlerTreeItem = within(tree).getByRole('treeitem', {
      name: /Recorded WU-ECS2E Work Unit Handler/,
    });
    expect(handlerTreeItem).toHaveTextContent('Work Unit Handler');
    expect(
      within(tree).getByRole('treeitem', {
        name: /Recorded WU-ECS2E Work Unit Implementer/,
      }),
    ).toHaveTextContent('Work Unit Implementer');
    expect(within(tree).queryByRole('treeitem', { name: /Reviewer/ })).toBeNull();
    fireEvent.click(handlerTreeItem);

    expect(await screen.findByRole('button', { name: 'Go to Work Unit' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Go to Work Unit' }));
    expect(await screen.findByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    expect(screen.queryByRole('region', { name: /Agent Session$/ })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Reviewer' })).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Reviewer' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Reviewer Agent Session' })).toBeNull();
    fireEvent.click(
      screen.getByRole('button', {
        name: /Handler action.*recorded-handler-WU-ECS2E-first-review/,
      }),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Open in Agent Sessions' }));
    expect(
      await screen.findByRole('heading', { name: 'Recorded WU-ECS2E Work Unit Handler' }),
    ).toBeVisible();
    expect(
      screen.getByRole('treeitem', { name: /Recorded WU-ECS2E Work Unit Handler/ }),
    ).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('region', { name: 'Work Unit return context' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Return to Work Unit Activity' }));
    expect(await screen.findByRole('main', { name: 'Work Unit detail: WU-ECS2E' })).toBeVisible();
    expect(
      screen.getByLabelText('Agent Session turn: recorded-handler-WU-ECS2E-first-review'),
    ).toBeVisible();
    expect(loadedSessionIds).toContain('recorded-session-WU-ECS2E');
    fireEvent.click(screen.getByRole('tab', { name: 'Evidence' }));
    fireEvent.click(
      screen.getByRole('button', {
        name: /src\/features\/orchestrations\/components\/WorkUnitDetailWorkspace.tsx/,
      }),
    );
    expect(await screen.findByRole('main', { name: 'Files and diffs' })).toBeVisible();
    expect(
      screen.getByRole('region', {
        name: 'src/features/orchestrations/components/WorkUnitDetailWorkspace.tsx',
      }),
    ).toBeVisible();
    expect(loadedSessionIds).not.toContain('recorded-session-reviewer-WU-ECS2E');
  });

  it('keeps the application-owned File Review source out of Sprint Document actions', async () => {
    render(<App {...createRecordedFileReviewApplicationComposition()} />);

    fireEvent.click(screen.getByRole('button', { name: 'Orchestration' }));
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Open Codex Epic Runner workspace development',
      }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );
    fireEvent.click(screen.getByRole('tab', { name: 'Documents' }));
    const documentTitle = screen.getByText('Application-owned file review');
    expect(documentTitle).toBeVisible();
    expect(documentTitle.closest('article')).toBeVisible();
    expect(screen.queryByRole('button', { name: /Review files|View document/ })).toBeNull();
  });

  it('navigates from initiated Sprint context only after the scoped File Review source is ready', async () => {
    const composition = createRecordedDevelopmentApplicationComposition();
    let settle!: (
      result: Awaited<ReturnType<ContextualFileReviewClient['requestForSprint']>>,
    ) => void;
    const contextualFileReviewClient: ContextualFileReviewClient = {
      requestForSprint: vi.fn(
        () =>
          new Promise<ContextualFileReviewResult>((resolve) => {
            settle = resolve;
          }),
      ),
    };
    render(<App {...composition} contextualFileReviewClient={contextualFileReviewClient} />);
    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Open Codex Epic Runner workspace development',
      }),
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Sprint: Sprint Control Surface Discovery' }),
    );

    expect(screen.queryByRole('button', { name: 'Files & diffs' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Review files' }));
    expect(screen.getByRole('main', { name: 'Sprint detail' })).toBeVisible();
    expect(screen.getByText('Preparing File Review…')).toBeVisible();

    await act(async () => {
      settle({
        status: 'ready',
        source: createRecordedFileReviewSource('working-tree'),
        idempotentReplay: false,
      });
    });

    expect(await screen.findByRole('main', { name: 'Files and diffs' })).toBeVisible();
    expect(await screen.findByText('5 changed files')).toBeVisible();
    expect(contextualFileReviewClient.requestForSprint).toHaveBeenCalledWith(
      'sprint-control-surface',
    );
    expect(screen.queryByRole('button', { name: 'Files & diffs' })).toBeNull();
  });
});

async function expandTreeItem(tree: HTMLElement, name: string | RegExp) {
  const item = await within(tree).findByRole('treeitem', { name });
  if (item.getAttribute('aria-expanded') !== 'true') fireEvent.click(item);
  await waitFor(() => expect(item).toHaveAttribute('aria-expanded', 'true'));
}

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
