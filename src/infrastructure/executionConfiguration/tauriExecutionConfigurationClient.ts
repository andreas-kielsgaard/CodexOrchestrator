import { invoke } from '@tauri-apps/api/core';
import type {
  CapabilityProfileDto,
  ExecutionConfigurationClient,
  RuntimeProfileSnapshotDto,
} from '../../application/executionConfiguration';

export type ExecutionConfigurationInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

export function createTauriExecutionConfigurationClient(
  invokeCommand: ExecutionConfigurationInvoke = invoke,
): ExecutionConfigurationClient {
  return {
    loadSelectedRuntimeProfile: () =>
      invokeCommand<RuntimeProfileSnapshotDto>('load_selected_runtime_profile'),
    listCapabilityProfiles: () => invokeCommand<CapabilityProfileDto[]>('list_capability_profiles'),
    loadCapabilityProfile: (capabilityProfileId) =>
      invokeCommand<CapabilityProfileDto>('load_capability_profile', {
        input: { capabilityProfileId },
      }),
    createCapabilityProfile: (input) =>
      invokeCommand<CapabilityProfileDto>('create_capability_profile', { input }),
    updateCapabilityProfile: (input) =>
      invokeCommand<CapabilityProfileDto>('update_capability_profile', { input }),
    deleteCapabilityProfile: (capabilityProfileId) =>
      invokeCommand<void>('delete_capability_profile', {
        input: { capabilityProfileId },
      }),
  };
}

export const tauriExecutionConfigurationClient = createTauriExecutionConfigurationClient();
