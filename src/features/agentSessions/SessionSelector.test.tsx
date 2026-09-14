import { within, createEvent, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { buildSessionNavigation } from '../../application/agentSessions/navigation';
import { navigationData, populatedNavigationData } from './navigationTestFixtures';
import { SessionSelector as Selector } from './SessionSelector';
import { useSessionNavigation } from './useSessionNavigation';
import type { ComponentProps } from 'react';
function SessionSelector(
  props: Omit<ComponentProps<typeof Selector>, 'tree'> & { revealKey: string },
) {
  const tree = useSessionNavigation(props.model, props.selectedSessionId, props.revealKey);
  return <Selector {...props} tree={tree} />;
}
function dragTo(source: HTMLElement, target: HTMLElement, y = -1) {
  source.setPointerCapture = vi.fn();
  source.hasPointerCapture = () => true;
  source.releasePointerCapture = vi.fn();
  Object.defineProperty(document, 'elementFromPoint', { configurable: true, value: () => target });
  for (const [kind, clientY] of [
    ['pointerDown', 30],
    ['pointerMove', y],
    ['pointerUp', y],
  ] as const) {
    const event = createEvent[kind](source);
    Object.defineProperties(event, {
      pointerId: { value: 1 },
      button: { value: 0 },
      clientX: { value: 100 },
      clientY: { value: clientY },
    });
    fireEvent(source, event);
  }
}
const props = () => ({
  model: buildSessionNavigation(navigationData()),
  selectedSessionId: null,
  revealKey: 'initial',
  loading: false,
  onSelect: vi.fn(),
  onNew: vi.fn(),
  onMove: vi.fn().mockResolvedValue(undefined),
  onReorder: vi.fn().mockResolvedValue(undefined),
  onPin: vi.fn().mockResolvedValue(undefined),
  onReload: vi.fn(),
});
it('shows five sessions per folder, reveals more, and creates at a folder target', async () => {
  const actions = props();
  render(<SessionSelector {...actions} />);
  expect(await screen.findByRole('treeitem', { name: 'Session 5' })).toBeVisible();
  expect(screen.queryByRole('treeitem', { name: 'Session 6' })).toBeNull();
  fireEvent.click(screen.getByRole('treeitem', { name: 'Show more in Sessions' }));
  expect(await screen.findByRole('treeitem', { name: 'Session 6' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'New session in Feature build' }));
  expect(actions.onNew).toHaveBeenCalledWith({ kind: 'workflow_instance', instanceId: 'flow-a' });
});
it('reveals a hidden selected session without expanding all folders on later refresh', async () => {
  const actions = props();
  const { rerender } = render(
    <SessionSelector {...actions} selectedSessionId="session-6" revealKey="link-1" />,
  );
  expect(await screen.findByRole('treeitem', { name: 'Session 6' })).toBeVisible();
  fireEvent.click(screen.getByRole('treeitem', { name: 'Alpha' }));
  rerender(
    <SessionSelector
      {...actions}
      model={buildSessionNavigation(navigationData())}
      selectedSessionId="session-6"
      revealKey="link-1"
    />,
  );
  expect(screen.queryByRole('treeitem', { name: 'Session 6' })).toBeNull();
});
it('routes dragging and the context-menu move to the same placement operation', async () => {
  const actions = props();
  render(<SessionSelector {...actions} />);
  const row = await screen.findByRole('treeitem', { name: 'Session 1' });
  dragTo(row, screen.getByRole('treeitem', { name: 'Feature build' }));
  expect(actions.onMove).toHaveBeenCalledWith(
    'session-1',
    {
      kind: 'workflow_instance',
      instanceId: 'flow-a',
    },
    ['session-1', 'session-7'],
  );
  fireEvent.contextMenu(row, { clientX: 30, clientY: 40 });
  fireEvent.click(screen.getByRole('menuitem', { name: 'Move to…' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Empty repo' }));
  expect(actions.onMove).toHaveBeenCalledWith('session-1', {
    kind: 'repository',
    repositoryId: 'repo-b',
  });
  fireEvent.contextMenu(row);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Pin' }));
  await waitFor(() => expect(actions.onPin).toHaveBeenCalledWith('session-1', true));
});

it('shares a five-row budget across neutral ownership groups and collapses only the nearest block', async () => {
  const actions = props();
  render(
    <SessionSelector {...actions} model={buildSessionNavigation(populatedNavigationData())} />,
  );
  const workflow = await screen.findByRole('treeitem', { name: 'Feature build' });
  const container = workflow.parentElement!;
  expect(container.querySelectorAll('.session-entry')).toHaveLength(5);
  expect(container.querySelectorAll('.session-owner-group')).toHaveLength(2);
  expect(
    screen.getByRole('tree', { name: 'Pinned sessions' }).querySelectorAll('.session-entry'),
  ).toHaveLength(6);
  fireEvent.click(screen.getByRole('treeitem', { name: 'Discussion 9' }));
  expect(workflow).toHaveAttribute('aria-expanded', 'true');
  fireEvent.click(screen.getByRole('treeitem', { name: 'Show more in Feature build' }));
  expect(container.querySelectorAll('.session-entry')).toHaveLength(8);
  fireEvent.click(container.querySelector('.session-block-content')!);
  expect(workflow).toHaveAttribute('aria-expanded', 'false');
  expect(screen.getAllByRole('treeitem', { name: 'Workflows' })[0]).toHaveAttribute(
    'aria-expanded',
    'true',
  );
  fireEvent.click(workflow);
  fireEvent.click(screen.getAllByRole('treeitem', { name: 'Sessions' })[0]);
  expect(screen.getAllByRole('treeitem', { name: 'Sessions' })[0]).toHaveTextContent('Sessions…');
});

it('toggles pins inline without selecting a row or collapsing its block', async () => {
  const actions = props();
  const data = populatedNavigationData();
  const { rerender } = render(
    <SessionSelector {...actions} model={buildSessionNavigation(data)} />,
  );
  const pinned = (await screen.findAllByRole('button', { name: 'Unpin Discussion 1' }))[0];
  fireEvent.focus(pinned);
  fireEvent.click(pinned);
  expect(actions.onPin).toHaveBeenCalledWith('demo-1', false);
  expect(actions.onSelect).not.toHaveBeenCalled();
  rerender(
    <SessionSelector
      {...actions}
      model={buildSessionNavigation({
        ...data,
        organization: data.organization.map((o) =>
          o.sessionId === 'demo-1' ? { ...o, pinnedAt: null } : o,
        ),
      })}
    />,
  );
  const pin = await screen.findByRole('button', { name: 'Pin Discussion 1' });
  fireEvent.click(pin);
  expect(actions.onPin).toHaveBeenCalledWith('demo-1', true);
  expect(screen.getAllByRole('treeitem', { name: 'Sessions' })[0]).toHaveAttribute(
    'aria-expanded',
    'true',
  );
});

it('reorders sibling headers by pointer or keyboard without reparenting', async () => {
  const actions = props();
  render(<SessionSelector {...actions} />);
  const alpha = await screen.findByRole('treeitem', { name: 'Alpha' });
  const empty = screen.getByRole('treeitem', { name: 'Empty repo' });
  dragTo(empty, alpha);
  expect(actions.onReorder).toHaveBeenCalledWith({ kind: 'repositories' }, ['repo-b', 'repo-a']);
  actions.onReorder.mockClear();
  fireEvent.focus(alpha);
  fireEvent.keyDown(alpha, { key: 'ArrowDown', altKey: true });
  expect(actions.onReorder).toHaveBeenCalledWith({ kind: 'repositories' }, ['repo-b', 'repo-a']);
  actions.onReorder.mockClear();
  dragTo(empty, screen.getByRole('treeitem', { name: 'Feature build' }));
  expect(actions.onReorder).not.toHaveBeenCalled();
  expect(actions.onMove).not.toHaveBeenCalled();
});

it('reorders pinned shortcuts independently and preserves closed folders when a shortcut is selected', async () => {
  const actions = props();
  const model = buildSessionNavigation(populatedNavigationData());
  const { rerender } = render(<SessionSelector {...actions} model={model} />);
  const pins = screen.getByRole('tree', { name: 'Pinned sessions' });
  const first = within(pins).getByRole('treeitem', { name: 'Discussion 6' });
  const last = within(pins).getByRole('treeitem', { name: 'Discussion 1' });
  dragTo(last, first);
  expect(actions.onReorder).toHaveBeenCalledWith({ kind: 'pinned' }, [
    'demo-1',
    'demo-6',
    'demo-5',
    'demo-4',
    'demo-3',
    'demo-2',
  ]);
  expect(actions.onMove).not.toHaveBeenCalled();
  const repo = await screen.findByRole('treeitem', { name: 'Alpha' });
  fireEvent.click(repo);
  await new Promise((resolve) => setTimeout(resolve, 0));
  fireEvent.click(last);
  rerender(
    <SessionSelector {...actions} model={model} selectedSessionId="demo-1" revealKey="pin-open" />,
  );
  await waitFor(() => expect(repo).toHaveAttribute('aria-expanded', 'false'));
  expect(repo).toHaveClass('contains-selection');
  expect(screen.getAllByRole('treeitem', { name: 'Discussion 1' })).toHaveLength(1);
  rerender(
    <SessionSelector
      {...actions}
      model={model}
      selectedSessionId="demo-1"
      revealKey="external-open"
    />,
  );
  await waitFor(() => expect(repo).toHaveAttribute('aria-expanded', 'true'));
});

it('collapses ownership groups independently and opens the exact workflow session from its hover action', async () => {
  const actions = props();
  const onOpenWorkflow = vi.fn();
  render(
    <SessionSelector
      {...actions}
      model={buildSessionNavigation(populatedNavigationData())}
      onOpenWorkflow={onOpenWorkflow}
    />,
  );
  const added = await screen.findByRole('treeitem', { name: 'Added sessions' });
  const owned = screen.getByRole('treeitem', { name: 'Workflow sessions' });
  fireEvent.click(added);
  expect(added).toHaveAttribute('aria-expanded', 'false');
  expect(screen.queryByRole('treeitem', { name: 'Discussion 9' })).toBeNull();
  expect(screen.getByRole('treeitem', { name: 'Discussion 16' })).toBeVisible();
  fireEvent.click(owned);
  expect(screen.queryByRole('treeitem', { name: 'Discussion 12' })).toBeNull();
  fireEvent.keyDown(owned, { key: 'ArrowRight' });
  fireEvent.click(screen.getByRole('button', { name: 'Open workflow for Discussion 12' }));
  expect(onOpenWorkflow).toHaveBeenCalledWith({
    instanceId: 'flow-a',
    session: { nodeId: 'worker', sessionId: 'demo-12' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Open workflow Feature build' }));
  expect(onOpenWorkflow).toHaveBeenLastCalledWith({ instanceId: 'flow-a' });
  expect(actions.onSelect).not.toHaveBeenCalled();
});

it('inserts cross-folder and same-folder drops at a row boundary and keeps ownership groups ordered', async () => {
  const actions = props();
  render(
    <SessionSelector {...actions} model={buildSessionNavigation(populatedNavigationData())} />,
  );
  const repo = within(screen.getByRole('treeitem', { name: 'Alpha' }).parentElement!);
  const row = (title: string) => repo.getByRole('treeitem', { name: title });
  const target = row('Discussion 3');
  const surface = target.closest('[data-session-placement]')!;
  surface.querySelectorAll<HTMLElement>('[data-session-row]').forEach((element, i) => {
    element.getBoundingClientRect = () => ({ top: i * 32, height: 32 }) as DOMRect;
  });
  const source = row('Discussion 9');
  source.setPointerCapture = vi.fn();
  source.hasPointerCapture = () => true;
  source.releasePointerCapture = vi.fn();
  Object.defineProperty(document, 'elementFromPoint', { configurable: true, value: () => target });
  for (const [kind, y] of [
    ['pointerDown', 200],
    ['pointerMove', 65],
  ] as const) {
    const event = createEvent[kind](source);
    Object.defineProperties(event, {
      pointerId: { value: 1 },
      button: { value: 0 },
      clientX: { value: 100 },
      clientY: { value: y },
    });
    fireEvent(source, event);
  }
  expect(target).toHaveAttribute('data-insertion', 'before');
  expect(surface).not.toHaveAttribute('data-drop-target');
  const up = createEvent.pointerUp(source);
  Object.defineProperties(up, {
    pointerId: { value: 1 },
    button: { value: 0 },
    clientX: { value: 100 },
    clientY: { value: 65 },
  });
  fireEvent(source, up);
  expect(actions.onMove).toHaveBeenLastCalledWith(
    'demo-9',
    { kind: 'repository', repositoryId: 'repo-a' },
    ['demo-1', 'demo-2', 'demo-9', 'demo-3', 'demo-4', 'demo-5', 'demo-6', 'demo-7', 'demo-8'],
  );
  expect(target).not.toHaveAttribute('data-insertion');
  dragTo(row('Discussion 4'), target, 65);
  expect(actions.onMove).toHaveBeenLastCalledWith(
    'demo-4',
    { kind: 'repository', repositoryId: 'repo-a' },
    ['demo-1', 'demo-2', 'demo-4', 'demo-3', 'demo-5', 'demo-6', 'demo-7', 'demo-8'],
  );
  dragTo(row('Discussion 1'), row('Discussion 12'), 1000);
  expect(actions.onMove).toHaveBeenLastCalledWith(
    'demo-1',
    { kind: 'workflow_instance', instanceId: 'flow-a' },
    [
      'demo-9',
      'demo-10',
      'demo-11',
      'demo-1',
      'demo-12',
      'demo-13',
      'demo-14',
      'demo-15',
      'demo-16',
    ],
  );
});
