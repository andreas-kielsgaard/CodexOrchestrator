import { invoke } from '@tauri-apps/api/core';

export interface AppMetadata {
  appName: string;
  storageMode: 'local-first';
  codexRuntime: 'adapter-pending';
}

export async function getAppMetadata(): Promise<AppMetadata> {
  return invoke<AppMetadata>('app_metadata');
}
