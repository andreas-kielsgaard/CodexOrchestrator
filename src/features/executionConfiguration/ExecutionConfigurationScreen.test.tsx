import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { CapabilityProfileDto } from '../../application/executionConfiguration';
import { DraftWorkspace } from '../../components/draftWorkspace';
import { repairClients } from '../workflowAuthoring/testFixtures';
import { ExecutionConfigurationScreen } from './ExecutionConfigurationScreen';
import type { CapabilityProfileDraft } from './types';
import type { NativeProfileClient } from '../../infrastructure/agentProviders/codex/profiles/nativeProfileClient';

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

it('saves a registered route and cached model choice while runtime observation fails', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  fixture.configuration.createCapabilityProfile = vi.fn(
    fixture.configuration.createCapabilityProfile,
  );
  fixture.configuration.loadSelectedRuntimeProfile = async () => {
    throw new Error('Codex runtime unavailable');
  };
  fixture.configuration.loadProfileModelCatalogue = async (route) => ({
    route,
    observedAt: '2026-09-21T12:00:00Z',
    models: [
      {
        id: 'model-a',
        label: 'model-a',
        description: '',
        defaultReasoningMode: 'medium',
        reasoningModes: [
          { id: 'medium', description: '' },
          { id: 'high', description: '' },
        ],
      },
    ],
    observationError: 'Codex runtime unavailable',
  });
  render(
    <ExecutionConfigurationScreen
      client={fixture.configuration}
      nativeProfileClient={nativeProfiles}
    />,
  );

  await user.click(await screen.findByRole('button', { name: 'New profile' }));
  await user.click(screen.getByRole('button', { name: 'Add execution route' }));
  expect(screen.getByRole('combobox', { name: 'Harness' })).toHaveValue('local-codex:team');
  await user.click(screen.getByRole('button', { name: 'Add route' }));
  await user.click(
    screen.getByRole('button', { name: 'This device Codex CLI OpenAI account via Codex CLI' }),
  );
  await user.click(screen.getByRole('button', { name: 'Add model' }));
  await user.click(
    within(screen.getByText('model-a').closest('li') as HTMLElement).getByRole('button', {
      name: 'Add',
    }),
  );
  await user.type(
    screen.getByRole('textbox', { name: 'Capability profile name' }),
    'Local profile',
  );
  await user.click(screen.getByRole('button', { name: 'Create profile' }));

  await waitFor(() =>
    expect(fixture.configuration.createCapabilityProfile).toHaveBeenCalledWith(
      expect.objectContaining({
        name: 'Local profile',
        defaultRouteId: expect.any(String),
        routePolicies: [
          expect.objectContaining({
            execution: expect.objectContaining({ configurationRef: 'team' }),
            modelAllowances: [
              expect.objectContaining({
                modelId: 'model-a',
                minimumReasoning: 'medium',
                maximumReasoning: 'high',
              }),
            ],
          }),
        ],
      }),
    ),
  );
});
