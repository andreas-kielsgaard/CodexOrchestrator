import { invoke } from '@tauri-apps/api/core';
import {
  decodeEpicBootstrapTransitionQueryV2,
  type EpicBootstrapTransitionQueryV2,
} from '../../application/orchestrations';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export interface EpicBootstrapTransitionClient {
  load(): Promise<EpicBootstrapTransitionQueryV2>;
}

export function createTauriEpicBootstrapTransitionClient(
  invokeCommand: TauriInvoke = invoke,
): EpicBootstrapTransitionClient {
  return {
    async load() {
      return decodeEpicBootstrapTransitionQueryV2(
        await invokeCommand('load_epic_bootstrap_transition_query'),
      );
    },
  };
}

export const tauriEpicBootstrapTransitionClient = createTauriEpicBootstrapTransitionClient();
