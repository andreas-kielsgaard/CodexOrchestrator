export interface ResolvedRepoBranchWorktreeTarget {
  readonly repository: {
    readonly id: string;
    readonly name: string;
    /** Local Git repository identity shared by all linked worktrees. */
    readonly gitCommonDirectory: string;
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

/** Stable Workflow input independent of repository-catalog presentation and transport. */
export interface RepoBranchWorktreeTargetSelectorProps {
  readonly id?: string;
  readonly value: ResolvedRepoBranchWorktreeTarget | null;
  readonly disabled?: boolean;
  onChange(target: ResolvedRepoBranchWorktreeTarget): void;
}
