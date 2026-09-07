import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type {
  WorkflowInstanceClient,
  WorkflowInstanceDetails,
} from '../../application/workflowInstances';
import { repairRecipe } from './testFixtures';
import { WorkflowInstancePanel } from './WorkflowInstancePanel';

function details(id: string): WorkflowInstanceDetails {
  return {
    instance: {
      id,
      name: `Run ${id}`,
      recipe: repairRecipe().draft,
      createdAt: 'today',
      target: {
        repository: { id: 'repo', name: 'Repo', gitCommonDirectory: 'C:/repo/.git' },
        branch: { id: 'main', name: 'main' },
        worktree: { id: 'worktree', path: 'C:/repo' },
      },
    },
    sessions: [],
    attempts: [],
  };
}

it('ignores results from an old instance and refreshes errors after an attempt is stored', async () => {
  let oldReply!: (value: WorkflowInstanceDetails) => void;
  let changed!: (id: string) => void;
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
    subscribeChanged: async (listener) => {
      changed = listener;
      return () => {};
    },
  };
  const { rerender } = render(<WorkflowInstancePanel key="a" instanceId="a" client={client} />);
  rerender(<WorkflowInstancePanel key="b" instanceId="b" client={client} />);
  expect(await screen.findByRole('heading', { name: 'Run b' })).toBeVisible();
  await act(async () => oldReply(details('a')));
  expect(screen.queryByRole('heading', { name: 'Run a' })).not.toBeInTheDocument();
  const updated = {
    ...b,
    attempts: [
      {
        id: 'failed',
        instanceId: 'b',
        definitionRef: { namespace: 'workflow', kind: 'event_definition', id: 'review' },
        sourceSessionId: 'source',
        createdAt: 'today',
        eventGroup: null,
        error: 'Cannot read prompt file plan.md',
      },
    ],
  };
  vi.mocked(client.load).mockResolvedValue(updated);
  act(() => changed('b'));
  fireEvent.click(screen.getByRole('button', { name: 'Expand Handoffs' }));
  await waitFor(() => expect(screen.getByText('Cannot read prompt file plan.md')).toBeVisible());
});
