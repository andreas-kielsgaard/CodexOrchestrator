import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { VoiceSessionsScreen } from './VoiceSessionsScreen';
import type { VoiceView } from '../../application/voiceSessions';

it('distinguishes a pending amendment from an outcome for the earlier request', async () => {
  const prompts = {
    relevance: '',
    completion: '',
    continuation: '',
    reaction: '',
    summary: '',
    router: '',
    digest: '',
    output: '',
    tools: '',
  };
  const view: VoiceView = {
    defaultPrompts: prompts,
    profiles: [],
    state: {
      settings: {
        bindAddress: '',
        token: '',
        profileId: 'profile',
        workingDirectory: null,
        instructions: '',
        prompts,
        models: {
          response: { model: 'gpt-6-luna', reasoningMode: 'low', fastMode: true },
          routing: { model: 'gpt-6.1-sol', reasoningMode: 'medium', fastMode: false },
        },
      },
      enabled: true,
      phoneConnected: false,
      speechConfigured: true,
      error: null,
      conversation: {
        id: 'voice',
        mainSessionId: 'router',
        pendingTopics: [],
        utterances: [],
        inputs: [],
        playback: null,
        tasks: [
          {
            id: 'picnic',
            title: 'Picnic plan',
            revision: 2,
            request: 'Make the ideas vegan',
            updatedAt: '',
          },
        ],
        delegations: [
          {
            id: 'first',
            taskId: 'picnic',
            revision: 1,
            sessionId: 'worker',
            task: 'Vegetarian ideas',
            invocationId: 'turn',
            outcome: {
              summary: 'Three vegetarian ideas',
              revision: 1,
              materialRefs: [],
              unresolved: [],
            },
          },
          {
            id: 'amendment',
            taskId: 'picnic',
            revision: 2,
            sessionId: 'worker',
            task: 'Make the ideas vegan',
            invocationId: 'turn',
            runtimeStatus: 'running',
            delivery: { inputId: 'steer', state: 'queued' },
          },
        ],
      },
    },
  };
  render(
    <VoiceSessionsScreen
      client={{
        read: vi.fn().mockResolvedValue(view),
        configure: vi.fn(),
        start: vi.fn(),
        stop: vi.fn(),
        sendText: vi.fn(),
      }}
      onOpenSession={vi.fn()}
    />,
  );
  fireEvent.click(await screen.findByRole('button', { name: 'Activity' }));
  expect(await screen.findByText('Picnic plan')).toBeVisible();
  expect(screen.getByText(/Request 2 · running · amendment queued/)).toBeVisible();
  expect(screen.getByText('Earlier outcome · request 1')).toBeVisible();
});

it('shows held input, actual microphone fallback, and a failed reasoning stage', async () => {
  const prompts = {
    relevance: '',
    completion: '',
    continuation: '',
    reaction: '',
    summary: '',
    router: '',
    digest: '',
    output: '',
    tools: '',
  };
  const view: VoiceView = {
    defaultPrompts: prompts,
    profiles: [],
    state: {
      settings: {
        bindAddress: '',
        token: '',
        profileId: 'profile',
        workingDirectory: null,
        instructions: '',
        prompts,
        models: {
          response: { model: 'gpt-6-luna', reasoningMode: 'low', fastMode: true },
          routing: { model: 'gpt-6.1-sol', reasoningMode: 'medium', fastMode: false },
        },
      },
      enabled: true,
      phoneConnected: true,
      speechConfigured: true,
      error: null,
      conversation: null,
    },
    diagnostics: {
      updatedAt: new Date().toISOString(),
      phoneConnected: true,
      pendingAudio: 0,
      error: null,
      capture: {
        recording: true,
        requestedMicrophone: 'headset',
        routedMicrophone: 'Phone: built-in microphone',
        external: false,
        silenced: false,
        level: 0.01,
        framesRead: 16000,
        segmentsSent: 1,
        error: null,
        updatedAt: new Date().toISOString(),
      },
      audio: {
        received: 1,
        transcribed: 1,
        lastTranscript: 'Orchid, can you hear me?',
        lastReceivedAt: null,
        lastTranscribedAt: null,
        error: null,
      },
      input: {
        title: 'Waiting for you to finish',
        detail: 'JEV thinks more is coming.',
        last: {
          text: 'Orchid, can you hear me?',
          relevance: 'direct_request',
          completion: 'continue',
          filterPending: false,
          createdAt: '',
        },
      },
      stages: [
        {
          role: 'router',
          sessionId: 'router',
          status: 'failed',
          queued: 0,
          error: 'Pinned skill changed',
        },
        { role: 'output', sessionId: 'output', status: null, queued: 0, error: null },
      ],
    },
  };
  render(
    <VoiceSessionsScreen
      client={{
        read: vi.fn().mockResolvedValue(view),
        configure: vi.fn(),
        start: vi.fn(),
        stop: vi.fn(),
        sendText: vi.fn(),
      }}
      onOpenSession={vi.fn()}
    />,
  );
  fireEvent.click(await screen.findByRole('button', { name: 'Activity' }));
  expect(await screen.findByText('Waiting for you to finish')).toBeVisible();
  expect(screen.getByText(/external requested, phone fallback/)).toBeVisible();
  expect(screen.getByText('Pinned skill changed')).toBeVisible();
  expect(screen.getByRole('button', { name: 'output · not started' })).toBeVisible();
  expect(screen.queryByText('Waiting for input')).toBeNull();
});
