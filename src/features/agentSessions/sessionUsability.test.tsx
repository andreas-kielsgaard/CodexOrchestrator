import { act, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react';
import type {
  AgentSessionUpdateListener,
  SessionInteractionDto,
} from '../../application/agentSessions';
import { ConversationViewport } from './ConversationViewport';
import { projectAgentSessionTranscript } from './transcriptProjector';
import { runtimeEvent, sessionDetails, sessionSummary } from './testFixtures';
import { repairSessionClients } from './profileTestFixtures';
import { useAgentSessionCollection } from './useAgentSessionCollection';

const request: SessionInteractionDto = {
  id: 'request-1',
  invocationId: 'invocation-1',
  sequence: 1,
  kind: 'request',
  state: 'pending',
  result: null,
  content: {
    title: 'Choose a direction',
    kind: 'questions',
    questions: [{ id: 'choice', question: 'Which direction?', isOther: true }],
  },
};

it('keeps a pending question reachable across runtime updates and steering without duplicating its form', () => {
  const scroll = vi.spyOn(HTMLElement.prototype, 'scrollIntoView');
  const details = sessionDetails('running', [runtimeEvent(1, 'unknown', 'Runtime detail')]);
  details.interactions = [request];
  const target = {
    sessionId: 'session-1',
    active: true,
    steeringAvailable: true,
    draft: 'Correction',
    workingDirectory: '',
    sending: false,
    canceling: false,
    setDraft: vi.fn(),
    setWorkingDirectory: vi.fn(),
    send: vi.fn(async () => {}),
    cancel: vi.fn(async () => {}),
    respondToRequest: vi.fn(async () => {}),
  };
  const view = () => (
    <ConversationViewport
      segments={[{ id: 'session-1', transcript: projectAgentSessionTranscript(details) }]}
      loading={false}
      expandedProcessing={new Set()}
      onToggleProcessing={vi.fn()}
      composerTarget={{ ...target, interactions: details.interactions }}
    />
  );
  const { rerender } = render(view());
  expect(screen.getAllByLabelText('Which direction?')).toHaveLength(1);
  expect(screen.getByText('Runtime detail')).not.toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Review request' }));
  expect(screen.getByRole('article', { name: 'Agent request' })).toHaveFocus();
  expect(scroll).toHaveBeenCalledWith({ block: 'start' });
  details.interactions.push({
    ...request,
    id: 'steer-1',
    kind: 'steering',
    state: 'accepted',
    content: { text: 'Correction' },
  });
  details.invocations[0].events.push(runtimeEvent(2, 'unknown', 'More runtime detail'));
  rerender(view());
  expect(screen.getByRole('button', { name: 'Review request' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Steer' })).toBeEnabled();
  expect(screen.getByText('More runtime detail')).not.toBeVisible();
  details.interactions[0] = { ...request, state: 'answered' };
  rerender(view());
  expect(screen.queryByRole('button', { name: 'Review request' })).toBeNull();
  expect(screen.getByRole('button', { name: 'Submit answers' })).toBeDisabled();
  scroll.mockRestore();
});

it('refreshes pending request counts for an unselected session without moving selection', async () => {
  const fixture = repairSessionClients();
  let notify!: AgentSessionUpdateListener;
  fixture.sessions.subscribeUpdates = async (listener) => {
    notify = listener;
    return () => {};
  };
  let count = 0;
  fixture.sessions.listSessions = async () => [
    { ...sessionSummary(true), id: 'session-1' },
    { ...sessionSummary(true), id: 'session-2', pendingRequestCount: count },
  ];
  const { result } = renderHook(() => useAgentSessionCollection(fixture.sessions));
  await waitFor(() => expect(result.current.loading).toBe(false));
  count = 1;
  act(() =>
    notify({
      kind: 'event_persisted',
      sessionId: 'session-2',
      invocationId: 'turn-2',
      event: {
        ...runtimeEvent(1, 'unknown', null),
        rawPayload: { kind: 'runtime_request_opened' },
      },
    }),
  );
  await waitFor(() => expect(result.current.summaries[1].pendingRequestCount).toBe(1));
  count = 0;
  act(() =>
    notify({
      kind: 'event_persisted',
      sessionId: 'session-2',
      invocationId: 'turn-2',
      event: {
        ...runtimeEvent(2, 'unknown', null),
        rawPayload: { kind: 'runtime_request_response' },
      },
    }),
  );
  await waitFor(() => expect(result.current.summaries[1].pendingRequestCount).toBe(0));
});
