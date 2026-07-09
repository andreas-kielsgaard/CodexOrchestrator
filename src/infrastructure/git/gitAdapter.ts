import {
  gitBranchSummaryArgs,
  parseGitBranchSummary,
  parseGitRemoteVerbose,
  parseGitStatusPorcelainV1Z,
  parseGitWorktreeListPorcelainZ,
  normalizeGitPath,
} from './parsers';
export { mapGitRepoScanToDomainFacts } from '../../application/ports/gitRepoScanner';
import type {
  GitBranchSummary,
  GitCommandRunner,
} from './types';
import type {
  GitRepoScanResult,
  GitRepoScanner,
} from '../../application/ports/gitRepoScanner';

export interface GitAdapterDependencies {
  commandRunner: GitCommandRunner;
  clock?: () => Date;
}

export interface GitScanCommandOutputs {
  statusPorcelainV1Z: string;
  branchSummary: string;
  remoteVerbose: string;
  worktreeListPorcelainZ: string;
}

export function buildGitStatusPorcelainV1ZArgs(): string[] {
  return ['status', '--porcelain=v1', '-z'];
}

export function buildGitBranchSummaryArgs(): string[] {
  return [...gitBranchSummaryArgs];
}

export function buildGitRemoteVerboseArgs(): string[] {
  return ['remote', '-v'];
}

export function buildGitWorktreeListPorcelainZArgs(): string[] {
  return ['worktree', 'list', '--porcelain', '-z'];
}

export function createGitRepoScanner(dependencies: GitAdapterDependencies): GitRepoScanner {
  const clock = dependencies.clock ?? (() => new Date());

  return {
    async scanRepo(input) {
      const status = await dependencies.commandRunner.runGit({
        cwd: input.rootPath,
        args: buildGitStatusPorcelainV1ZArgs(),
      });
      const branches = await dependencies.commandRunner.runGit({
        cwd: input.rootPath,
        args: buildGitBranchSummaryArgs(),
      });
      const remotes = await dependencies.commandRunner.runGit({
        cwd: input.rootPath,
        args: buildGitRemoteVerboseArgs(),
      });
      const worktrees = await dependencies.commandRunner.runGit({
        cwd: input.rootPath,
        args: buildGitWorktreeListPorcelainZArgs(),
      });

      return buildGitRepoScanResult({
        rootPath: input.rootPath,
        defaultBranch: input.defaultBranch,
        scannedAt: input.scannedAt ?? clock().toISOString(),
        outputs: {
          statusPorcelainV1Z: status.stdout,
          branchSummary: branches.stdout,
          remoteVerbose: remotes.stdout,
          worktreeListPorcelainZ: worktrees.stdout,
        },
      });
    },
  };
}

export function currentBranch(branches: GitBranchSummary[]): string | undefined {
  return branches.find((branch) => branch.isCurrent)?.name;
}

export interface BuildGitRepoScanResultInput {
  rootPath: string;
  defaultBranch?: string;
  outputs: GitScanCommandOutputs;
  scannedAt: string;
}

export function buildGitRepoScanResult(input: BuildGitRepoScanResultInput): GitRepoScanResult {
  const branches = parseGitBranchSummary(input.outputs.branchSummary);

  return {
    rootPath: normalizeGitPath(input.rootPath),
    currentBranch: currentBranch(branches),
    defaultBranch: input.defaultBranch,
    remotes: parseGitRemoteVerbose(input.outputs.remoteVerbose),
    branches,
    status: parseGitStatusPorcelainV1Z(input.outputs.statusPorcelainV1Z),
    worktrees: parseGitWorktreeListPorcelainZ(input.outputs.worktreeListPorcelainZ),
    scannedAt: input.scannedAt,
  };
}
