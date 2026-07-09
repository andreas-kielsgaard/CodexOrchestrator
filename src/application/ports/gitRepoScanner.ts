import type { GitRepoScanDomainFacts } from '../../domain/repoScanFacts';

export interface GitRepoScanner {
  scanRepo(input: GitRepoScanInput): Promise<GitRepoScanResult>;
}

export interface GitRepoScanInput {
  rootPath: string;
  defaultBranch?: string;
  scannedAt?: string;
}

export interface GitRepoScanResult {
  rootPath: string;
  currentBranch?: string;
  defaultBranch?: string;
  remotes: GitRemoteSummary[];
  branches: GitBranchSummary[];
  status: GitStatusSnapshot;
  worktrees: GitWorktreeSummary[];
  scannedAt: string;
}

export interface GitRemoteSummary {
  name: string;
  fetchUrl?: string;
  pushUrl?: string;
}

export interface GitBranchSummary {
  name: string;
  headSha: string;
  isCurrent: boolean;
  upstreamName?: string;
  upstreamTrack?: string;
  worktreePath?: string;
}

export type GitStatusCode = ' ' | '!' | '?' | 'A' | 'C' | 'D' | 'M' | 'R' | 'T' | 'U';

export type GitStatusEntryKind =
  | 'added'
  | 'copied'
  | 'deleted'
  | 'ignored'
  | 'modified'
  | 'renamed'
  | 'type_changed'
  | 'unmerged'
  | 'unknown'
  | 'untracked';

export interface GitStatusEntry {
  path: string;
  originalPath?: string;
  indexStatus: GitStatusCode;
  worktreeStatus: GitStatusCode;
  kind: GitStatusEntryKind;
}

export interface GitStatusSnapshot {
  entries: GitStatusEntry[];
  isDirty: boolean;
}

export type GitWorktreeState = 'bare' | 'branch' | 'detached';

export interface GitWorktreeSummary {
  path: string;
  headSha?: string;
  branchName?: string;
  state: GitWorktreeState;
  isBare: boolean;
  isDetached: boolean;
  isLocked: boolean;
  lockReason?: string;
  isPrunable: boolean;
  pruneReason?: string;
}

export function mapGitRepoScanToDomainFacts(scan: GitRepoScanResult): GitRepoScanDomainFacts {
  const rootPath = normalizeGitScanPath(scan.rootPath);
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
      const isMain = normalizeGitScanPath(worktree.path) === rootPath;
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

export function normalizeGitScanPath(path: string): string {
  return path.replace(/\\/g, '/');
}

function repoNameFromPath(path: string): string {
  const trimmedPath = path.replace(/\/+$/, '');
  const segments = trimmedPath.split('/');
  return segments[segments.length - 1] || trimmedPath;
}
