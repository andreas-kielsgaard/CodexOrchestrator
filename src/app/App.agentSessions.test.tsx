import { fireEvent, render, screen } from '@testing-library/react';
import type { AgentSessionClient } from '../application/agentSessions';
import { sessionDetails } from '../features/agentSessions/testFixtures';
import type { OrchestrationApplicationClient } from '../application/orchestrations';
import { createRecordedFileReviewApplicationComposition } from '../dev/fileReview/recordedFileReviewClient';
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
    expect(screen.queryByRole('button', { name: 'Files & diffs' })).toBeNull();
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
    expect(await screen.findByText('Start with a message')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Files & diffs' }));
    expect(screen.getByRole('main', { name: 'Files and diffs' })).toBeVisible();
    expect(await screen.findByText('5 changed files')).toBeVisible();
  });

  it('binds a development fixture before the scoped File Review view renders', async () => {
    render(<App {...createRecordedFileReviewApplicationComposition('application-owned')} />);

    expect(await screen.findByRole('main', { name: 'Files and diffs' })).toBeVisible();
    expect(await screen.findByText('1 changed files')).toBeVisible();
    expect(screen.queryByRole('combobox')).toBeNull();
    expect(screen.queryByText('Recorded application-owned review material')).toBeNull();
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
