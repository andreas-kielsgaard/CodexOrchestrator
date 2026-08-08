import { invoke } from '@tauri-apps/api/core';
import {
  decodeSprintRunnerTransitionQueryV1,
  type SprintRunnerTransitionQueryV1,
} from '../../application/orchestrations';
type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
export interface SprintRunnerTransitionClient {
  load(): Promise<SprintRunnerTransitionQueryV1>;
}
export function createTauriSprintRunnerTransitionClient(
  invokeCommand: TauriInvoke = invoke,
): SprintRunnerTransitionClient {
  return {
    async load() {
      return decodeSprintRunnerTransitionQueryV1(
        await invokeCommand<unknown>('load_sprint_runner_transition_query'),
      );
    },
  };
}
export const tauriSprintRunnerTransitionClient = createTauriSprintRunnerTransitionClient();
