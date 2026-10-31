import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { VoiceView } from '../../application/voiceSessions';
import { VoiceSessionsScreen } from './VoiceSessionsScreen';
import { repairClients } from '../workflowAuthoring/testFixtures';
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

function clientFor(view: VoiceView) {
  return {
    read: vi.fn().mockResolvedValue(view),
    configure: vi.fn().mockResolvedValue(view),
    start: vi.fn().mockResolvedValue(view),
    stop: vi.fn(),
    sendText: vi.fn(),
  };
}
it('uses the shared saved design editor and keeps its draft across Voice tabs', async () => {
  const view = fixture();
  view.recipeId = 'review';
  const clients = repairClients();
  render(
    <VoiceSessionsScreen
      client={clientFor(view)}
      onOpenSession={vi.fn()}
      workflowEditor={{
        client: clients.authoring,
        executionConfigurationClient: clients.configuration,
      }}
    />,
  );
  const name = await screen.findByRole('textbox', { name: 'Workflow name' });
  fireEvent.change(name, { target: { value: 'Edited voice design' } });
  fireEvent.click(screen.getByRole('button', { name: 'Connection' }));
  expect(screen.queryByLabelText('Working directory')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Workflow' }));
  expect(screen.getByRole('textbox', { name: 'Workflow name' })).toHaveValue('Edited voice design');
});
it('requires an instance and passes the selected identity when connecting', async () => {
  const view = fixture();
  view.instances = [{ id: 'voice-one', name: 'Voice: Instance 1' }];
  const client = clientFor(view);
  render(<VoiceSessionsScreen client={client} onOpenSession={vi.fn()} />);
  const start = await screen.findByRole('button', { name: 'Start connection' });
  expect(start).toBeDisabled();
  fireEvent.change(screen.getByLabelText('Voice instance'), { target: { value: 'voice-one' } });
  expect(start).toBeEnabled();
  fireEvent.click(start);
  await waitFor(() => expect(client.start).toHaveBeenCalledWith(false, 'voice-one'));
});
