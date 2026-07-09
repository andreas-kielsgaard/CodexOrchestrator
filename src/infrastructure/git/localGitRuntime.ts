import { spawn } from 'node:child_process';

import {
  buildGitBranchSummaryArgs,
  buildGitRemoteVerboseArgs,
  buildGitStatusPorcelainV1ZArgs,
  buildGitWorktreeListPorcelainZArgs,
  createGitRepoScanner,
} from './gitAdapter';
import type { GitRepoScanner } from '../../application/ports/gitRepoScanner';
import { normalizeGitPath } from './parsers';
import type { GitCommandInput, GitCommandResult, GitCommandRunner } from './types';

export interface GitProcessRunner {
  run(input: GitProcessRunInput): Promise<GitProcessRunResult>;
}

export interface GitProcessRunInput {
  command: string;
  args: string[];
  cwd: string;
}

export interface GitProcessRunResult {
  stdout: string;
  stderr: string;
  exitCode: number | null;
  signal: string | null;
}

export interface NodeGitCommandRunnerOptions {
  command?: string;
  processRunner?: GitProcessRunner;
}

export interface GitCommandErrorDetails {
  command: string;
  args: string[];
  cwd: string;
  exitCode: number | null;
  signal: string | null;
  stdout: string;
  stderr: string;
}

export class GitCommandError extends Error {
  readonly command: string;
  readonly args: string[];
  readonly cwd: string;
  readonly exitCode: number | null;
  readonly signal: string | null;
  readonly stdout: string;
  readonly stderr: string;

  constructor(details: GitCommandErrorDetails) {
    super(gitCommandErrorMessage(details));
    this.name = 'GitCommandError';
    this.command = details.command;
    this.args = [...details.args];
    this.cwd = details.cwd;
    this.exitCode = details.exitCode;
    this.signal = details.signal;
    this.stdout = details.stdout;
    this.stderr = details.stderr;
  }
}

export interface GitCommandLaunchErrorDetails {
  command: string;
  args: string[];
  cwd: string;
  cause: unknown;
}

export class GitCommandLaunchError extends Error {
  readonly command: string;
  readonly args: string[];
  readonly cwd: string;
  override readonly cause: unknown;

  constructor(details: GitCommandLaunchErrorDetails) {
    super(`Git command failed to launch: ${describeGitCommand(details)} (cwd: ${details.cwd})`);
    this.name = 'GitCommandLaunchError';
    this.command = details.command;
    this.args = [...details.args];
    this.cwd = details.cwd;
    this.cause = details.cause;
  }
}

export function createNodeGitCommandRunner(
  options: NodeGitCommandRunnerOptions = {},
): GitCommandRunner {
  const command = options.command ?? 'git';
  const processRunner = options.processRunner ?? createNodeGitProcessRunner();

  return {
    async runGit(input: GitCommandInput): Promise<GitCommandResult> {
      const args = [...input.args];
      let result: GitProcessRunResult;

      try {
        result = await processRunner.run({
          command,
          args,
          cwd: input.cwd,
        });
      } catch (cause) {
        throw new GitCommandLaunchError({
          command,
          args,
          cwd: input.cwd,
          cause,
        });
      }

      if (result.exitCode !== 0 || result.signal !== null) {
        throw new GitCommandError({
          command,
          args,
          cwd: input.cwd,
          exitCode: result.exitCode,
          signal: result.signal,
          stdout: result.stdout,
          stderr: result.stderr,
        });
      }

      return {
        stdout: result.stdout,
        stderr: result.stderr,
        exitCode: result.exitCode,
      };
    },
  };
}

export function createNodeGitProcessRunner(): GitProcessRunner {
  return {
    run: (input) =>
      new Promise((resolve, reject) => {
        const child = spawn(input.command, input.args, {
          cwd: input.cwd,
          shell: false,
          windowsHide: true,
        });
        let stdout = '';
        let stderr = '';
        let settled = false;

        child.stdout.setEncoding('utf8');
        child.stdout.on('data', (chunk: string) => {
          stdout += chunk;
        });

        child.stderr.setEncoding('utf8');
        child.stderr.on('data', (chunk: string) => {
          stderr += chunk;
        });

        child.once('error', (error) => {
          if (settled) {
            return;
          }

          settled = true;
          reject(error);
        });

        child.once('close', (exitCode, signal) => {
          if (settled) {
            return;
          }

          settled = true;
          resolve({
            stdout,
            stderr,
            exitCode,
            signal,
          });
        });
      }),
  };
}

export interface LocalGitWorktreeCreator {
  createWorktree(input: LocalGitCreateWorktreeInput): Promise<LocalGitCreateWorktreeResult>;
}

export interface LocalGitCreateWorktreeInput {
  repoRootPath: string;
  worktreePath: string;
  branchName: string;
  baseBranch?: string;
}

export interface LocalGitCreateWorktreeResult {
  repoRootPath: string;
  worktreePath: string;
  branchName: string;
  baseBranch?: string;
}

export interface LocalGitWorktreeCreatorDependencies {
  commandRunner: GitCommandRunner;
}

export function buildGitWorktreeAddArgs(input: LocalGitCreateWorktreeInput): string[] {
  return [
    'worktree',
    'add',
    '-b',
    input.branchName,
    input.worktreePath,
    ...(input.baseBranch === undefined ? [] : [input.baseBranch]),
  ];
}

export function createLocalGitWorktreeCreator(
  dependencies: LocalGitWorktreeCreatorDependencies,
): LocalGitWorktreeCreator {
  return {
    async createWorktree(input) {
      await dependencies.commandRunner.runGit({
        cwd: input.repoRootPath,
        args: buildGitWorktreeAddArgs(input),
      });

      return {
        repoRootPath: normalizeGitPath(input.repoRootPath),
        worktreePath: normalizeGitPath(input.worktreePath),
        branchName: input.branchName,
        ...(input.baseBranch === undefined ? {} : { baseBranch: input.baseBranch }),
      };
    },
  };
}

export interface LocalGitDiffProvider {
  collectDiff(input: LocalGitDiffProviderInput): Promise<LocalGitDiffProviderResult>;
}

export interface LocalGitDiffProviderInput {
  worktreePath: string;
}

export interface LocalGitDiffProviderResult {
  diff: string;
}

export interface LocalGitDiffProviderDependencies {
  commandRunner: GitCommandRunner;
}

export function buildGitTrackedDiffArgs(): string[] {
  return ['diff', '--binary', 'HEAD', '--'];
}

/**
 * Collects tracked staged and unstaged file changes. Untracked files are intentionally omitted by
 * the underlying `git diff --binary HEAD --` command.
 */
export function createLocalGitDiffProvider(
  dependencies: LocalGitDiffProviderDependencies,
): LocalGitDiffProvider {
  return {
    async collectDiff(input) {
      const result = await dependencies.commandRunner.runGit({
        cwd: input.worktreePath,
        args: buildGitTrackedDiffArgs(),
      });

      return {
        diff: result.stdout,
      };
    },
  };
}

export interface LocalGitRuntimeAdapters {
  commandRunner: GitCommandRunner;
  repoScanner: GitRepoScanner;
  worktreeCreator: LocalGitWorktreeCreator;
  diffProvider: LocalGitDiffProvider;
}

export interface LocalGitRuntimeAdaptersOptions extends NodeGitCommandRunnerOptions {
  clock?: () => Date;
}

export function createLocalGitRuntimeAdapters(
  options: LocalGitRuntimeAdaptersOptions = {},
): LocalGitRuntimeAdapters {
  const commandRunner = createNodeGitCommandRunner({
    ...(options.command === undefined ? {} : { command: options.command }),
    ...(options.processRunner === undefined ? {} : { processRunner: options.processRunner }),
  });

  return {
    commandRunner,
    repoScanner: createGitRepoScanner({
      commandRunner,
      ...(options.clock === undefined ? {} : { clock: options.clock }),
    }),
    worktreeCreator: createLocalGitWorktreeCreator({ commandRunner }),
    diffProvider: createLocalGitDiffProvider({ commandRunner }),
  };
}

export {
  buildGitBranchSummaryArgs,
  buildGitRemoteVerboseArgs,
  buildGitStatusPorcelainV1ZArgs,
  buildGitWorktreeListPorcelainZArgs,
};

function gitCommandErrorMessage(details: GitCommandErrorDetails): string {
  const reason =
    details.signal === null
      ? details.exitCode === null
        ? 'without an exit code'
        : `with exit code ${details.exitCode}`
      : `on signal ${details.signal}`;

  return `Git command failed ${reason}: ${describeGitCommand(details)} (cwd: ${details.cwd})`;
}

function describeGitCommand(input: { command: string; args: readonly string[] }): string {
  return [input.command, ...input.args].join(' ');
}
