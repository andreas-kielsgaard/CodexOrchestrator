import {
  parseGitBranchSummary,
  parseGitRemoteVerbose,
  parseGitStatusPorcelainV1Z,
  parseGitWorktreeListPorcelainZ,
  normalizeGitPath,
} from './parsers';
import type {
  GitBranchSummary,
  GitCommandRunner,
  GitRepoScanDomainFacts,
  GitRepoScanResult,
} from './types';

export interface GitRepoScanner {
  scanRepo(input: GitRepoScanInput): Promise<GitRepoScanResult>;
}

export interface GitRepoScanInput {
  rootPath: string;
  defaultBranch?: string;
  scannedAt?: string;
}

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

export function mapGitRepoScanToDomainFacts(scan: GitRepoScanResult): GitRepoScanDomainFacts {
  const rootPath = normalizeGitPath(scan.rootPath);
  const defaultBranch = scan.defaultBranch ?? scan.currentBranch;
  const primaryRemote = scan.remotes.find((remote) => remote.name === 'origin') ?? scan.remotes[0];

  return {
    repo: {
      name: repoNameFromPath(rootPath),
      rootPath,
      ...(defaultBranch ? { defaultBranch } : {}),
      ...(primaryRemote?.fetchUrl || primaryRemote?.pushUrl
        ? { remoteUrl: primaryRemote.fetchUrl ?? primaryRemote.pushUrl }
        : {}),
    },
    branches: scan.branches.map((branch) => ({
      name: branch.name,
      headSha: branch.headSha,
      isCurrent: branch.isCurrent,
      ...(branch.upstreamName ? { upstreamName: branch.upstreamName } : {}),
      ...(branch.upstreamTrack ? { upstreamTrack: branch.upstreamTrack } : {}),
      ...(branch.worktreePath ? { worktreePath: branch.worktreePath } : {}),
    })),
    worktrees: scan.worktrees.map((worktree) => {
      const isMain = normalizeGitPath(worktree.path) === rootPath;
      const dirtyState = isMain ? (scan.status.isDirty ? 'dirty' : 'clean') : 'unknown';

      return {
        path: worktree.path,
        branchName: worktree.branchName,
        headSha: worktree.headSha,
        isMain,
        dirtyState,
        isDirty: dirtyState === 'dirty',
        isBare: worktree.isBare,
        isDetached: worktree.isDetached,
        isLocked: worktree.isLocked,
        ...(worktree.lockReason ? { lockReason: worktree.lockReason } : {}),
        isPrunable: worktree.isPrunable,
        ...(worktree.pruneReason ? { pruneReason: worktree.pruneReason } : {}),
        lastScannedAt: scan.scannedAt,
      };
    }),
  };
}

function repoNameFromPath(path: string): string {
  const trimmedPath = path.replace(/\/+$/, '');
  const segments = trimmedPath.split('/');
  return segments[segments.length - 1] || trimmedPath;
}
