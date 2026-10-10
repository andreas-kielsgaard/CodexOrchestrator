import { invoke } from '@tauri-apps/api/core';

export interface JevCredentialStatus {
  readonly saved: boolean;
  readonly environmentAvailable: boolean;
}

export interface JevCredentialClient {
  loadStatus(): Promise<JevCredentialStatus>;
  saveKey(key: string): Promise<JevCredentialStatus>;
  removeKey(): Promise<JevCredentialStatus>;
}

export function createJevCredentialClient(
  invokeCommand: typeof invoke = invoke,
): JevCredentialClient {
  return {
    loadStatus: () => invokeCommand('load_jev_credential_status'),
    saveKey: (key) => invokeCommand('save_jev_api_key', { key }),
    removeKey: () => invokeCommand('remove_jev_api_key'),
  };
}

export const tauriJevCredentialClient = createJevCredentialClient();
