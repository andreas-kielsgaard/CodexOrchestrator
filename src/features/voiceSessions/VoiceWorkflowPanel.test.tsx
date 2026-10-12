import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { VoiceView } from '../../application/voiceSessions';
import { VoiceSessionsScreen } from './VoiceSessionsScreen';

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

it('edits the real prompt draft and reaction route through the existing save boundary', async () => {
  const { client } = mount();
  const prompt = await screen.findByLabelText('Relevance prompt');
  fireEvent.change(prompt, { target: { value: 'Include recent task context.' } });
  fireEvent.click(screen.getByRole('button', { name: 'Inspect Request buffer' }));
  expect(screen.getByLabelText('Sound when a turn finishes')).not.toBeChecked();
  fireEvent.click(screen.getByLabelText('Sound when a turn finishes'));
  fireEvent.click(screen.getByLabelText('Route reaction-only requests directly to Output'));
  fireEvent.click(screen.getByRole('button', { name: 'Save workflow settings' }));
  await waitFor(() =>
    expect(client.configure).toHaveBeenCalledWith(
      expect.objectContaining({
        prompts: expect.objectContaining({ relevance: 'Include recent task context.' }),
        workflow: { directReactions: false, notificationSounds: { buffer: true } },
      }),
    ),
  );
  expect(await screen.findByText('Saved configuration')).toBeVisible();
});

it('separates queued steering from completed work and opens the actual routed session', async () => {
  const { onOpenSession } = mount();
  expect(await screen.findByText('Tell me more about those ideas.')).toBeVisible();
  expect(screen.getByText('Earlier activity · route evidence unavailable')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'output · queued' }));
  expect(onOpenSession).toHaveBeenCalledWith('output-session');
  fireEvent.click(screen.getByText('Connections · 1'));
  fireEvent.click(screen.getByRole('button', { name: 'Open Direct reaction in connection list' }));
  expect(screen.getByText('Reaction-only input.')).toBeVisible();
  expect(screen.getByText('Accepted request.')).toBeVisible();
});

it('blocks edits during an active connection and evaluates text through the real client', async () => {
  const view = fixture();
  view.state.enabled = true;
  const { client } = mount(view);
  expect(await screen.findByLabelText('Relevance prompt')).toBeDisabled();
  expect(screen.getByLabelText('Sound when a turn finishes')).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Save workflow settings' })).toBeDisabled();
  fireEvent.change(screen.getByLabelText('Try a text request'), {
    target: { value: 'Make the afternoon less rushed.' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Evaluate request' }));
  await waitFor(() =>
    expect(client.sendText).toHaveBeenCalledWith('Make the afternoon less rushed.'),
  );
});
