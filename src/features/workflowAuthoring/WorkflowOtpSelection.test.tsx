import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { WorkflowDestinationActionPicker } from './WorkflowDestinationActionPicker';
import { otpCatalogue } from './testFixtures';
import type { OtpCapabilityRefDto } from '../../application/otp';

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
