import { createTauriWorktreeReviewClient, type WorktreeReviewInvoke } from './tauriWorktreeReview';

describe('tauri Worktree Review client', () => {
  it('preserves typed branch, worktree, source, and workspace inputs at the transport boundary', async () => {
    const calls: { command: string; args?: Record<string, unknown> }[] = [];
    const invoke: WorktreeReviewInvoke = async <Result>(
      command: string,
      args?: Record<string, unknown>,
    ) => {
      calls.push({ command, args });
      return {} as Result;
    };
    const client = createTauriWorktreeReviewClient(invoke);

    await client.overview();
    await client.selectRepository('repository-one');
    const target = {
      kind: 'branch' as const,
      repositoryId: 'repository-one',
      branchRef: 'refs/heads/codex/review',
    };
    const query = { target, scope: { kind: 'ancestry' as const, tipObjectId: 'a'.repeat(40) } };
    await client.targetDetail(target);
    await client.commitHistory(query);
    await client.commitHistory(query, 'history-page-2');
    await client.associateWorktree({
      repositoryId: 'repository-one',
      branchRef: 'refs/heads/codex/review',
      worktreeId: 'worktree-one',
      baseline: { kind: 'selected_commit', objectId: 'abcdef' },
    });
    await client.createWorktree({
      repositoryId: 'repository-one',
      branchRef: 'refs/heads/codex/review',
    });
    await client.createBuild({
      repositoryId: 'repository-one',
      branchRef: 'refs/heads/codex/review',
      name: 'Snapshot build',
      profile: 'release',
      source: {
        kind: 'worktree_snapshot',
        associationId: 'association-one',
      },
      workspacePlan: {
        kind: 'create_owned_build_worktree',
        originatingAssociationId: 'association-one',
      },
    });
    await client.openBuild({ buildId: 'build-one' });

    expect(calls).toEqual([
      { command: 'worktree_review_overview', args: undefined },
      {
        command: 'select_worktree_review_repository',
        args: { input: { repositoryId: 'repository-one' } },
      },
      {
        command: 'worktree_review_target_detail',
        args: { input: target },
      },
      { command: 'worktree_review_commit_history', args: { input: { query } } },
      {
        command: 'worktree_review_commit_history',
        args: { input: { query, cursor: 'history-page-2' } },
      },
      {
        command: 'associate_worktree_review_worktree',
        args: {
          input: {
            repositoryId: 'repository-one',
            branchRef: 'refs/heads/codex/review',
            worktreeId: 'worktree-one',
            baseline: { kind: 'selected_commit', objectId: 'abcdef' },
          },
        },
      },
      {
        command: 'create_worktree_review_worktree',
        args: {
          input: {
            repositoryId: 'repository-one',
            branchRef: 'refs/heads/codex/review',
          },
        },
      },
      {
        command: 'create_worktree_review_build',
        args: {
          input: {
            repositoryId: 'repository-one',
            branchRef: 'refs/heads/codex/review',
            name: 'Snapshot build',
            profile: 'release',
            source: {
              kind: 'worktree_snapshot',
              associationId: 'association-one',
            },
            workspacePlan: {
              kind: 'create_owned_build_worktree',
              originatingAssociationId: 'association-one',
            },
          },
        },
      },
      {
        command: 'worktree_review_open_build',
        args: { input: { buildId: 'build-one' } },
      },
    ]);
  });
});
