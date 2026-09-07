import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { CapabilityProfileDto } from '../../application/executionConfiguration';
import { DraftWorkspace } from '../../components/draftWorkspace';
import { repairClients } from '../workflowAuthoring/testFixtures';
import { ExecutionConfigurationScreen } from './ExecutionConfigurationScreen';
import type { CapabilityProfileDraft } from './types';

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
  const name = await screen.findByRole('textbox', { name: 'Capability profile name' });
  fireEvent.change(name, { target: { value: 'Submitted' } });
  await user.click(screen.getByRole('button', { name: 'Save new revision' }));
  fireEvent.change(name, { target: { value: 'Later typing' } });
  await user.click(screen.getByRole('button', { name: /Other capabilities other/ }));
  await act(async () => finish({ ...fixture.profiles[0], name: 'Submitted', revision: 2 }));
  expect(screen.getByRole('textbox', { name: 'Capability profile name' })).toHaveValue(
    'Other capabilities',
  );
  await user.click(screen.getByRole('button', { name: /Review capabilities review/ }));
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
    <ExecutionConfigurationScreen client={fixture.configuration} workspace={workspace} />
  );
  const mounted = render(element);
  await screen.findByRole('textbox', { name: 'Capability profile name' });
  await user.click(screen.getByRole('button', { name: 'New profile' }));
  fireEvent.change(screen.getByRole('textbox', { name: 'Capability profile name' }), {
    target: { value: 'Unfinished' },
  });
  fireEvent.change(screen.getByRole('textbox', { name: 'Capability profile ID' }), {
    target: { value: 'unfinished' },
  });
  await user.click(screen.getByRole('button', { name: 'Create profile' }));
  await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('Save failed'));
  mounted.unmount();
  render(element);
  expect(await screen.findByRole('textbox', { name: 'Capability profile name' })).toHaveValue(
    'Unfinished',
  );
  expect(screen.getByRole('textbox', { name: 'Capability profile ID' })).toBeEnabled();
});
