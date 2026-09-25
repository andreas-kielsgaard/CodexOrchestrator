import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import { DeviceSetupOverview, InferenceSourceOverview } from './ExecutionSetupOverview';

const executionClient = {
  listProviderSetups: async () => [
    {
      deviceId: 'local',
      provider: 'codex',
      configurationId: 'team',
      folder: 'C:/codex-team',
      executable: 'codex',
      state: 'ready' as const,
      detail: null,
      selected: true,
    },
  ],
} as unknown as ExecutionConfigurationClient;

describe('Execution setup overview', () => {
  it('shows provider setups under the device and keeps the account device-local', async () => {
    render(
      <DeviceSetupOverview
        executionClient={executionClient}
        providers={['codex']}
        onConfigureProvider={() => undefined}
      />,
    );

    expect(await screen.findByText('C:/codex-team', { exact: false })).toBeVisible();
    expect(
      screen.getByText(/connected inference source: OpenAI account via Codex CLI/i),
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Configure this harness' })).toBeVisible();
    expect(screen.queryByText(/SSH/i)).not.toBeInTheDocument();
  });

  it('shows the harness-to-source binding separately from device configuration', async () => {
    render(<InferenceSourceOverview executionClient={executionClient} />);

    expect(await screen.findByText(/Connected to Codex CLI.*C:\/codex-team/i)).toBeVisible();
    expect(screen.getByText(/separate harness connection/i)).toBeVisible();
  });
});
