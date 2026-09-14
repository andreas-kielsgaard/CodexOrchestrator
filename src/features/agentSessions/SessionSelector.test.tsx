import { createEvent, fireEvent, render, screen, waitFor } from '@testing-library/react';
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
  expect(actions.onMove).toHaveBeenCalledWith('session-1', {
    kind: 'workflow_instance',
    instanceId: 'flow-a',
  });
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
