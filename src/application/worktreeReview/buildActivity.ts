import type { BuildId, ReviewBuild } from './contracts';

export interface WorktreeReviewBuildActivitySnapshot {
  readonly unreadBuilds: readonly ReviewBuild[];
}

export interface WorktreeReviewBuildActivitySource {
  getSnapshot(): WorktreeReviewBuildActivitySnapshot;
  subscribe(listener: () => void): () => void;
  markRead(buildIds: readonly BuildId[]): void;
}

export class InMemoryWorktreeReviewBuildActivitySource
  implements WorktreeReviewBuildActivitySource
{
  private snapshot: WorktreeReviewBuildActivitySnapshot = { unreadBuilds: [] };
  private readonly listeners = new Set<() => void>();

  getSnapshot = () => this.snapshot;

  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  record(build: ReviewBuild) {
    const unreadBuilds = [
      build,
      ...this.snapshot.unreadBuilds.filter((entry) => entry.buildId !== build.buildId),
    ];
    this.publish({ unreadBuilds });
  }

  markRead = (buildIds: readonly BuildId[]) => {
    if (buildIds.length === 0) return;
    const ids = new Set(buildIds);
    const unreadBuilds = this.snapshot.unreadBuilds.filter((build) => !ids.has(build.buildId));
    if (unreadBuilds.length !== this.snapshot.unreadBuilds.length) this.publish({ unreadBuilds });
  };

  private publish(snapshot: WorktreeReviewBuildActivitySnapshot) {
    this.snapshot = snapshot;
    this.listeners.forEach((listener) => listener());
  }
}
