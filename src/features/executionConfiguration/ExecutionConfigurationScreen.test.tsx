import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { CapabilityProfileDto } from '../../application/executionConfiguration';
import { DraftWorkspace } from '../../components/draftWorkspace';
import { repairClients } from '../workflowAuthoring/testFixtures';
import { ExecutionConfigurationScreen } from './ExecutionConfigurationScreen';
import type { CapabilityProfileDraft } from './types';
import type { NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';

const nativeProfiles = {
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

it('preserves typing during a save, even after selecting another profile', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  let finish!: (value: CapabilityProfileDto) => void;
  fixture.configuration.updateCapabilityProfile = vi.fn(
    () =>
      new Promise<CapabilityProfileDto>((resolve) => {
        finish = resolve;
      }),
  );
  render(<ExecutionConfigurationScreen client={fixture.configuration} />);
  await user.click(await screen.findByRole('button', { name: /Review capabilities Revision 1/ }));
  const name = screen.getByRole('textbox', { name: 'Capability profile name' });
  fireEvent.change(name, { target: { value: 'Submitted' } });
  await user.click(screen.getByRole('button', { name: 'Save new revision' }));
  fireEvent.change(name, { target: { value: 'Later typing' } });
  await user.click(screen.getByRole('button', { name: /Other capabilities Revision 1/ }));
  await act(async () => finish({ ...fixture.profiles[0], name: 'Submitted', revision: 2 }));
  expect(screen.getByRole('textbox', { name: 'Capability profile name' })).toHaveValue(
    'Other capabilities',
  );
  await user.click(screen.getByRole('button', { name: /Review capabilities Revision 1/ }));
  expect(screen.getByRole('textbox', { name: 'Capability profile name' })).toHaveValue(
    'Later typing',
  );
  expect(screen.getByText('Revision 2')).toBeInTheDocument();
});

it('retains a new unsaved profile across remount and failed save', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  const workspace = new DraftWorkspace<CapabilityProfileDraft>();
  fixture.configuration.createCapabilityProfile = vi.fn(async () => {
    throw new Error('Save failed');
  });
  const element = (
    <ExecutionConfigurationScreen
      client={fixture.configuration}
      workspace={workspace}
      nativeProfileClient={nativeProfiles}
    />
  );
  const mounted = render(element);
  await screen.findByRole('button', { name: 'New profile' });
  await user.click(screen.getByRole('button', { name: 'New profile' }));
  fireEvent.change(screen.getByRole('textbox', { name: 'Capability profile name' }), {
    target: { value: 'Unfinished' },
  });
  await user.click(screen.getByRole('button', { name: 'Add execution route' }));
  await user.click(screen.getByRole('button', { name: 'Add route' }));
  await user.click(screen.getByRole('button', { name: 'Create profile' }));
  await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('Save failed'));
  mounted.unmount();
  render(element);
  expect(await screen.findByRole('textbox', { name: 'Capability profile name' })).toHaveValue(
    'Unfinished',
  );
  expect(screen.queryByRole('textbox', { name: 'Capability profile ID' })).not.toBeInTheDocument();
});

it('projects each active local Codex home as a profile route without exposing credentials', async () => {
  const fixture = repairClients();
  render(
    <ExecutionConfigurationScreen
      client={fixture.configuration}
      nativeProfileClient={nativeProfiles}
    />,
  );

  await screen.findByRole('button', { name: 'New profile' });
  const user = userEvent.setup();
  await user.click(screen.getByRole('button', { name: 'New profile' }));
  await user.click(screen.getByRole('button', { name: 'Add execution route' }));
  expect(screen.getByRole('combobox', { name: 'Harness' })).toHaveValue('local-codex:team');
  expect(screen.getByText('OpenAI account via Codex CLI')).toBeVisible();
  expect(screen.queryByText(/SSH/i)).not.toBeInTheDocument();
});
