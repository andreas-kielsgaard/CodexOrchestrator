import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WorkflowTriggerPicker } from './WorkflowTriggerPicker';
import { otpCatalogue } from './testFixtures';

it('shows every OTP package and leaves empty trigger packages non-collapsible', async () => {
  const user = userEvent.setup();
  render(
    <WorkflowTriggerPicker
      packages={otpCatalogue}
      connection={{
        connectionId: 'connection',
        name: 'Connection',
        sourceNodeId: 'source',
        destinationNodeId: 'destination',
        trigger: {
          capability: { package: 'workflow', tool: 'on_invocation_completed' },
          output: 'completed',
        },
        promptInputs: [],
        promptText: '',
        action: { package: 'workflow', tool: 'prompt_agent' },
        configuration: {},
      }}
      onChange={vi.fn()}
    />,
  );
  await user.click(screen.getByRole('button', { name: 'Set trigger' }));
  expect(screen.getByText('Job Agent')).toBeVisible();
  expect(screen.getByText('No triggers')).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Job Agent' })).toBeNull();
});
