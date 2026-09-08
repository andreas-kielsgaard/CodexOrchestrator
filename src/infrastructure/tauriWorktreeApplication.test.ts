import {
  createTauriWorktreeCreator,
  type WorktreeApplicationInvoke,
} from './tauriWorktreeApplication';

describe('Tauri physical worktree adapter', () => {
  it('delegates named worktree creation to the exact native boundary', async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const invoke: WorktreeApplicationInvoke = async <Result>(
      command: string,
      args?: Record<string, unknown>,
    ) => {
      calls.push({ command, args });
      return {
        repositoryRoot: 'C:/Repos/App',
        worktreeRoot: 'C:/Worktrees/App-42',
        commitId: 'a'.repeat(40),
        headRef: 'refs/heads/worker/42',
      } as Result;
    };
    const creator = createTauriWorktreeCreator(invoke);

    await expect(
      creator.createWorktree({
        repoRootPath: 'C:\\Repos\\App',
        worktreePath: 'C:\\Worktrees\\App-42',
        branchName: 'worker/42',
        baseBranch: 'main',
      }),
    ).resolves.toEqual({
      repoRootPath: 'C:/Repos/App',
      worktreePath: 'C:/Worktrees/App-42',
      branchName: 'worker/42',
      baseBranch: 'main',
    });
    expect(calls).toEqual([
      {
        command: 'create_physical_worktree',
        args: {
          input: {
            repositoryRoot: 'C:\\Repos\\App',
            worktreeRoot: 'C:\\Worktrees\\App-42',
            branchName: 'worker/42',
            startPointRef: 'main',
          },
        },
      },
    ]);
  });
});
