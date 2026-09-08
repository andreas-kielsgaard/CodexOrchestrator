import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type {
  SessionEventQueryClient,
  SessionEventResultDto,
} from '../../application/sessionEvents';
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
        context: {
          instanceId: 'b',
          occurrenceId: 'call',
          capability: { package: 'workflow', tool: 'prompt_agent' },
          source: null,
          connectionId: 'review',
          outputNodeId: 'reviewer',
        },
        output: null,
        payload: {},
        sessionRequests: [],
        createdAt: 'today',
        eventGroups: [],
        error: 'Cannot read prompt file plan.md',
      },
    ],
  };
  vi.mocked(client.load).mockResolvedValue(updated);
  act(() => changed('b'));
  fireEvent.click(screen.getByRole('button', { name: 'Expand Workflow deliveries' }));
  await waitFor(() => expect(screen.getByText('Cannot read prompt file plan.md')).toBeVisible());
});

it('opens each recorded group in the main delivery inspector and can return to the conversation', async () => {
  const reference = (kind: string, id: string) => ({ namespace: 'workflow', kind, id });
  const eventGroupId = reference('event_group', 'second');
  const recorded: SessionEventResultDto = {
    group: {
      eventGroupId,
      definitionRef: reference('event_definition', 'review'),
      trigger: { kind: 'application_event', event: reference('output', 'continuation') },
      source: { kind: 'application_event', event: reference('output', 'continuation') },
      promptSources: [],
      createdSessionPromptSources: [],
      targetSelection: {
        target: { kind: 'exact', session: reference('session', 'reviewer') },
        cardinality: 'first',
        ordering: 'newest',
        running: 'any',
        createdBy: null,
        missing: 'fail',
      },
      resolvedSessions: [],
      createdSession: null,
      outcome: 'noop',
      deliveryCount: 0,
    },
    deliveries: [],
  };
  const value = details('inspect');
  const loaded: WorkflowInstanceDetails = {
    ...value,
    attempts: [
      {
        id: 'attempt',
        instanceId: 'inspect',
        definitionRef: recorded.group.definitionRef,
        context: {
          instanceId: 'inspect',
          occurrenceId: 'call',
          capability: { package: 'workflow', tool: 'prompt_agent' },
          source: null,
          connectionId: 'review',
          outputNodeId: 'reviewer',
        },
        output: null,
        payload: {},
        sessionRequests: [],
        createdAt: 'today',
        error: null,
        eventGroups: [reference('event_group', 'first'), eventGroupId],
      },
    ],
  };
  const client: WorkflowInstanceClient = {
    load: async () => loaded,
    list: async () => [],
    create: async () => value.instance,
    messageNode: async () => recorded,
  };
  const queryClient: SessionEventQueryClient = {
    loadRecordedEvent: vi.fn(async () => recorded),
    loadEventGroup: async () => recorded.group,
    listDeliveriesForGroup: async () => [],
    listDeliveriesForSession: async () => [],
  };
  render(<WorkflowInstancePanel instanceId="inspect" client={client} queryClient={queryClient} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Expand Workflow deliveries' }));
  fireEvent.click(screen.getByRole('button', { name: 'Show delivery 2' }));
  const inspector = await screen.findByRole('region', { name: 'Workflow delivery details' });
  expect(queryClient.loadRecordedEvent).toHaveBeenCalledWith(eventGroupId);
  expect(inspector.closest('aside')).toBeNull();
  expect(within(inspector).getByText('Exact Session')).toBeVisible();
  fireEvent.click(within(inspector).getByRole('button', { name: 'Close delivery details' }));
  expect(
    screen.queryByRole('region', { name: 'Workflow delivery details' }),
  ).not.toBeInTheDocument();
  expect(screen.getByText('Select a Session to open its conversation.')).toBeVisible();
});
