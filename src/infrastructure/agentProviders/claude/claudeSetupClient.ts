import { invoke } from '@tauri-apps/api/core';
import type { ProviderSetupDto } from '../../../application/agentProviders';

/** Claude setups on this device: a Claude configuration folder and the CLI that uses it. */
export interface ClaudeSetupClient {
  listSetups(): Promise<readonly ProviderSetupDto[]>;
  addSetup(folder: string, executable?: string): Promise<void>;
  removeSetup(setupId: string): Promise<void>;
}

export type ClaudeSetupInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createClaudeSetupClient(
  invokeCommand: ClaudeSetupInvoke = invoke,
): ClaudeSetupClient {
  return {
    listSetups: async () =>
      (await invokeCommand<ProviderSetupDto[]>('list_provider_setups')).filter(
        (setup) => setup.provider === 'claude',
      ),
    addSetup: async (folder, executable) => {
      await invokeCommand('add_claude_setup', {
        input: { folder, executable: executable?.trim() || null },
      });
    },
    removeSetup: async (setupId) => {
      await invokeCommand('remove_claude_setup', { setupId });
    },
  };
}

export const tauriClaudeSetupClient = createClaudeSetupClient();
