import { describe, expect, it, vi } from 'vitest';
import { completedBuild } from '../../features/worktreeReview/WorktreeReviewScreen.fixtures';
import { InMemoryWorktreeReviewBuildActivitySource } from './buildActivity';

describe('InMemoryWorktreeReviewBuildActivitySource', () => {
  it('keeps one unread receipt per build and clears only requested builds', () => {
    const source = new InMemoryWorktreeReviewBuildActivitySource();
    const changed = vi.fn();
    source.subscribe(changed);

    source.record(completedBuild);
    source.record({ ...completedBuild, name: 'Updated terminal receipt' });
    expect(source.getSnapshot().unreadBuilds).toEqual([
      { ...completedBuild, name: 'Updated terminal receipt' },
    ]);
    expect(changed).toHaveBeenCalledTimes(2);

    source.markRead([completedBuild.buildId]);
    expect(source.getSnapshot().unreadBuilds).toEqual([]);
  });
});
