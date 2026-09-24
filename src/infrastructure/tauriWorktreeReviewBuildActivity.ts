import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  InMemoryWorktreeReviewBuildActivitySource,
  type WorktreeReviewBuildActivitySource,
} from '../application/worktreeReview/buildActivity';
import type { ReviewBuild } from '../application/worktreeReview';

const TERMINAL_EVENT = 'worktree-review://build-terminal';

export function createTauriWorktreeReviewBuildActivitySource(
  subscribe: (
    event: string,
    listener: (payload: ReviewBuild) => void,
  ) => Promise<UnlistenFn> = async (event, listener) =>
    listen<ReviewBuild>(event, ({ payload }) => listener(payload)),
): WorktreeReviewBuildActivitySource {
  const source = new InMemoryWorktreeReviewBuildActivitySource();
  let listeners = 0;
  let unlisten: UnlistenFn | undefined;
  let subscription: Promise<UnlistenFn> | undefined;
  return {
    getSnapshot: source.getSnapshot,
    markRead: source.markRead,
    subscribe(listener) {
      listeners += 1;
      const detach = source.subscribe(listener);
      subscription ??= subscribe(TERMINAL_EVENT, (build) => source.record(build))
        .then((dispose) => {
          if (listeners === 0) {
            dispose();
            subscription = undefined;
            return dispose;
          }
          unlisten = dispose;
          return dispose;
        })
        .catch(() => {
          subscription = undefined;
          return () => undefined;
        });
      return () => {
        detach();
        listeners -= 1;
        if (listeners === 0 && unlisten) {
          unlisten();
          unlisten = undefined;
          subscription = undefined;
        }
      };
    },
  };
}

export const tauriWorktreeReviewBuildActivity =
  createTauriWorktreeReviewBuildActivitySource();
