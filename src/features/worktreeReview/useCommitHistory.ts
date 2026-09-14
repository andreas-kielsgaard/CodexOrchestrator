import { useCallback, useEffect, useRef, useState } from 'react';
import {
  uniqueCommits,
  type CommitHistoryQuery,
  type GitCommit,
  type WorktreeReviewClient,
} from '../../application/worktreeReview';

export function useCommitHistory(client: WorktreeReviewClient, query: CommitHistoryQuery | null) {
  const identity = JSON.stringify(query);
  const active = useRef(identity);
  active.current = identity;
  const request = useRef(0);
  const [state, setState] = useState<{
    key: string;
    commits: readonly GitCommit[];
    cursor: string | null;
    total: number;
    loading: boolean;
    error: string | null;
    loaded: boolean;
  }>({ key: '', commits: [], cursor: null, total: 0, loading: false, error: null, loaded: false });
  const current =
    state.key === identity
      ? state
      : {
          key: identity,
          commits: [],
          cursor: null,
          total: 0,
          loading: false,
          error: null,
          loaded: false,
        };
  useEffect(
    () => () => {
      request.current++;
    },
    [],
  );
  const load = useCallback(
    async (more = false) => {
      if (!query || (state.key === identity && state.loading)) return;
      if (!more && state.key === identity && state.loaded) return;
      const cursor = more && state.key === identity ? state.cursor : null;
      if (more && !cursor) return;
      const token = ++request.current;
      setState((previous) => ({
        key: identity,
        commits: previous.key === identity ? previous.commits : [],
        cursor,
        total: previous.key === identity ? previous.total : 0,
        loading: true,
        error: null,
        loaded: false,
      }));
      try {
        const page = await client.commitHistory(query, cursor ?? undefined);
        if (token !== request.current || active.current !== identity) return;
        setState((previous) => ({
          key: identity,
          commits: uniqueCommits([...(cursor ? previous.commits : []), ...page.commits]),
          cursor: page.nextCursor,
          total: page.totalCount,
          loading: false,
          error: null,
          loaded: true,
        }));
      } catch (cause) {
        if (token === request.current && active.current === identity)
          setState((previous) => ({ ...previous, loading: false, error: String(cause) }));
      }
    },
    [client, identity, query, state],
  );
  return {
    ...current,
    load: () => load(),
    loadMore: () => load(true),
    hasMore: Boolean(current.cursor),
  };
}
