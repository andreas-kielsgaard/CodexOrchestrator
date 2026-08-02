import { describe, expect, it, vi } from 'vitest';
import { createTauriEpicPauseRestartController } from './tauriEpicPauseRestart';

describe('Tauri Epic Pause/Restart controller', () => {
  it('uses exact commands and validates returned state', async () => {
    const invoke = vi.fn().mockResolvedValue({ actionId: 'action-1', kind: 'pause', status: 'completed', targetCount: 1, launchedCount: 1 });
    const controller = createTauriEpicPauseRestartController(invoke);
    await expect(controller.requestPause('epic-1')).resolves.toMatchObject({ kind: 'pause' });
    expect(invoke).toHaveBeenCalledWith('request_epic_pause', { input: { epicId: 'epic-1' } });
  });
});
