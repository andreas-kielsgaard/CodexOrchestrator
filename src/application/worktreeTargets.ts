export interface ResolvedRepoBranchWorktreeTarget {
  readonly repository: {
    readonly id: string;
    readonly name: string;
    readonly rootPath: string;
  };
  readonly branch: {
    readonly id: string;
    readonly name: string;
  };
  readonly worktree: {
    readonly id: string;
    readonly path: string;
  };
}

export interface RepoBranchWorktreeTargetSource {
  listTargets(): Promise<readonly ResolvedRepoBranchWorktreeTarget[]>;
}

/** Stable selector input shared by the temporary picker and its future replacement. */
export interface RepoBranchWorktreeTargetSelectorProps {
  readonly id?: string;
  readonly value: ResolvedRepoBranchWorktreeTarget | null;
  readonly disabled?: boolean;
  onChange(target: ResolvedRepoBranchWorktreeTarget): void;
}
