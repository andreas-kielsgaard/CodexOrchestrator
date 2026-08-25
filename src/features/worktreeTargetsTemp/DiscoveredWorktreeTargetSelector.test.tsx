import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi } from 'vitest';
import type {
  RepoBranchWorktreeTargetSource,
  ResolvedRepoBranchWorktreeTarget,
} from '../../application/worktreeTargets';
import { DiscoveredWorktreeTargetSelector } from './DiscoveredWorktreeTargetSelector';

const targets: readonly ResolvedRepoBranchWorktreeTarget[] = [
  {
    repository: {
      id: 'repo-orchestrator',
      name: 'Codex Orchestrator',
      rootPath: 'C:\\Repos\\Codex Orchestrator',
    },
    branch: { id: 'branch-main', name: 'main' },
    worktree: { id: 'worktree-main', path: 'C:\\Repos\\Codex Orchestrator' },
  },
  {
    repository: {
      id: 'repo-extension',
      name: 'Image Saver',
      rootPath: 'C:\\Repos\\Image Saver',
    },
    branch: { id: 'branch-viewer', name: 'viewer' },
    worktree: { id: 'worktree-viewer', path: 'C:\\Worktrees\\viewer' },
  },
];

describe('DiscoveredWorktreeTargetSelector', () => {
  it('shows repository and branch choices and emits the exact resolved target', async () => {
    const user = userEvent.setup();
    const source: RepoBranchWorktreeTargetSource = {
      listTargets: vi.fn(async () => targets),
    };
    const onChange = vi.fn();
    render(<DiscoveredWorktreeTargetSelector source={source} value={null} onChange={onChange} />);

    const selector = await screen.findByRole('combobox', { name: 'Repository and branch' });
    await waitFor(() => expect(selector).toBeEnabled());
    expect(screen.getByRole('option', { name: 'Codex Orchestrator · main' })).toBeVisible();
    expect(screen.getByRole('option', { name: 'Image Saver · viewer' })).toBeVisible();
    expect(screen.queryByText(targets[1]!.worktree.path)).toBeNull();

    await user.selectOptions(selector, 'worktree-viewer');
    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith(targets[1]);
  });

  it('keeps discovery failure local to the temporary selector', async () => {
    const source: RepoBranchWorktreeTargetSource = {
      listTargets: vi.fn(async () => {
        throw new Error('Worktree discovery is unavailable.');
      }),
    };
    render(
      <DiscoveredWorktreeTargetSelector source={source} value={null} onChange={() => undefined} />,
    );

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Worktree discovery is unavailable.',
    );
    expect(screen.getByRole('combobox', { name: 'Repository and branch' })).toBeDisabled();
  });
});
