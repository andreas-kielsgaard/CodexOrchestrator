import { fireEvent, render, screen } from '@testing-library/react';
import type { OtpPackageDto, WorkflowCapabilityNodeDto } from '../../application/workflowAuthoring';
import { WorkflowCapabilityEditor } from './WorkflowCapabilityEditor';
import { offeredActions, offeredOutputs } from './otpPresentation';

const node: WorkflowCapabilityNodeDto = {
  kind: 'capability',
  nodeId: 'renamed-buffer',
  name: 'Input gate',
  positionX: 0,
  positionY: 0,
  capability: { package: 'voice', tool: 'buffer' },
  configuration: { directReactions: true, prompts: { completion: 'Original question' } },
};
const packages: readonly OtpPackageDto[] = [
  {
    id: 'voice',
    name: 'Voice',
    summary: '',
    description: '',
    contractVersion: 1,
    requestedHandles: [],
    agentMcpServers: [],
    nodeCapabilities: [
      {
        id: 'buffer',
        name: 'Request buffer',
        configurationSchema: {
          type: 'object',
          properties: {
            directReactions: { type: 'boolean', title: 'Direct reactions' },
            prompts: {
              type: 'object',
              properties: { completion: { type: 'string', title: 'Completion question' } },
            },
          },
        },
        actions: [{ package: 'voice', tool: 'accept' }],
        outputs: [{ capability: { package: 'voice', tool: 'accept' }, output: 'ready' }],
      },
    ],
    tools: ['accept', 'unrelated'].map((id) => ({
      id,
      name: id,
      description: '',
      expectedBehavior: '',
      recommendedUsage: '',
      configuration: [],
      entrypoint: { kind: 'action', usesPrompt: false },
      outputs: [{ id: 'ready', name: 'Ready', kind: 'data', schema: { type: 'object' } }],
    })),
  },
];

it('edits capability settings without agent profiles and preserves other configuration', () => {
  const change = vi.fn();
  render(<WorkflowCapabilityEditor node={node} packages={packages} onChange={change} />);
  fireEvent.click(screen.getByRole('checkbox', { name: 'Direct reactions' }));
  expect(change.mock.lastCall?.[0]).toEqual({
    ...node,
    configuration: { ...node.configuration, directReactions: false },
  });
  fireEvent.change(screen.getByRole('textbox', { name: 'Completion question' }), {
    target: { value: 'Revised question' },
  });
  expect(change.mock.lastCall?.[0].configuration).toEqual({
    directReactions: true,
    prompts: { completion: 'Revised question' },
  });
});

it('offers only the bound capability actions and outputs, including action completion outputs', () => {
  expect(offeredActions(packages, node).map((item) => item.ref.tool)).toEqual(['accept']);
  expect(offeredOutputs(packages, node).map((item) => item.ref.capability.tool)).toEqual([
    'accept',
  ]);
  expect(offeredOutputs(packages)).toEqual([]);
  expect(offeredActions(packages)).toEqual([]);
});
