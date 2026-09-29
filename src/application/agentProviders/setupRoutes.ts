import { agentProviderDescriptor } from './descriptors';
import type { HarnessInferenceRouteOption, ProviderSetupDto } from './contracts';

/** Removes the Windows extended-path transport prefix from user-facing folder labels. */
export function displayFolderPath(path: string): string {
  return path.replace(/^\\\\\?\\/, '');
}

/** Routes a Capability Profile can add: every provider setup on every device. */
export function providerSetupRoutes(
  setups: readonly ProviderSetupDto[],
): readonly HarnessInferenceRouteOption[] {
  return setups.map((setup) => {
    const descriptor = agentProviderDescriptor(setup.provider);
    const deviceLabel = setup.deviceId === 'local' ? 'This device' : setup.deviceId;
    const folder = displayFolderPath(setup.folder);
    return {
      id: `${setup.deviceId}/${setup.provider}/${setup.configurationId}`,
      selected: setup.selected,
      label: `${deviceLabel} · ${descriptor.harnessLabel} · ${folder}${setup.selected ? ' · selected' : ''}`,
      sourceLabel: descriptor.inferenceLabel,
      deviceLabel,
      harnessLabel: descriptor.harnessLabel,
      inferenceLabel: descriptor.inferenceLabel,
      detail: `${folder} · account configuration stays in this ${descriptor.configurationLabel}`,
      execution: {
        deviceId: setup.deviceId,
        deviceName: deviceLabel,
        provider: setup.provider,
        configurationRef: setup.configurationId,
        connection: { kind: 'local' as const },
      },
    };
  });
}
