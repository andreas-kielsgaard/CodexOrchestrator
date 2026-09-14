import {
  commitSourceContext,
  type AssociatedWorktree,
  type BranchReviewDetail,
  type CreateBuildRequest,
  type GitCommit,
} from '../../application/worktreeReview';

export type SourceMode = 'direct' | 'snapshot' | 'commit';
export interface BuildDraft {
  readonly sourceMode: SourceMode;
  readonly commit: GitCommit;
  readonly name: string;
}

export function initialBuildDraft(
  detail: BranchReviewDetail,
  activeWorktreeId?: string,
): BuildDraft {
  const available = detail.worktrees.find(
    (worktree) =>
      worktree.availability.state === 'available' && worktree.worktreeId !== activeWorktreeId,
  );
  return {
    sourceMode: detail.branch.target.kind !== 'commit' && available ? 'direct' : 'commit',
    commit: detail.branch.tip,
    name: `Review ${detail.branch.displayName}`,
  };
}

export function buildRequest(
  detail: BranchReviewDetail,
  worktree: AssociatedWorktree | undefined,
  draft: BuildDraft,
): CreateBuildRequest | null {
  const common = {
    repositoryId: detail.branch.repositoryId,
    branchRef: detail.branch.branchRef,
    name: draft.name.trim(),
  };
  if (draft.sourceMode !== 'commit') {
    if (!worktree || worktree.availability.state !== 'available') return null;
    const snapshot = draft.sourceMode === 'snapshot';
    if (!worktree.associationId || !detail.branch.branchRef)
      return {
        ...common,
        branchRef: null,
        source: {
          kind: 'physical_worktree',
          worktreeId: worktree.worktreeId,
          headObjectId: worktree.currentHead.objectId,
          snapshot,
        },
        workspacePlan: snapshot
          ? { kind: 'create_owned_build_worktree' }
          : { kind: 'borrow_physical_worktree', worktreeId: worktree.worktreeId },
      };
    return {
      ...common,
      source: {
        kind: snapshot ? 'worktree_snapshot' : 'existing_worktree',
        associationId: worktree.associationId,
      },
      workspacePlan: snapshot
        ? { kind: 'create_owned_build_worktree', originatingAssociationId: worktree.associationId }
        : { kind: 'borrow_selected_worktree', associationId: worktree.associationId },
    };
  }
  if (detail.branch.target.kind === 'branch' && detail.branch.branchRef)
    return {
      ...common,
      source: {
        kind: 'branch_commit',
        branchRef: detail.branch.branchRef,
        objectId: draft.commit.objectId,
      },
      workspacePlan: worktree
        ? { kind: 'create_owned_build_worktree' }
        : { kind: 'create_managed_branch_worktree', branchRef: detail.branch.branchRef },
    };
  return {
    ...common,
    branchRef: null,
    source: {
      kind: 'exact_commit',
      objectId: draft.commit.objectId,
      context: commitSourceContext(detail.branch),
    },
    workspacePlan: { kind: 'create_owned_build_worktree' },
  };
}

export function requiresCheckout(request: CreateBuildRequest): boolean {
  return (
    request.workspacePlan.kind === 'create_managed_branch_worktree' ||
    request.workspacePlan.kind === 'create_owned_build_worktree'
  );
}
