import { describe, expect, it, vi } from 'vitest';
import { createTauriEpicPauseRestartController } from './tauriEpicPauseRestart';

describe('Tauri Epic Pause/Restart controller', () => {
  it('uses exact commands and validates returned state', async () => {
    const query = { epicId: 'epic-1', pause: { availability: 'available', reason: 'ready', current: null }, restart: { availability: 'unavailable', reason: 'not interrupted', current: null } };
    const invoke = vi.fn()
      .mockResolvedValueOnce(query)
      .mockResolvedValueOnce({ actionId: 'action-1', kind: 'pause', status: 'completed', targetCount: 1, launchedCount: 1 })
      .mockResolvedValueOnce({ actionId: 'action-2', kind: 'restart', status: 'pending', targetCount: 0, launchedCount: 0 });
    const controller = createTauriEpicPauseRestartController(invoke);
    await expect(controller.load('epic-1')).resolves.toEqual({ epicId: 'epic-1', pause: { availability: 'available', reason: 'ready' }, restart: { availability: 'unavailable', reason: 'not interrupted' } });
    await expect(controller.requestPause('epic-1')).resolves.toMatchObject({ kind: 'pause' });
    await expect(controller.requestRestart('epic-1')).resolves.toMatchObject({ kind: 'restart' });
    expect(invoke.mock.calls).toEqual([
      ['load_epic_pause_restart_query', { input: { epicId: 'epic-1' } }],
      ['request_epic_pause', { input: { epicId: 'epic-1' } }],
      ['request_epic_restart', { input: { epicId: 'epic-1' } }],
    ]);
  });

  it('rejects unknown native results before application state can use them', async () => {
    const invoke = vi.fn().mockResolvedValue({ actionId: 'action-1', kind: 'pause', status: 'completed', targetCount: 1, launchedCount: 1, providerReceipt: true });
    const controller = createTauriEpicPauseRestartController(invoke);
    await expect(controller.requestPause('epic-1')).rejects.toThrow(/invalid shape/);
  });
});
