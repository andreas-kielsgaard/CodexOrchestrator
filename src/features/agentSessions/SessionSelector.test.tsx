import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { buildSessionNavigation } from '../../application/agentSessions/navigation';
import { navigationData } from './navigationTestFixtures';
import { SessionSelector as Selector } from './SessionSelector';
import { useSessionTree } from './useSessionTree';
import type { ComponentProps } from 'react';
function SessionSelector(
  props: Omit<ComponentProps<typeof Selector>, 'tree'> & { revealKey: string },
) {
  const tree = useSessionTree(props.model, props.selectedSessionId, props.revealKey);
  return <Selector {...props} tree={tree} />;
}
const props = () => ({
  model: buildSessionNavigation(navigationData()),
  selectedSessionId: null,
  revealKey: 'initial',
  loading: false,
  onSelect: vi.fn(),
  onNew: vi.fn(),
  onMove: vi.fn().mockResolvedValue(undefined),
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
  const data = new Map<string, string>();
  const transfer = {
    types: ['application/x-orchestrator-session'],
    setData: (k: string, v: string) => data.set(k, v),
    getData: (k: string) => data.get(k) ?? '',
    effectAllowed: '',
    dropEffect: '',
  };
  fireEvent.dragStart(row, { dataTransfer: transfer });
  fireEvent.drop(screen.getByRole('treeitem', { name: 'Feature build' }), {
    dataTransfer: transfer,
  });
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
