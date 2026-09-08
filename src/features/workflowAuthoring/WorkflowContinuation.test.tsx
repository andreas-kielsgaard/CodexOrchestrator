import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { WorkflowTriggerCapabilityDto } from '../../application/workflowAuthoring';
import { WorkflowAuthoringScreen } from './WorkflowAuthoringScreen';
import { repairClients } from './testFixtures';

const capability: WorkflowTriggerCapabilityDto = {
  id: 'workflow.continuation',
  version: 1,
  name: 'Workflow continuation',
  server: 'workflow_handoff',
  tool: 'trigger_workflow_continuation',
  inputSchema: { type: 'object' },
  fields: [
    {
      name: 'outputFiles',
      label: 'Output files',
      schema: { type: 'array', items: { type: 'string' } },
    },
    { name: 'sourceNode', label: 'Source node', schema: { type: 'object' } },
  ],
};

it('discovers continuation fields and saves ordered file selections for multiple nodes', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  fixture.authoring.listTriggerCapabilities = vi.fn(async () => [capability]);
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
          trigger: { kind: 'invocation_completed' },
          promptInputs: [],
          promptText: 'Review these files.',
          target: {
            cardinality: 'first',
            ordering: 'newest',
            running: 'any',
            createdBy: null,
            missing: 'create',
          },
        },
      ],
    },
  };
  const element = (
    <WorkflowAuthoringScreen
      client={fixture.authoring}
      executionConfigurationClient={fixture.configuration}
    />
  );
  const mounted = render(element);
  await user.click(await screen.findByRole('button', { name: 'Edit Review changes' }));
  await user.selectOptions(screen.getByLabelText('Workflow action'), 'workflow.continuation');
  await user.click(screen.getByRole('button', { name: 'Add prompt source' }));
  expect(screen.getByLabelText('Trigger field')).toHaveValue('outputFiles');
  await user.selectOptions(screen.getByLabelText('Trigger field'), 'sourceNode');
  await user.click(screen.getByRole('button', { name: 'Add prompt source' }));
  await user.selectOptions(screen.getAllByLabelText('Source type')[1], 'node_files');
  await user.selectOptions(screen.getByLabelText('Include files from node'), 'author');
  await user.selectOptions(screen.getByLabelText('File association'), 'created');
  await user.click(screen.getByRole('button', { name: 'Add prompt source' }));
  await user.selectOptions(screen.getAllByLabelText('Source type')[2], 'node_files');
  await user.selectOptions(screen.getAllByLabelText('Include files from node')[1], 'reviewer');
  await user.selectOptions(screen.getAllByLabelText('File association')[1], 'edited');
  await user.click(screen.getByRole('button', { name: 'Move prompt source 3 up' }));
  await user.click(screen.getByRole('button', { name: 'Close editor' }));
  await user.click(screen.getByRole('button', { name: 'Save draft' }));
  await waitFor(() => expect(fixture.states[0].draft.revision).toBe(2));
  expect(fixture.states[0].draft.connections[0]).toMatchObject({
    trigger: {
      kind: 'mcp_call',
      server: { id: 'workflow_handoff' },
      tool: { id: 'trigger_workflow_continuation' },
    },
    promptInputs: [
      { kind: 'trigger_field', field: 'sourceNode' },
      { kind: 'node_files', nodeId: 'reviewer', association: 'edited' },
      { kind: 'node_files', nodeId: 'author', association: 'created' },
    ],
  });
  mounted.unmount();
  render(element);
  await user.click(await screen.findByRole('button', { name: 'Edit Review changes' }));
  expect(screen.getByLabelText('Workflow action')).toHaveValue('workflow.continuation');
  expect(screen.getAllByLabelText('Include files from node')[0]).toHaveValue('reviewer');
  expect(screen.getAllByLabelText('File association')[1]).toHaveValue('created');
});
