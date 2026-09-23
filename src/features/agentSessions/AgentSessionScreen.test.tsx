import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { AgentSessionScreen } from './AgentSessionScreen';
import { repairSessionClients } from './profileTestFixtures';
import { runtimeEvent, sessionDetails } from './testFixtures';

describe('AgentSessionScreen', () => {
  it('reloads completed history after a correlated update and lets the user expand processing', async () => {
    const { sessions, profiles } = repairSessionClients();
    const processing = runtimeEvent(1, 'tool_activity', 'Reading files');
    vi.spyOn(sessions, 'loadSession').mockResolvedValue(sessionDetails('running', [processing]));
    const completed = sessionDetails('completed', [
      processing,
      runtimeEvent(2, 'agent_message', 'The final answer', { role: 'final' }),
    ]);
    const reload = vi.spyOn(sessions, 'reloadSession').mockResolvedValue(completed);
    const subscriptions = vi.spyOn(sessions, 'subscribeUpdates');
    render(<AgentSessionScreen client={sessions} profileClient={profiles} />);
    expect(await screen.findByText('Reading files')).not.toBeVisible();
    fireEvent.click(
      screen.getByText('Working', { selector: '.processing-disclosure summary span' }),
    );
    expect(screen.getByText('Reading files')).toBeVisible();
    expect(screen.queryByText('The final answer')).toBeNull();

    await act(async () => {
      for (const [notify] of subscriptions.mock.calls)
        notify({
          kind: 'invocation_terminal',
          sessionId: 'session-1',
          invocationId: 'invocation-1',
          invocation: completed.invocations[0].invocation,
        });
    });
    expect(reload).toHaveBeenCalledWith({ sessionId: 'session-1' });
    expect(await screen.findByText('The final answer')).toBeVisible();
    expect(screen.getByText('Reading files')).toBeVisible();
    fireEvent.click(screen.getByText('Processing'));
    expect(screen.getByText('Reading files')).not.toBeVisible();
  });

  it('ignores another Session update while reflecting an update for the selected Session', async () => {
    const { sessions, profiles } = repairSessionClients();
    vi.spyOn(sessions, 'loadSession').mockResolvedValue(sessionDetails('running'));
    const updated = sessionDetails('running', [
      runtimeEvent(1, 'agent_message', 'Selected update', { role: 'intermediate' }),
    ]);
    const reload = vi.spyOn(sessions, 'reloadSession').mockImplementation(async ({ sessionId }) => {
      if (sessionId !== 'session-1')
        return sessionDetails('running', [runtimeEvent(1, 'agent_message', 'Foreign update')]);
      return updated;
    });
    const subscriptions = vi.spyOn(sessions, 'subscribeUpdates');
    render(<AgentSessionScreen client={sessions} profileClient={profiles} />);
    expect(await screen.findByText('Do the work')).toBeVisible();

    await act(async () => {
      for (const [notify] of subscriptions.mock.calls)
        notify({
          kind: 'event_persisted',
          sessionId: 'session-2',
          invocationId: 'invocation-1',
          event: runtimeEvent(1, 'agent_message', 'Foreign update'),
        });
    });
    expect(reload).not.toHaveBeenCalled();
    expect(screen.queryByText('Foreign update')).toBeNull();
    expect(screen.getByText('Do the work')).toBeVisible();

    await act(async () => {
      for (const [notify] of subscriptions.mock.calls)
        notify({
          kind: 'event_persisted',
          sessionId: 'session-1',
          invocationId: 'invocation-1',
          event: updated.invocations[0].events[0],
        });
    });
    expect(await screen.findByText('Selected update')).not.toBeVisible();
    fireEvent.click(
      screen.getByText('Working', { selector: '.processing-disclosure summary span' }),
    );
    expect(screen.getByText('Selected update')).toBeVisible();
    expect(reload).toHaveBeenCalledWith({ sessionId: 'session-1' });
  });

  it('displays and dismisses a collection-load failure', async () => {
    const { sessions, profiles } = repairSessionClients();
    vi.spyOn(sessions, 'listSessions').mockRejectedValue(new Error('Session list unavailable'));
    render(<AgentSessionScreen client={sessions} profileClient={profiles} />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Session list unavailable');
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss error' }));
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  });
});
