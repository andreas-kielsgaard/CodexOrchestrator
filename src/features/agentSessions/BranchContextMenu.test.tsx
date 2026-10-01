import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { BranchContextMenu } from './BranchContextMenu';
import { SessionGitActionDialog } from './SessionGitActionDialog';
import type { SessionNavigationFolder } from '../../application/agentSessions/navigation';
import type { SessionNavigationClient } from '../../application/agentSessions/organization';
const worktree = {
  deviceId: 'remote',
  deviceName: 'Remote device',
  path: '/srv/worktree',
  head: 'abc',
  createdAt: '2026-10-01T00:00:00Z',
  dirty: true,
};
const node: SessionNavigationFolder = {
  kind: 'folder',
  role: 'branch',
  id: 'branch:test',
  label: 'test',
  branch: {
    id: 'branch:test',
    repositoryId: 'repo',
    branchRef: 'refs/heads/test',
    paths: ['/srv/worktree'],
    createdAt: worktree.createdAt,
    worktrees: [worktree],
  },
  order: { id: 'branch:test', scope: { kind: 'session_layout', folderId: 'repo:repo:sessions' } },
  placement: { kind: 'repository', repositoryId: 'repo' },
  createTarget: null,
  children: [],
};
function client(): SessionNavigationClient {
  return {
    load: vi.fn(),
    move: vi.fn(),
    pin: vi.fn(),
    reorder: vi.fn(),
    previewGitAction: vi.fn().mockResolvedValue({
      path: '/local/repo',
      arguments: ['branch', '--', 'next', 'abc'],
      status: 'clean',
    }),
    executeGitAction: vi.fn().mockResolvedValue('Done'),
    inspectWorktree: vi.fn().mockResolvedValue({
      path: worktree.path,
      branchRef: 'refs/heads/test',
      head: 'abc',
      detached: false,
      staged: { files: 2, bytes: 10 },
      unstaged: { files: 1, bytes: 3 },
      untracked: { files: 4, bytes: 20 },
    }),
  };
}
it('exposes Git actions without executing and inspects device-specific worktrees', async () => {
  const api = client(),
    action = vi.fn(),
    close = vi.fn();
  render(
    <BranchContextMenu
      node={node}
      position={{ x: 100, y: 100 }}
      client={api}
      onAction={action}
      onClose={close}
    />,
  );
  fireEvent.mouseEnter(screen.getByRole('menuitem', { name: 'Git commands ›' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Rename branch…' }));
  expect(action).toHaveBeenCalledWith('rename');
  expect(api.executeGitAction).not.toHaveBeenCalled();
  fireEvent.mouseEnter(screen.getByRole('menuitem', { name: 'Worktrees ›' }));
  const item = screen.getByRole('menuitem', { name: 'Remote device · worktree' });
  fireEvent.mouseEnter(item);
  await screen.findByText('2 staged · 1 unstaged · 4 untracked files');
  expect(api.inspectWorktree).toHaveBeenCalledWith('repo', 'refs/heads/test', worktree);
  fireEvent.click(item);
  fireEvent.mouseLeave(item);
  expect(screen.getByRole('dialog', { name: 'Worktree information' })).toBeVisible();
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  fireEvent.pointerDown(document.body);
  expect(close).toHaveBeenCalled();
});
it('closes an unpinned hover preview after leaving', async () => {
  render(
    <BranchContextMenu
      node={node}
      position={{ x: 0, y: 0 }}
      client={client()}
      onAction={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('menuitem', { name: 'Worktrees ›' }));
  const item = screen.getByRole('menuitem', { name: 'Remote device · worktree' });
  fireEvent.mouseEnter(item);
  await screen.findByRole('dialog');
  fireEvent.mouseLeave(item);
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
});
it('requires preview followed by confirmation and resets confirmation on edits', async () => {
  const api = client();
  render(<SessionGitActionDialog node={node} action="create" client={api} onClose={vi.fn()} />);
  fireEvent.change(screen.getByLabelText('New branch name'), { target: { value: 'next' } });
  fireEvent.click(screen.getByRole('button', { name: 'Preview command' }));
  await screen.findByRole('button', { name: 'Confirm and run' });
  expect(api.executeGitAction).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText('New branch name'), { target: { value: 'other' } });
  expect(screen.queryByRole('button', { name: 'Confirm and run' })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Preview command' }));
  await screen.findByRole('button', { name: 'Confirm and run' });
  fireEvent.click(screen.getByRole('button', { name: 'Confirm and run' }));
  await screen.findByText('Done');
  expect(api.executeGitAction).toHaveBeenCalledWith(
    expect.objectContaining({
      value: 'other',
      startPoint: 'refs/heads/test',
      repositoryId: 'repo',
    }),
    expect.objectContaining({ path: '/local/repo', arguments: ['branch', '--', 'next', 'abc'] }),
  );
});
