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
  observedStateFingerprint: 'fingerprint-a',
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
  applicationMetadata: [{ label: 'Package', value: 'codex-orchestrator', source: 'worktree' }],
};

export const worktreeTwo: AssociatedWorktree = {
  ...worktreeOne,
  associationId: 'association-agent-two',
  worktreeId: 'worktree-agent-two',
  name: 'Agent checkout B',
  locationLabel: 'C:\\worktrees\\durable-b',
  provenance: 'user_associated_detached_checkout',
  observedStateFingerprint: 'fingerprint-b',
  changes: {
    commitsAheadOfBaseline: 1,
    commitsBehindBaseline: 0,
    stagedFiles: 0,
    unstagedFiles: 0,
    untrackedFiles: 3,
  },
  detachedHead: true,
};

export const passedBuild: ReviewBuild = {
  buildId: 'build-passed',
  name: 'Verified review build',
  branchRef: 'refs/heads/codex/durable-review',
  source: {
    kind: 'branch_commit',
    branchRef: 'refs/heads/codex/durable-review',
    objectId: tipCommit.objectId,
  },
  workspace: {
    worktreeId: 'worktree-build-owned',
    ownership: 'owned_build_worktree',
    locationLabel: 'Worktree Review storage / workspaces / build-passed',
    lifecycle: 'ready',
  },
  latestAttempt: {
    attemptId: 'attempt-passed',
    executionState: 'completed',
    verdict: 'passed',
    stage: 'complete',
    startedAt: '2026-08-21T11:00:00Z',
    completedAt: '2026-08-21T11:05:00Z',
  },
  artifact: {
    state: 'available',
    artifactSetId: 'artifact-passed',
    fileCount: 2,
    manifestHash: 'sha256:abcdef',
    storageLabel: 'AppData / artifacts / artifact-passed',
  },
  cleanup: { state: 'retained', policy: 'Newest successful build for this source' },
};

export const failedBuild: ReviewBuild = {
  ...passedBuild,
  buildId: 'build-failed',
  name: 'Failed rebuild',
  latestAttempt: {
    attemptId: 'attempt-failed',
    executionState: 'completed',
    verdict: 'failed',
    stage: 'compile',
    startedAt: '2026-08-22T11:00:00Z',
    completedAt: '2026-08-22T11:01:00Z',
    failure: { category: 'build', stage: 'compile', summary: 'Compiler exited with code 1.' },
  },
  artifact: { state: 'not_produced' },
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
        artifactStorage: { state: 'available' },
      },
    },
  ],
  branches: [
    {
      repositoryId: 'repository-one',
      branchRef: 'refs/heads/codex/durable-review',
      displayName: 'codex/durable-review',
      tip: tipCommit,
      aheadOfDefault: 3,
      behindDefault: 0,
      associatedWorktreeCount: 2,
    },
    {
      repositoryId: 'repository-one',
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
    applicationMetadata: [
      { label: 'Application', value: 'Codex Orchestrator', source: 'committed-branch-tip' },
      { label: 'Version', value: '0.1.0', source: 'committed-branch-tip' },
    ],
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
    builds: [failedBuild, passedBuild],
    ...overrides,
  };
}
