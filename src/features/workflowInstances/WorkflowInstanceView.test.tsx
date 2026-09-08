import { act, fireEvent, render, screen } from '@testing-library/react';
import type {
  WorkflowInstanceClient,
  WorkflowInstanceDetails,
} from '../../application/workflowInstances';
import { repairRecipe } from '../workflowAuthoring/testFixtures';
import { WorkflowInstanceView } from './WorkflowInstanceView';

function details(id: string): WorkflowInstanceDetails {
  const recipe = repairRecipe().draft;
  return {
    instance: {
      id,
      name: `Run ${id}`,
      recipe: {
        ...recipe,
        connections: [
          {
            connectionId: 'review-handoff',
            name: 'Review handoff',
            sourceNodeId: 'author',
            destinationNodeId: 'reviewer',
            trigger: { kind: 'invocation_completed' },
            promptInputs: [{ kind: 'invocation_output' }],
            promptText: '',
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
      createdAt: '2026-09-08',
      target: {
        repository: { id: 'repo', name: 'Repo', gitCommonDirectory: 'C:/repo/.git' },
        branch: { id: 'main', name: 'main' },
        worktree: { id: 'worktree', path: 'C:/repo' },
      },
    },
    sessions: [],
    attempts: [
      {
        id: 'failed',
        instanceId: id,
        definitionRef: { namespace: 'workflow', kind: 'event_definition', id: 'review' },
        workflowElementRef: { namespace: 'workflow', kind: 'connection', id: 'review-handoff' },
        sourceSessionId: 'source',
        createdAt: '2026-09-08T12:00:00Z',
        eventGroup: null,
        error: 'Cannot read prompt file plan.md',
      },
    ],
  };
}

it('keeps stale loads out and opens activity from its connection on the shared graph', async () => {
  let oldReply!: (value: WorkflowInstanceDetails) => void;
  const b = details('b');
  const client: WorkflowInstanceClient = {
    load: vi.fn(async (id) =>
      id === 'a'
        ? new Promise<WorkflowInstanceDetails>((resolve) => {
            oldReply = resolve;
          })
        : b,
    ),
    list: async () => [],
    create: async () => b.instance,
    messageNode: async () => {
      throw new Error('Not used');
    },
  };

  const { rerender } = render(<WorkflowInstanceView key="a" instanceId="a" client={client} />);
  rerender(<WorkflowInstanceView key="b" instanceId="b" client={client} />);
  expect(await screen.findByRole('heading', { name: 'Run b' })).toBeVisible();
  await act(async () => oldReply(details('a')));
  expect(screen.queryByRole('heading', { name: 'Run a' })).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole('button', { name: 'Open Review handoff activity' }));
  expect(screen.getByText('Cannot read prompt file plan.md')).toBeVisible();
});
