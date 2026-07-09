export type GitWorktreeDirtyState = 'clean' | 'dirty' | 'unknown';

export interface GitRepoDomainFacts {
  name: string;
  rootPath: string;
  defaultBranch?: string;
  remoteUrl?: string;
}

export interface GitBranchDomainFacts {
  name: string;
  headSha?: string;
  isCurrent: boolean;
  upstreamName?: string;
  upstreamTrack?: string;
  worktreePath?: string;
}

export interface GitWorktreeDomainFacts {
  path: string;
  branchName?: string;
  headSha?: string;
  isMain: boolean;
  dirtyState: GitWorktreeDirtyState;
  isDirty: boolean;
  isBare: boolean;
  isDetached: boolean;
  isLocked: boolean;
  lockReason?: string;
  isPrunable: boolean;
  pruneReason?: string;
  lastScannedAt: string;
}

export interface GitRepoScanDomainFacts {
  repo: GitRepoDomainFacts;
  branches: GitBranchDomainFacts[];
  worktrees: GitWorktreeDomainFacts[];
}
