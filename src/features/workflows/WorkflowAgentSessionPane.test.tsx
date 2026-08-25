import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { vi } from 'vitest';
import {
  createRecordedAgentSessionClient,
  createRecordedAgentSessionStore,
} from '../../dev/agentSessions';
import { sessionDetails } from '../agentSessions/testFixtures';
import {
  WorkflowAgentSessionPane,
  type WorkflowAgentSessionItem,
} from './WorkflowAgentSessionPane';

describe('WorkflowAgentSessionPane', () => {
  it('hosts the shared new-Session workspace and reports the Session created by its bound client', async () => {
    const client = createRecordedAgentSessionClient();
    const sendMessage = vi.spyOn(client, 'sendMessage');
    const onSessionCreated = vi.fn();

    render(
      <WorkflowAgentSessionPane
        node={node}
        sessions={[]}
        client={client}
        allowEmptySession
        onReturn={() => undefined}
        onClose={() => undefined}
        onSessionCreated={onSessionCreated}
      />,
    );

    expect(await screen.findByRole('heading', { name: 'Start Sender' })).toBeVisible();
    expect(screen.getByLabelText('Initial message')).toBeVisible();
    expect(document.querySelectorAll('.agent-session-workspace')).toHaveLength(1);

    fireEvent.change(screen.getByLabelText('Initial message'), {
      target: { value: 'Review the implementation.' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Send' }));

    await waitFor(() =>
      expect(sendMessage).toHaveBeenCalledWith({ submittedText: 'Review the implementation.' }),
    );
    await waitFor(() => expect(onSessionCreated).toHaveBeenCalledWith('recorded-session-1'));
    expect(await screen.findByRole('heading', { name: 'Recorded Agent Session' })).toBeVisible();
  });

  it('sorts and selects Workflow-associated Sessions while rendering the shared workspace', async () => {
    const older = namedSession('session-older', 'Older Session');
    const newer = namedSession('session-newer', 'Newer Session');
    const client = createRecordedAgentSessionClient({
      store: createRecordedAgentSessionStore([older, newer]),
    });
    const sessions: WorkflowAgentSessionItem[] = [
      sessionItem('session-older', 'Older Session', '2026-08-25T10:00:00.000Z'),
      sessionItem('session-newer', 'Newer Session', '2026-08-25T11:00:00.000Z'),
    ];

    render(
      <WorkflowAgentSessionPane
        node={node}
        sessions={sessions}
        client={client}
        allowEmptySession={false}
        onReturn={() => undefined}
        onClose={() => undefined}
      />,
    );

    const navigation = screen.getByRole('navigation', { name: 'Sender Agent Sessions' });
    const sessionButtons = within(navigation).getAllByRole('button');
    expect(sessionButtons.map((button) => button.textContent)).toEqual([
      expect.stringContaining('Newer Session'),
      expect.stringContaining('Older Session'),
    ]);
    expect(sessionButtons[0]).toHaveAttribute('aria-pressed', 'true');
    expect(await screen.findByRole('heading', { name: 'Newer Session' })).toBeVisible();

    fireEvent.click(within(navigation).getByRole('button', { name: /Older Session/ }));
    expect(await screen.findByRole('heading', { name: 'Older Session' })).toBeVisible();
    expect(document.querySelectorAll('.agent-session-workspace')).toHaveLength(1);
  });

  it('keeps Workflow return and close controls in the shell and withholds an ineligible empty composer', async () => {
    const onReturn = vi.fn();
    const onClose = vi.fn();
    const client = createRecordedAgentSessionClient();
    const subscribeUpdates = vi.spyOn(client, 'subscribeUpdates');

    render(
      <WorkflowAgentSessionPane
        node={node}
        sessions={[]}
        client={client}
        allowEmptySession={false}
        onReturn={onReturn}
        onClose={onClose}
      />,
    );

    expect(screen.getByText('No Agent Sessions belong to this node.')).toBeVisible();
    expect(screen.queryByLabelText('Initial message')).toBeNull();
    expect(document.querySelector('.agent-session-workspace')).toBeNull();
    await waitFor(() => expect(subscribeUpdates).toHaveBeenCalledOnce());

    fireEvent.click(screen.getByRole('button', { name: 'Return to node' }));
    fireEvent.click(screen.getByRole('button', { name: 'Close node details' }));
    expect(onReturn).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });
});

const node = {
  id: 'sender',
  name: 'Sender',
  harnessName: 'Sender Harness',
};

function namedSession(id: string, title: string) {
  const details = structuredClone(sessionDetails('completed'));
  details.session.id = id;
  details.session.title = title;
  details.invocations[0]!.invocation.sessionId = id;
  return details;
}

function sessionItem(
  sessionId: string,
  title: string,
  associatedAt: string,
): WorkflowAgentSessionItem {
  return { sessionId, title, activity: 'idle', associatedAt };
}
