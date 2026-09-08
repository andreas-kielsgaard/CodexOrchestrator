import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi } from 'vitest';
import type { RepositoryCatalogClient } from '../../application/repositoryCatalog';
import type { ResolvedRepoBranchWorktreeTarget } from '../../application/worktreeTargets';
import { RepositoryWorktreeTargetSelector } from './RepositoryWorktreeTargetSelector';

const targets: readonly ResolvedRepoBranchWorktreeTarget[] = [
  {
    repository: {
      id: 'repo-orchestrator',
      name: 'Codex Orchestrator',
      gitCommonDirectory: 'C:\\Repos\\Codex Orchestrator\\.git',
    },
    branch: { id: 'branch-main', name: 'main' },
    worktree: { id: 'worktree-main', path: 'C:\\Repos\\Codex Orchestrator' },
  },
  {
    repository: {
      id: 'repo-extension',
      name: 'Image Saver',
      gitCommonDirectory: 'C:\\Repos\\Image Saver\\.git',
    },
    branch: { id: 'branch-viewer', name: 'viewer' },
    worktree: { id: 'worktree-viewer', path: 'C:\\Worktrees\\viewer' },
  },
];

describe('RepositoryWorktreeTargetSelector', () => {
  it('shows registered repository worktrees and emits the exact resolved target', async () => {
    const user = userEvent.setup();
    const catalog = catalogWithTargets(targets);
    const onChange = vi.fn();
    render(<RepositoryWorktreeTargetSelector catalog={catalog} value={null} onChange={onChange} />);

    const selector = await screen.findByRole('combobox', { name: 'Repository and branch' });
    await waitFor(() => expect(selector).toBeEnabled());
    expect(screen.getByRole('option', { name: 'Codex Orchestrator · main' })).toBeVisible();
    expect(screen.getByRole('option', { name: 'Image Saver · viewer' })).toBeVisible();
    expect(screen.queryByText(targets[1]!.worktree.path)).toBeNull();

    await user.selectOptions(selector, 'worktree-viewer');
    expect(onChange).toHaveBeenCalledWith(targets[1]);
  });

  it('keeps catalog failure local to the selector', async () => {
    const catalog = catalogWithTargets(async () => {
      throw new Error('Repository catalog is unavailable.');
    });
    render(
      <RepositoryWorktreeTargetSelector
        catalog={catalog}
        value={null}
        onChange={() => undefined}
      />,
    );

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Repository catalog is unavailable.',
    );
    expect(screen.getByRole('combobox', { name: 'Repository and branch' })).toBeDisabled();
  });
});

function catalogWithTargets(
  result:
    | readonly ResolvedRepoBranchWorktreeTarget[]
    | (() => Promise<readonly ResolvedRepoBranchWorktreeTarget[]>),
): Pick<RepositoryCatalogClient, 'listWorktreeTargets'> {
  return {
    listWorktreeTargets: typeof result === 'function' ? result : vi.fn(async () => result),
  };
}
