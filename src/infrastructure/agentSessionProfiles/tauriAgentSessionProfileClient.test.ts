import { vi } from 'vitest';
import { createTauriAgentSessionProfileClient } from './tauriAgentSessionProfileClient';

describe('Tauri Agent Session Profile client', () => {
  it('keeps pinned-profile reads separate from message-local runtime choices', async () => {
    const invoke = vi.fn().mockResolvedValue(null);
    const client = createTauriAgentSessionProfileClient(invoke);

    await client.loadPinnedProfile('session-1');
    await client.sendDirectUserMessage({
      sessionId: 'session-1',
      submittedText: 'Continue the review.',
      model: 'codex-a',
      reasoningMode: 'high',
    });

    expect(invoke.mock.calls).toEqual([
      ['load_pinned_agent_session_profile', { input: { sessionId: 'session-1' } }],
      [
        'send_direct_user_agent_session_message',
        {
          input: {
            sessionId: 'session-1',
            submittedText: 'Continue the review.',
            model: 'codex-a',
            reasoningMode: 'high',
          },
        },
      ],
    ]);
  });
});
