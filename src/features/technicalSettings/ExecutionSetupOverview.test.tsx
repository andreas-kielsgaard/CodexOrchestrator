import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import { DeviceSetupOverview, InferenceSourceOverview } from './ExecutionSetupOverview';

const nativeClient = {
  load: vi.fn(async () => ({
    contract: 'native-codex-profile-query/v1' as const,
    profiles: [
      {
        id: 'team',
        homePath: 'C:/codex-team',
        lifecycle: 'active',
        selected: true,
      },
    ],
  })),
} as unknown as NativeProfileClient;

describe('Execution setup overview', () => {
  it('shows local harnesses under the device and keeps the Codex account device-local', async () => {
    render(<DeviceSetupOverview nativeClient={nativeClient} onOpenCodexHarness={() => undefined} />);

    expect(await screen.findByText('C:/codex-team', { exact: false })).toBeVisible();
    expect(screen.getByText(/connected inference source: OpenAI via this Codex CLI/i)).toBeVisible();
    expect(screen.queryByText(/SSH/i)).not.toBeInTheDocument();
  });

  it('shows the harness-to-source binding separately from device configuration', async () => {
    render(<InferenceSourceOverview nativeClient={nativeClient} />);

    expect(await screen.findByText(/Connected to Codex CLI.*C:\/codex-team/i)).toBeVisible();
    expect(screen.getByText(/separate harness connection/i)).toBeVisible();
  });
});
