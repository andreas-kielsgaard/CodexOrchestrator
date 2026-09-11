import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  orderReviewTargets,
  targetKey,
  type BranchReviewDetail,
  type ReviewTarget,
  type WorktreeReviewClient,
  type WorktreeReviewOverview,
  type WorktreeActivity,
} from '../../application/worktreeReview';

export function useReviewSelection(
  client: WorktreeReviewClient,
  onSelected: (detail: BranchReviewDetail, activeWorktreeId?: string) => void,
) {
  const [overview, setOverview] = useState<WorktreeReviewOverview>({
    repositories: [],
    branches: [],
  });
  const [target, setTarget] = useState<ReviewTarget | null>(null);
  const [detail, setDetail] = useState<BranchReviewDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const activityGeneration = useRef(0);
  const targetRef = useRef(target);
  const overviewRef = useRef(overview);
  targetRef.current = target;
  overviewRef.current = overview;

  const discoverActivity = useCallback(
    async (snapshot: WorktreeReviewOverview) => {
      const token = ++activityGeneration.current;
      const repositoryId = snapshot.selectedRepositoryId;
      if (!repositoryId) return;
      const ids = [...new Set(snapshot.branches.flatMap((branch) => branch.worktreeIds))];
      const estimates = new Map<string, WorktreeActivity>();
      for (let offset = 0; offset < ids.length; offset += 4) {
        if (token !== activityGeneration.current) return;
        try {
          const batch = await client.worktreeActivity(repositoryId, ids.slice(offset, offset + 4));
          if (token !== activityGeneration.current) return;
          batch.forEach((item) => estimates.set(item.worktreeId, item));
          setOverview((previous) =>
            previous.selectedRepositoryId !== repositoryId
              ? previous
              : {
                  ...previous,
                  branches: previous.branches.map((branch) => {
                    const candidates = branch.worktreeIds.flatMap((id) =>
                      estimates.has(id) ? [estimates.get(id)!.estimate] : [],
                    );
                    if (branch.activity) candidates.push(branch.activity);
                    const activity =
                      candidates.sort(
                        (a, b) => Date.parse(b.changedAt) - Date.parse(a.changedAt),
                      )[0] ?? null;
                    return { ...branch, activity };
                  }),
                },
          );
        } catch {
          /* Approximate recency retains the commit fallback when a checkout is unavailable. */
        }
      }
    },
    [client],
  );

  const acceptDetail = useCallback(
    async (
      next: ReviewTarget,
      snapshot: WorktreeReviewOverview,
      token: number,
      resetDraft: boolean,
    ) => {
      const result = await client.targetDetail(next);
      if (token !== generation.current) return;
      setDetail(result);
      setTarget(next);
      setOverview((previous) => ({
        ...previous,
        branches: previous.branches.map((branch) =>
          targetKey(branch.target) === targetKey(next) ? result.branch : branch,
        ),
      }));
      if (resetDraft) onSelected(result, snapshot.activeBuildContext?.worktreeId);
    },
    [client, onSelected],
  );

  const loadOverview = useCallback(
    async (repositoryId?: string, preserve = false) => {
      const token = ++generation.current;
      activityGeneration.current++;
      const previousTarget = preserve ? targetRef.current : null;
      setLoading(true);
      setError(null);
      if (!preserve) {
        setTarget(null);
        setDetail(null);
      }
      try {
        const snapshot = repositoryId
          ? await client.selectRepository(repositoryId)
          : await client.overview();
        if (token !== generation.current) return;
        setOverview(snapshot);
        overviewRef.current = snapshot;
        const targets = orderReviewTargets(snapshot.branches);
        const next =
          previousTarget &&
          previousTarget.repositoryId === snapshot.selectedRepositoryId &&
          (previousTarget.kind === 'commit' ||
            (previousTarget.kind === 'worktree' &&
              targets.some((item) => item.worktreeIds.includes(previousTarget.worktreeId))) ||
            targets.some((item) => targetKey(item.target) === targetKey(previousTarget)))
            ? previousTarget
            : targets[0]?.target;
        if (next)
          await acceptDetail(
            next,
            snapshot,
            token,
            !previousTarget || targetKey(next) !== targetKey(previousTarget),
          );
        else {
          setTarget(null);
          setDetail(null);
        }
        if (token === generation.current) void discoverActivity(snapshot);
      } catch (cause) {
        if (token === generation.current) {
          setDetail(null);
          setError(String(cause));
        }
      } finally {
        if (token === generation.current) setLoading(false);
      }
    },
    [client, acceptDetail, discoverActivity],
  );

  const selectTarget = useCallback(
    async (next: ReviewTarget) => {
      const token = ++generation.current;
      setTarget(next);
      targetRef.current = next;
      setDetail(null);
      setLoading(true);
      setError(null);
      try {
        await acceptDetail(next, overviewRef.current, token, true);
      } catch (cause) {
        if (token === generation.current) setError(String(cause));
      } finally {
        if (token === generation.current) setLoading(false);
      }
    },
    [acceptDetail],
  );

  useEffect(() => {
    void loadOverview();
    return () => {
      // These are request counters, not DOM refs; invalidate all work on unmount.
      // eslint-disable-next-line react-hooks/exhaustive-deps
      generation.current++;
      // eslint-disable-next-line react-hooks/exhaustive-deps
      activityGeneration.current++;
    };
  }, [loadOverview]);
  const branches = useMemo(() => orderReviewTargets(overview.branches), [overview.branches]);
  return {
    overview,
    branches,
    target,
    detail,
    loading,
    error,
    selectTarget,
    selectRepository: (id: string) => loadOverview(id),
    refresh: () => loadOverview(undefined, true),
  };
}
