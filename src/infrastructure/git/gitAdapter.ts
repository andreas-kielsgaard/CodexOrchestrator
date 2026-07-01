import type { GitBranchSummary, GitCommandRunner, GitRepoScanResult } from './types';

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
  worktreeListPorcelainZ: string;
}

export function currentBranch(branches: GitBranchSummary[]): string | undefined {
  return branches.find((branch) => branch.isCurrent)?.name;
}
