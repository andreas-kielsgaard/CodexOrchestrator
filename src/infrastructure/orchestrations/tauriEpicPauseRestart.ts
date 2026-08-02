import { invoke } from '@tauri-apps/api/core';
import { decodeEpicPauseRestartQuery, type EpicPauseRestartController, type EpicPauseRestartOutcome } from '../../application/orchestrations';

export function createTauriEpicPauseRestartController(
  invokeCommand: typeof invoke = invoke,
): EpicPauseRestartController {
  const request = (command: string, epicId: string) =>
    invokeCommand<EpicPauseRestartOutcome>(command, { input: { epicId } });
  return {
    async load(epicId) {
      return decodeEpicPauseRestartQuery(
        await invokeCommand<unknown>('load_epic_pause_restart_query', { input: { epicId } }),
      );
    },
    requestPause: (epicId) => request('request_epic_pause', epicId),
    requestRestart: (epicId) => request('request_epic_restart', epicId),
  };
}
