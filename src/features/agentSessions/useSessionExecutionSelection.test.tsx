import { act, renderHook, waitFor } from '@testing-library/react';
import type { AgentSessionProfileClient } from '../../application/agentSessions';
import { writeLastSentRuntimeSelection } from './composerDraftCache';
import { useSessionExecutionSelection } from './useSessionExecutionSelection';

afterEach(() => window.localStorage.clear());

it('restores each session choice and seeds a new conversation from the last sent choice', async () => {
  writeLastSentRuntimeSelection({ model: 'model-global', reasoningMode: 'medium' });
  const client = {
    loadPinnedProfile: vi.fn().mockResolvedValue({}),
  } as unknown as AgentSessionProfileClient;
  const { result, rerender } = renderHook(
    ({ sessionId, draftId, repositoryId }: {
      sessionId: string | null;
      draftId: string | undefined;
      repositoryId: string | null;
    }) =>
      useSessionExecutionSelection(
        client,
        sessionId,
        draftId,
        repositoryId ? { kind: 'repository', repositoryId } : null,
      ),
    {
      initialProps: {
        sessionId: null as string | null,
        draftId: 'draft-one' as string | undefined,
        repositoryId: null as string | null,
      },
    },
  );

  await waitFor(() =>
    expect(result.current.selection).toEqual({
      model: 'model-global',
      reasoningMode: 'medium',
    }),
  );
  act(() => {
    result.current.execution?.afterAccepted('session-one', {
      model: 'model-session',
      reasoningMode: 'high',
    });
  });

  rerender({ sessionId: 'session-one', draftId: undefined, repositoryId: null });
  await waitFor(() =>
    expect(result.current.selection).toEqual({
      model: 'model-session',
      reasoningMode: 'high',
    }),
  );

  rerender({ sessionId: null, draftId: 'draft-two', repositoryId: 'another-repo' });
  await waitFor(() =>
    expect(result.current.selection).toEqual({
      model: 'model-session',
      reasoningMode: 'high',
    }),
  );
});
