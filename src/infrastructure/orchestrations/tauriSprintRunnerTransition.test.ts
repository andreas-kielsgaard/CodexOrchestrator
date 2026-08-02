import { describe, expect, it } from 'vitest';
import { createTauriSprintRunnerTransitionClient } from './tauriSprintRunnerTransition';

describe('Tauri Sprint Runner transition client', () => {
  it('loads only the registered native query through its strict decoder', async () => {
    const client = createTauriSprintRunnerTransitionClient(async <T>(command: string) => {
      expect(command).toBe('load_sprint_runner_transition_query');
      return {
        contract: 'sprint-runner-transition-query/v1',
        transitions: [],
      } as T;
    });
    await expect(client.load()).resolves.toMatchObject({
      contract: 'sprint-runner-transition-query/v1',
    });
  });
});
