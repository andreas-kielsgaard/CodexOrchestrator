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

export interface GitBranchSummary {
  name: string;
  headSha: string;
  isCurrent: boolean;
  upstreamName?: string;
  upstreamTrack?: string;
  worktreePath?: string;
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

export interface GitCommandRunner {
  runGit(input: GitCommandInput): Promise<GitCommandResult>;
}

export interface GitCommandInput {
  cwd: string;
  args: string[];
}

export interface GitCommandResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}
