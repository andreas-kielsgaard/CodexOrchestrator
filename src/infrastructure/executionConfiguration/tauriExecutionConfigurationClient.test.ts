import { vi } from 'vitest';
import type { CapabilitySetDto } from '../../application/executionConfiguration';
import { createTauriExecutionConfigurationClient } from './tauriExecutionConfigurationClient';

describe('Tauri Execution Configuration client', () => {
  it('maps the explicit runtime and Capability Profile operations to their commands', async () => {
    const invoke = vi.fn().mockResolvedValue(null);
    const client = createTauriExecutionConfigurationClient(invoke);
    const allowedCapabilities: CapabilitySetDto = {
      models: ['codex-a'],
      reasoningModes: ['high'],
      sandboxModes: ['workspace_write'],
      mcpTools: { orchestrator: ['session_events'] },
      skills: ['review'],
    };

    await client.loadSelectedRuntimeProfile();
    await client.listCapabilityProfiles();
    await client.loadCapabilityProfile('review');
    await client.createCapabilityProfile({
      capabilityProfileId: 'review',
      name: 'Review',
      allowedCapabilities,
    });
    await client.updateCapabilityProfile({
      capabilityProfileId: 'review',
      name: 'Architecture review',
      allowedCapabilities,
    });
    await client.deleteCapabilityProfile('review');

    expect(invoke.mock.calls).toEqual([
      ['load_selected_runtime_profile'],
      ['list_capability_profiles'],
      ['load_capability_profile', { input: { capabilityProfileId: 'review' } }],
      [
        'create_capability_profile',
        { input: { capabilityProfileId: 'review', name: 'Review', allowedCapabilities } },
      ],
      [
        'update_capability_profile',
        {
          input: {
            capabilityProfileId: 'review',
            name: 'Architecture review',
            allowedCapabilities,
          },
        },
      ],
      ['delete_capability_profile', { input: { capabilityProfileId: 'review' } }],
    ]);
  });
});
