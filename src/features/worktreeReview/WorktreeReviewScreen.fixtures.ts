import type {
  AssociatedWorktree,
  BranchReviewDetail,
  GitCommit,
  ReviewBuild,
  WorktreeReviewOverview,
} from '../../application/worktreeReview';

export const baseCommit: GitCommit = {
  objectId: '1111111111111111111111111111111111111111',
  abbreviatedObjectId: '11111111',
  subject: 'Establish review runtime',
  author: 'Codex',
  committedAt: '2026-08-20T10:00:00Z',
};

export const tipCommit: GitCommit = {
  objectId: '2222222222222222222222222222222222222222',
  abbreviatedObjectId: '22222222',
  subject: 'Add durable build records',
  author: 'Codex',
  committedAt: '2026-08-21T10:00:00Z',
};

export const earlierCommit: GitCommit = {
  objectId: '1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a',
  abbreviatedObjectId: '1a1a1a1a',
  subject: 'Separate product composition',
  author: 'Codex',
  committedAt: '2026-08-20T16:00:00Z',
};

export const worktreeOne: AssociatedWorktree = {
  associationId: 'association-agent-one',
  worktreeId: 'worktree-agent-one',
  branchRef: 'refs/heads/codex/durable-review',
  name: 'Agent checkout A',
  locationLabel: 'C:\\worktrees\\durable-a',
  provenance: 'git_branch_checkout',
  ownership: 'borrowed_external',
  baseline: { kind: 'observed_at_association', commit: baseCommit },
  currentHead: tipCommit,
  changes: {
    commitsAheadOfBaseline: 1,
    commitsBehindBaseline: 0,
    stagedFiles: 1,
    unstagedFiles: 2,
    untrackedFiles: 0,
  },
  detachedHead: false,
  branchReachability: 'reachable',
  availability: { state: 'available' },
};

export const worktreeTwo: AssociatedWorktree = {
  ...worktreeOne,
  associationId: 'association-agent-two',
  worktreeId: 'worktree-agent-two',
  name: 'Agent checkout B',
  locationLabel: 'C:\\worktrees\\durable-b',
  provenance: 'user_associated_detached_checkout',
  changes: {
    commitsAheadOfBaseline: 1,
    commitsBehindBaseline: 0,
    stagedFiles: 0,
    unstagedFiles: 0,
    untrackedFiles: 3,
  },
  detachedHead: true,
};

export const completedBuild: ReviewBuild = {
  buildId: 'build-completed',
  name: 'Completed review build',
  branchRef: 'refs/heads/codex/durable-review',
  source: {
    kind: 'branch_commit',
    branchRef: 'refs/heads/codex/durable-review',
    objectId: tipCommit.objectId,
  },
  workspace: {
    worktreeId: 'worktree-build-owned',
    ownership: 'owned_build_worktree',
    locationLabel: 'Worktree Review storage / workspaces / build-completed',
    lifecycle: 'ready',
  },
  latestAttempt: {
    attemptId: 'attempt-completed',
    executionState: 'completed',
    outcome: 'succeeded',
    stage: 'complete',
    startedAt: '2026-08-21T11:00:00Z',
    completedAt: '2026-08-21T11:05:00Z',
  },
  output: {
    state: 'available',
    buildOutputId: 'output-completed',
    storageLabel: 'AppData / build-output / build-completed',
  },
  cleanup: { state: 'retained', policy: 'Newest successful build for this source' },
};

export const failedBuild: ReviewBuild = {
  ...completedBuild,
  buildId: 'build-failed',
  name: 'Failed rebuild',
  latestAttempt: {
    attemptId: 'attempt-failed',
    executionState: 'completed',
    outcome: 'failed',
    stage: 'compile',
    startedAt: '2026-08-22T11:00:00Z',
    completedAt: '2026-08-22T11:01:00Z',
    failure: { category: 'build', stage: 'compile', summary: 'Compiler exited with code 1.' },
  },
  output: { state: 'not_produced' },
  cleanup: { state: 'eligible', reason: 'Failed terminal attempt' },
};

export const overviewFixture: WorktreeReviewOverview = {
  selectedRepositoryId: 'repository-one',
  repositories: [
    {
      repositoryId: 'repository-one',
      name: 'Codex Orchestrator',
      locationLabel: 'C:\\code\\Codex Orchestrator',
      readiness: {
        state: 'ready',
        browse: { state: 'available' },
        createWorktree: { state: 'available' },
        build: { state: 'available' },
        buildOutputStorage: { state: 'available' },
      },
    },
  ],
  branches: [
    {
      repositoryId: 'repository-one',
      target: {
        kind: 'branch',
        repositoryId: 'repository-one',
        branchRef: 'refs/heads/codex/durable-review',
      },
      availableWorktreeCount: 2,
      worktreeIds: [worktreeOne.worktreeId, worktreeTwo.worktreeId],
      activity: null,
      branchRef: 'refs/heads/codex/durable-review',
      displayName: 'codex/durable-review',
      tip: tipCommit,
      aheadOfDefault: 3,
      behindDefault: 0,
      associatedWorktreeCount: 2,
    },
    {
      repositoryId: 'repository-one',
      target: { kind: 'branch', repositoryId: 'repository-one', branchRef: 'refs/heads/main' },
      availableWorktreeCount: 1,
      worktreeIds: ['worktree-main'],
      activity: null,
      branchRef: 'refs/heads/main',
      displayName: 'main',
      tip: baseCommit,
      aheadOfDefault: 0,
      behindDefault: 0,
      associatedWorktreeCount: 1,
    },
  ],
};

export function branchDetailFixture(
  overrides: Partial<BranchReviewDetail> = {},
): BranchReviewDetail {
  return {
    branch: overviewFixture.branches[0],
    worktrees: [worktreeOne, worktreeTwo],
    associationCandidates: [
      {
        worktreeId: 'candidate-detached',
        name: 'Detached investigation',
        locationLabel: 'C:\\worktrees\\detached-investigation',
        currentHead: earlierCommit,
        detachedHead: true,
        associationReason: 'This checkout has not been associated with a Worktree Review branch.',
      },
    ],
    builds: [failedBuild, completedBuild],
    ...overrides,
  };
}
