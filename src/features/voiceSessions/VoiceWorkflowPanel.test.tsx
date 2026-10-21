import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { VoiceView } from '../../application/voiceSessions';
import { VoiceSessionsScreen } from './VoiceSessionsScreen';
import { App } from '../../app/App';
import { sessionDetails, sessionSummary } from '../agentSessions/testFixtures';

function fixture(): VoiceView {
  const prompts = {
    relevance: 'Decide relevance.',
    completion: '',
    continuation: '',
    reaction: '',
    summary: '',
    router: '',
    digest: '',
    output: '',
    tools: '',
  };
  return {
    profiles: [],
    defaultPrompts: prompts,
    workflow: {
      id: 'voice',
      name: 'Voice interaction',
      owner: 'voice',
      runtime: { id: 'voice-runtime', name: 'Voice runtime', services: ['Phone gateway'] },
      nodes: [
        {
          id: 'filter',
          name: 'JEV evaluation',
          kind: 'capability',
          description: 'Evaluate current sentences.',
          x: 40,
          y: 40,
          promptKeys: ['relevance'],
          sessionIds: ['filter-session'],
        },
        {
          id: 'buffer',
          name: 'Request buffer',
          kind: 'function',
          description: 'Hold accepted input.',
          x: 340,
          y: 40,
          promptKeys: [],
          sessionIds: [],
        },
        {
          id: 'output',
          name: 'Output',
          kind: 'agent',
          description: 'Publish speech.',
          x: 640,
          y: 40,
          promptKeys: ['output'],
          sessionIds: ['output-session'],
        },
      ],
      connections: [
        {
          id: 'buffer-output',
          name: 'Direct reaction',
          source: 'buffer',
          destination: 'output',
          event: 'reaction_ready',
          condition: 'Reaction-only input.',
          payload: 'Accepted request.',
          enabled: true,
        },
      ],
    },
    state: {
      enabled: false,
      phoneConnected: false,
      speechConfigured: true,
      error: null,
      settings: {
        bindAddress: '127.0.0.1:4818',
        token: 'test-token',
        profileId: 'profile',
        workingDirectory: null,
        instructions: '',
        prompts,
        models: {
          response: { model: 'gpt-6-luna', reasoningMode: 'low', fastMode: true },
          routing: { model: 'gpt-6.1-sol', reasoningMode: 'medium', fastMode: false },
        },
        workflow: { directReactions: true },
      },
      conversation: {
        id: 'conversation',
        mainSessionId: 'router',
        delegations: [],
        pendingTopics: [],
        utterances: [],
        inputs: [],
        playback: null,
        activity: [
          {
            id: 'request',
            source: 'user',
            kind: 'accepted_input',
            createdAt: '2026-10-12T10:00:00Z',
            data: { blocks: [{ text: 'Tell me more about those ideas.' }] },
            route: {
              sourceNodeId: 'buffer',
              connectionIds: ['buffer-output'],
              destinations: [{ nodeId: 'output', sessionId: 'output-session' }],
            },
            deliveries: [
              {
                sessionId: 'output-session',
                receipt: { invocationId: 'active', inputId: 'steer', state: 'queued' },
              },
            ],
          },
          {
            id: 'older',
            source: 'user',
            kind: 'progress',
            createdAt: '2026-10-12T09:00:00Z',
            data: { text: 'Earlier event' },
          },
        ],
      },
    },
  };
}
function mount(view = fixture()) {
  const client = {
    read: vi.fn().mockImplementation(async () => view),
    configure: vi.fn().mockImplementation(async (settings) => {
      view.state.settings = settings;
      return view;
    }),
    start: vi.fn(),
    stop: vi.fn(),
    sendText: vi.fn(),
  };
  const onOpenSession = vi.fn();
  render(<VoiceSessionsScreen client={client} onOpenSession={onOpenSession} />);
  return { client, onOpenSession };
}

it('starts with an overview and retains a prompt draft across related views', async () => {
  const { client } = mount();
  expect(await screen.findByRole('heading', { name: 'How Voice works' })).toBeVisible();
  expect(screen.queryByLabelText('Relevance prompt')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Inspect JEV evaluation' }));
  fireEvent.click(screen.getByRole('button', { name: 'Relevance prompt' }));
  fireEvent.change(screen.getByLabelText('Relevance prompt'), {
    target: { value: 'Include recent task context.' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Connection' }));
  expect(screen.getByText('Connect your phone')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Workflow' }));
  expect(screen.getByLabelText('Relevance prompt')).toHaveValue('Include recent task context.');
  fireEvent.click(screen.getByRole('button', { name: '← Back to JEV evaluation' }));
  expect(screen.getByRole('heading', { name: 'JEV evaluation' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Inspect Request buffer' }));
  fireEvent.click(screen.getByRole('button', { name: 'To Output Direct reaction' }));
  fireEvent.click(screen.getByLabelText('Route reaction-only requests directly to Output'));
  expect(screen.getByText('Previewing draft route · not yet applied')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Start connection' })).toBeDisabled();
  fireEvent.click(screen.getByText('Review 2 unsaved changes'));
  expect(screen.getByText('Direct reaction route: disabled')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Save all changes' }));
  await waitFor(() =>
    expect(client.configure).toHaveBeenCalledWith(
      expect.objectContaining({
        prompts: expect.objectContaining({ relevance: 'Include recent task context.' }),
        workflow: { directReactions: false },
      }),
    ),
  );
  expect(await screen.findByText('Saved configuration')).toBeVisible();
});

it('discards the shared draft and restores the route preview', async () => {
  const { client } = mount();
  fireEvent.click(await screen.findByRole('button', { name: 'Inspect Request buffer' }));
  fireEvent.click(screen.getByRole('button', { name: 'To Output Direct reaction' }));
  fireEvent.click(screen.getByLabelText('Route reaction-only requests directly to Output'));
  fireEvent.click(screen.getByRole('button', { name: 'Connection' }));
  fireEvent.click(screen.getByRole('button', { name: 'Discard changes' }));
  fireEvent.click(screen.getByRole('button', { name: 'Workflow' }));
  expect(screen.getByLabelText('Route reaction-only requests directly to Output')).toBeChecked();
  expect(screen.queryByText('Previewing draft route · not yet applied')).toBeNull();
  expect(client.configure).not.toHaveBeenCalled();
});

it('keeps recorded delivery separate from the draft and opens the delivered invocation', async () => {
  const { onOpenSession } = mount();
  fireEvent.click(await screen.findByRole('button', { name: 'Recorded routes' }));
  fireEvent.change(screen.getByLabelText('Event'), { target: { value: 'request' } });
  expect(screen.getByText('Tell me more about those ideas.')).toBeVisible();
  expect(screen.getByText('Input queued')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Open delivered invocation' }));
  expect(onOpenSession).toHaveBeenCalledWith('output-session', 'active');
  fireEvent.click(screen.getByRole('button', { name: 'Connection' }));
  fireEvent.click(screen.getByRole('button', { name: 'Workflow' }));
  expect(screen.getByLabelText('Event')).toHaveValue('request');
  fireEvent.change(screen.getByLabelText('Event'), { target: { value: 'older' } });
  expect(screen.getByText('Route evidence unavailable for this event.')).toBeVisible();
});

it('blocks behavior edits while connected and sends text through the real client', async () => {
  const view = fixture();
  view.state.enabled = true;
  const { client } = mount(view);
  fireEvent.click(await screen.findByRole('button', { name: 'Inspect JEV evaluation' }));
  fireEvent.click(screen.getByRole('button', { name: 'Relevance prompt' }));
  expect(screen.getByLabelText('Relevance prompt')).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Activity' }));
  fireEvent.change(screen.getByLabelText('Try a text request'), {
    target: { value: 'Make the afternoon less rushed.' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Evaluate request' }));
  await waitFor(() =>
    expect(client.sendText).toHaveBeenCalledWith('Make the afternoon less rushed.'),
  );
});

it('edits the shared model from a role and keeps notification preferences secondary', async () => {
  const { client } = mount();
  fireEvent.click(await screen.findByRole('button', { name: 'Inspect Output' }));
  expect(screen.getByText('Turn notifications')).toBeVisible();
  expect(screen.getByLabelText('Sound when a turn finishes')).not.toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Response model group' }));
  expect(screen.getByText(/Quick receipt, Digest and Output/)).toBeVisible();
  fireEvent.change(screen.getByLabelText('Model'), { target: { value: 'custom-model' } });
  fireEvent.click(screen.getByRole('button', { name: 'Save all changes' }));
  await waitFor(() =>
    expect(client.configure).toHaveBeenCalledWith(
      expect.objectContaining({
        models: expect.objectContaining({
          response: expect.objectContaining({ model: 'custom-model' }),
        }),
      }),
    ),
  );
});

it('returns from a focused stage to the overview selection', async () => {
  mount();
  fireEvent.click(await screen.findByRole('button', { name: 'Inspect Request buffer' }));
  fireEvent.click(screen.getByRole('button', { name: 'Focus Request buffer' }));
  fireEvent.click(screen.getByRole('button', { name: 'Inspect Output' }));
  fireEvent.click(screen.getByRole('button', { name: '← All stages' }));
  expect(screen.getByRole('heading', { name: 'Request buffer' })).toBeVisible();
});

it('opens a recorded invocation and restores the exact Voice route through product Back', async () => {
  const view = fixture();
  const details = sessionDetails('completed');
  details.session.id = 'output-session';
  details.invocations[0].invocation.id = 'active';
  details.invocations[0].invocation.sessionId = 'output-session';
  render(
    <App
      initialSurface="voice-sessions"
      voiceSessionClient={{
        read: async () => view,
        configure: vi.fn(),
        start: vi.fn(),
        stop: vi.fn(),
        sendText: vi.fn(),
      }}
      orchestrationClient={{
        load: async () => ({ kind: 'empty', reason: 'No orchestration records.' }),
      }}
      agentSessionClient={{
        createSession: async () => details.session,
        listSessions: async () => [{ ...sessionSummary(), id: 'output-session' }],
        loadSession: async () => details,
        reloadSession: async () => details,
        subscribeUpdates: async () => () => undefined,
        sendMessage: vi.fn(),
        cancelInvocation: vi.fn(),
        disconnectUpdates: async () => undefined,
      }}
    />,
  );
  fireEvent.click(await screen.findByRole('button', { name: 'Recorded routes' }));
  fireEvent.change(screen.getByLabelText('Event'), { target: { value: 'request' } });
  fireEvent.click(screen.getByRole('button', { name: 'Open delivered invocation' }));
  await waitFor(() =>
    expect(document.querySelector('[data-invocation-id="active"]')).toHaveFocus(),
  );
  fireEvent.click(
    within(screen.getByRole('navigation', { name: 'Product commands' })).getByRole('button', {
      name: 'Back',
    }),
  );
  expect(screen.getByLabelText('Event')).toHaveValue('request');
  expect(screen.getByText('Tell me more about those ideas.')).toBeVisible();
});

it('keeps the request text after a failed submission so it can be retried', async () => {
  const view = fixture();
  view.state.enabled = true;
  const { client } = mount(view);
  client.sendText.mockRejectedValueOnce(new Error('Connection interrupted'));
  fireEvent.click(await screen.findByRole('button', { name: 'Activity' }));
  fireEvent.change(screen.getByLabelText('Try a text request'), {
    target: { value: 'Keep this request' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Evaluate request' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Connection interrupted');
  expect(screen.getByLabelText('Try a text request')).toHaveValue('Keep this request');
});

it('follows an applied evaluation to its recorded route and returns to the same evaluation', async () => {
  const view = fixture();
  view.state.conversation!.activity![0].data.requestId = 'release';
  view.state.conversation!.jevApplications = [
    { invocationId: 'evaluation', releaseId: 'release', decisions: [] },
  ];
  const invocation = { ...sessionDetails('completed').invocations[0].invocation, id: 'evaluation' };
  render(
    <VoiceSessionsScreen
      onOpenSession={vi.fn()}
      client={{
        read: async () => view,
        configure: vi.fn(),
        start: vi.fn(),
        stop: vi.fn(),
        sendText: vi.fn(),
      }}
      queryClient={{
        list: async () => [
          {
            invocationId: invocation.id,
            status: 'completed',
            createdAt: invocation.createdAt,
            startedAt: invocation.startedAt,
            completedAt: invocation.completedAt,
            inputPreview: 'Tell me more',
            questionCount: 0,
            answers: [],
          },
        ],
        load: async () => ({ invocation, evaluation: { state: {}, questions: {} }, result: null }),
      }}
    />,
  );
  fireEvent.click(await screen.findByRole('button', { name: 'Inspect JEV evaluation' }));
  fireEvent.click(screen.getByRole('button', { name: 'Inspect evaluations' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Show recorded route' }));
  expect(screen.getByLabelText('Event')).toHaveValue('request');
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: '← Back to evaluation' }));
  });
  expect(screen.getByRole('article', { name: 'Selected JEV query' })).toHaveTextContent(
    'Tell me more',
  );
  expect(screen.getByRole('button', { name: /Tell me more/ })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
});
