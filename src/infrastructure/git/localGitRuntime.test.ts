import type { GitCommandInput, GitCommandResult, GitCommandRunner } from './types';
import {
  GitCommandError,
  buildGitTrackedDiffArgs,
  buildGitWorktreeAddArgs,
  createLocalGitDiffProvider,
  createLocalGitRuntimeAdapters,
  createLocalGitWorktreeCreator,
  createNodeGitCommandRunner,
  type GitProcessRunInput,
  type GitProcessRunResult,
  type GitProcessRunner,
} from './localGitRuntime';

describe('createNodeGitCommandRunner', () => {
  it('runs Git through the injected process runner and returns successful output', async () => {
    const processRunner = new FakeGitProcessRunner({
      stdout: 'clean',
      stderr: '',
      exitCode: 0,
      signal: null,
    });
    const runner = createNodeGitCommandRunner({
      command: 'git-preview',
      processRunner,
    });

    const result = await runner.runGit({
      cwd: 'C:\\Repos\\App',
      args: ['status', '--porcelain=v1', '-z'],
    });

    expect(processRunner.inputs).toEqual([
      {
        command: 'git-preview',
        cwd: 'C:\\Repos\\App',
        args: ['status', '--porcelain=v1', '-z'],
      },
    ]);
    expect(result).toEqual({
      stdout: 'clean',
      stderr: '',
      exitCode: 0,
    });
  });

  it('throws a typed error with command context for non-zero Git exits', async () => {
    const processRunner = new FakeGitProcessRunner({
      stdout: 'partial output',
      stderr: 'fatal: not a git repository',
      exitCode: 128,
      signal: null,
    });
    const runner = createNodeGitCommandRunner({ processRunner });

    try {
      await runner.runGit({
        cwd: 'C:\\Repos\\Missing',
        args: ['status', '--porcelain=v1', '-z'],
      });
    } catch (error) {
      expect(error).toBeInstanceOf(GitCommandError);
      expect(error).toMatchObject({
        name: 'GitCommandError',
        command: 'git',
        args: ['status', '--porcelain=v1', '-z'],
        cwd: 'C:\\Repos\\Missing',
        exitCode: 128,
        signal: null,
        stdout: 'partial output',
        stderr: 'fatal: not a git repository',
      });
      expect((error as Error).message).toContain('exit code 128');
      return;
    }

    throw new Error('Expected GitCommandError');
  });

  it('wraps launch failures with command context', async () => {
    const launchError = new Error('spawn git ENOENT');
    const processRunner = new FakeGitProcessRunner(launchError);
    const runner = createNodeGitCommandRunner({ processRunner });

    await expect(
      runner.runGit({
        cwd: 'C:\\Repos\\App',
        args: ['status'],
      }),
    ).rejects.toMatchObject({
      name: 'GitCommandLaunchError',
      command: 'git',
      args: ['status'],
      cwd: 'C:\\Repos\\App',
      cause: launchError,
    });
  });
});

describe('local Git worktree creator', () => {
  it('builds narrow git worktree add arguments', () => {
    expect(
      buildGitWorktreeAddArgs({
        repoRootPath: 'C:\\Repos\\App',
        worktreePath: 'C:\\Repos\\App Worktrees\\035',
        branchName: 'worker/035-local-git-runtime-adapters',
        baseBranch: 'main',
      }),
    ).toEqual([
      'worktree',
      'add',
      '-b',
      'worker/035-local-git-runtime-adapters',
      'C:\\Repos\\App Worktrees\\035',
      'main',
    ]);
  });

  it('runs the worktree add command and returns the created worktree facts', async () => {
    const runner = new FakeGitCommandRunner([{ stdout: '', stderr: '', exitCode: 0 }]);
    const creator = createLocalGitWorktreeCreator({ commandRunner: runner });

    const result = await creator.createWorktree({
      repoRootPath: 'C:\\Repos\\App',
      worktreePath: 'C:\\Repos\\App Worktrees\\035',
      branchName: 'worker/035-local-git-runtime-adapters',
      baseBranch: 'main',
    });

    expect(runner.inputs).toEqual([
      {
        cwd: 'C:\\Repos\\App',
        args: [
          'worktree',
          'add',
          '-b',
          'worker/035-local-git-runtime-adapters',
          'C:\\Repos\\App Worktrees\\035',
          'main',
        ],
      },
    ]);
    expect(result).toEqual({
      repoRootPath: 'C:/Repos/App',
      worktreePath: 'C:/Repos/App Worktrees/035',
      branchName: 'worker/035-local-git-runtime-adapters',
      baseBranch: 'main',
    });
  });
});

describe('local Git diff provider', () => {
  it('builds the tracked binary diff arguments', () => {
    expect(buildGitTrackedDiffArgs()).toEqual(['diff', '--binary', 'HEAD', '--']);
  });

  it('returns diff stdout from the worktree path', async () => {
    const diff = [
      'diff --git a/src/app.ts b/src/app.ts',
      'index 1111111..2222222 100644',
      '--- a/src/app.ts',
      '+++ b/src/app.ts',
      '@@ -1 +1 @@',
      '-old',
      '+new',
      '',
    ].join('\n');
    const runner = new FakeGitCommandRunner([{ stdout: diff, stderr: '', exitCode: 0 }]);
    const provider = createLocalGitDiffProvider({ commandRunner: runner });

    const result = await provider.collectDiff({ worktreePath: 'C:\\Repos\\App Worktrees\\035' });

    expect(runner.inputs).toEqual([
      {
        cwd: 'C:\\Repos\\App Worktrees\\035',
        args: ['diff', '--binary', 'HEAD', '--'],
      },
    ]);
    expect(result).toEqual({ diff });
  });
});

describe('createLocalGitRuntimeAdapters', () => {
  it('bundles one command runner into the local scanner, worktree creator, and diff provider', () => {
    const processRunner = new FakeGitProcessRunner({
      stdout: '',
      stderr: '',
      exitCode: 0,
      signal: null,
    });

    const adapters = createLocalGitRuntimeAdapters({
      command: 'git-preview',
      processRunner,
      clock: () => new Date('2026-07-02T12:00:00.000Z'),
    });

    expect(adapters.commandRunner).toBeDefined();
    expect(adapters.repoScanner).toBeDefined();
    expect(adapters.worktreeCreator).toBeDefined();
    expect(adapters.diffProvider).toBeDefined();
  });
});

class FakeGitProcessRunner implements GitProcessRunner {
  readonly inputs: GitProcessRunInput[] = [];

  constructor(private readonly result: GitProcessRunResult | Error) {}

  async run(input: GitProcessRunInput): Promise<GitProcessRunResult> {
    this.inputs.push({
      command: input.command,
      cwd: input.cwd,
      args: [...input.args],
    });

    if (this.result instanceof Error) {
      throw this.result;
    }

    return {
      stdout: this.result.stdout,
      stderr: this.result.stderr,
      exitCode: this.result.exitCode,
      signal: this.result.signal,
    };
  }
}

class FakeGitCommandRunner implements GitCommandRunner {
  readonly inputs: GitCommandInput[] = [];

  constructor(private readonly results: GitCommandResult[]) {}

  async runGit(input: GitCommandInput): Promise<GitCommandResult> {
    this.inputs.push({
      cwd: input.cwd,
      args: [...input.args],
    });

    const result = this.results.shift();

    if (result === undefined) {
      throw new Error('Unexpected git command');
    }

    return { ...result };
  }
}
