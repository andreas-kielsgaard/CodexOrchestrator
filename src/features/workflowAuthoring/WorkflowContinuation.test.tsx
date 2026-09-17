import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WorkflowAuthoringScreen } from './WorkflowAuthoringScreen';
import { otpCatalogue, repairClients } from './testFixtures';

it('discovers continuation fields and saves ordered file selections for multiple nodes', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  fixture.states[0] = {
    ...fixture.states[0],
    draft: {
      ...fixture.states[0].draft,
      connections: [
        {
          connectionId: 'review',
          name: 'Review changes',
          sourceNodeId: 'author',
          destinationNodeId: 'reviewer',
          trigger: {
            capability: { package: 'workflow', tool: 'on_invocation_completed' },
            output: 'completed',
          },
          promptInputs: [],
          promptText: 'Review these files.',
          action: { package: 'workflow', tool: 'prompt_agent' },
          configuration: {},
        },
      ],
    },
  };
  const element = (
    <WorkflowAuthoringScreen
      client={fixture.authoring}
      readOtpCatalogue={async () => otpCatalogue}
      executionConfigurationClient={fixture.configuration}
    />
  );
  const mounted = render(element);
  await user.click(await screen.findByRole('button', { name: 'Edit Review changes' }));
  await user.click(screen.getByRole('button', { name: 'Set trigger' }));
  await user.click(screen.getByRole('button', { name: 'Workflow continuation · Continuation' }));
  await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Set trigger' }));
  await user.click(screen.getByRole('button', { name: 'Add prompt source' }));
  await user.selectOptions(screen.getByLabelText('Output field'), 'outputFiles');
  expect(screen.getByLabelText('Output field')).toHaveValue('outputFiles');
  await user.selectOptions(screen.getByLabelText('Output field'), 'sourceNode');
  await user.click(screen.getByRole('button', { name: 'Add prompt source' }));
  await user.selectOptions(screen.getAllByLabelText('Source type')[1], 'node_files');
  await user.selectOptions(screen.getByLabelText('Include files from node'), 'author');
  await user.selectOptions(screen.getByLabelText('File association'), 'created');
  await user.click(screen.getByRole('button', { name: 'Add prompt source' }));
  await user.selectOptions(screen.getAllByLabelText('Source type')[2], 'node_files');
  await user.selectOptions(screen.getAllByLabelText('Include files from node')[1], 'reviewer');
  await user.selectOptions(screen.getAllByLabelText('File association')[1], 'edited');
  await user.click(screen.getByRole('button', { name: 'Move prompt source 3 up' }));
  await user.selectOptions(
    within(screen.getByRole('region', { name: 'Selected flow element' })).getByLabelText(
      'Session mode',
    ),
    'new',
  );
  expect(
    within(screen.getByRole('region', { name: 'Selected flow element' })).queryByLabelText(
      'Sessions to select',
    ),
  ).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Close editor' }));
  await user.click(screen.getByRole('button', { name: 'Save draft' }));
  await waitFor(() => expect(fixture.states[0].draft.revision).toBe(2));
  expect(fixture.states[0].draft.connections[0]).toMatchObject({
    trigger: {
      capability: { package: 'workflow', tool: 'trigger_workflow_continuation' },
      output: 'continuation',
    },
    configuration: { mode: 'new' },
    promptInputs: [
      { kind: 'output_field', field: 'sourceNode' },
      { kind: 'node_files', nodeId: 'reviewer', association: 'edited' },
      { kind: 'node_files', nodeId: 'author', association: 'created' },
    ],
  });
  mounted.unmount();
  render(element);
  await user.click(await screen.findByRole('button', { name: 'Edit Review changes' }));
  expect(screen.getByText('Workflow continuation · Continuation')).toBeVisible();
  expect(
    within(screen.getByRole('region', { name: 'Selected flow element' })).getByLabelText(
      'Session mode',
    ),
  ).toHaveValue('new');
  expect(screen.getAllByLabelText('Include files from node')[0]).toHaveValue('reviewer');
  expect(screen.getAllByLabelText('File association')[1]).toHaveValue('created');
});
