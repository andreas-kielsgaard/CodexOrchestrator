import { vi } from 'vitest';
import type { ResolvedRepoBranchWorktreeTarget } from '../../application/worktreeTargets';
import { createTauriDiscoveredWorktreeTargetSource } from './tauriDiscoveredWorktreeTargetSource';

describe('Tauri discovered worktree target source', () => {
  it('returns the exact resolved targets supplied by the temporary command', async () => {
    const targets: ResolvedRepoBranchWorktreeTarget[] = [
      {
        repository: {
          id: 'repo-orchestrator',
          name: 'Codex Orchestrator',
          rootPath: 'C:\\Repos\\Codex Orchestrator',
        },
        branch: { id: 'branch-feature', name: 'codex/workflow-engine-v1' },
        worktree: {
          id: 'worktree-feature',
          path: 'C:\\Worktrees\\workflow-engine-v1',
        },
      },
    ];
    const invoke = vi.fn().mockResolvedValue(targets);
    const source = createTauriDiscoveredWorktreeTargetSource(invoke);

    await expect(source.listTargets()).resolves.toBe(targets);
    expect(invoke).toHaveBeenCalledWith('list_discovered_worktree_targets');
  });
});
