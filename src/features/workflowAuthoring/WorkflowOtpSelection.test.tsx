import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { WorkflowDestinationActionPicker } from './WorkflowDestinationActionPicker';
import { WorkflowAuthoringScreen } from './WorkflowAuthoringScreen';
import { otpCatalogue, repairClients } from './testFixtures';
import type { OtpCapabilityRefDto } from '../../application/otp';

it('shares the destination picker for initial requests and saves stop configuration', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  render(
    <WorkflowAuthoringScreen
      client={fixture.authoring}
      readOtpCatalogue={async () => otpCatalogue}
      executionConfigurationClient={fixture.configuration}
    />,
  );
  await user.click(await screen.findByRole('button', { name: 'Expand User request destination' }));
  await user.click(screen.getByRole('button', { name: 'Set action' }));
  const dialog = within(screen.getByRole('dialog'));
  await user.click(dialog.getByRole('button', { name: 'Stop session' }));
  expect(dialog.getByText(/Request cancellation of one/)).toBeVisible();
  await user.click(dialog.getByRole('button', { name: 'Set action' }));
  await user.selectOptions(screen.getByLabelText('Order running sessions'), 'last_addressed');
  await user.click(screen.getByRole('button', { name: 'Save draft' }));
  expect(fixture.states[0].draft.entryAction.tool).toBe('stop_session');
  expect(fixture.states[0].draft.entryConfiguration).toEqual({ ordering: 'last_addressed' });
});
it('does not clear action configuration when confirming the same action or cancelling a preview', async () => {
  const user = userEvent.setup();
  const change = vi.fn();
  function Controlled() {
    const [action, setAction] = useState<OtpCapabilityRefDto>({
      package: 'workflow',
      tool: 'prompt_agent',
    });
    const [configuration, setConfiguration] = useState<Readonly<Record<string, unknown>>>({
      mode: 'new',
    });
    return (
      <WorkflowDestinationActionPicker
        packages={otpCatalogue}
        action={action}
        configuration={configuration}
        onChange={(next, config) => {
          change(next, config);
          setAction(next);
          setConfiguration(config);
        }}
      />
    );
  }
  render(<Controlled />);
  await user.click(screen.getByRole('button', { name: 'Set action' }));
  await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Set action' }));
  expect(change).toHaveBeenLastCalledWith(
    { package: 'workflow', tool: 'prompt_agent' },
    { mode: 'new' },
  );
  await user.click(screen.getByRole('button', { name: 'Set action' }));
  await user.click(screen.getByRole('button', { name: 'Stop session' }));
  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(screen.getByLabelText('Session mode')).toHaveValue('new');
});
