import { useEffect, useRef, useState } from 'react';
import {
  targetKey,
  sourceTarget,
  type BranchGraphData,
  type GraphConnection,
  type ReviewBranch,
  type ReviewTarget,
  type WorktreeReviewClient,
} from '../../../application/worktreeReview';
import { ReviewDialog } from '../ReviewDialog';
import { BranchGraph } from './BranchGraph';
import { CommitRangeDialog } from './CommitRangeDialog';
import './branchSelection.css';

export function BranchGraphDialog({
  client,
  repositoryId,
  selectedTarget,
  onClose,
  onSelect,
}: {
  readonly client: WorktreeReviewClient;
  readonly repositoryId: string;
  readonly selectedTarget: ReviewTarget | null;
  readonly onClose: () => void;
  readonly onSelect: (target: ReviewTarget) => void;
}) {
  const [graph, setGraph] = useState<BranchGraphData>();
  const [selected, setSelected] = useState<ReviewBranch>();
  const [preview, setPreview] = useState<ReviewBranch>();
  const [range, setRange] = useState<{ segment: GraphConnection; source: ReviewBranch }>();
  const [limit, setLimit] = useState(1200);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();
  const [frameVersion, setFrameVersion] = useState(0);
  const snapshot = useRef<{ repositoryId: string; id: string } | undefined>(undefined);
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(undefined);
    client
      .branchGraph(
        repositoryId,
        limit,
        snapshot.current?.repositoryId === repositoryId ? snapshot.current.id : undefined,
      )
      .then((data) => {
        if (cancelled) return;
        if (data.snapshotId) snapshot.current = { repositoryId, id: data.snapshotId };
        setGraph(data);
        setSelected(
          (previous) =>
            data.targets.find((item) => {
              const source = previous?.target ?? (selectedTarget && sourceTarget(selectedTarget));
              return (
                source &&
                (targetKey(item.target) === targetKey(source) ||
                  (source.kind === 'worktree' && item.worktreeIds.includes(source.worktreeId)))
              );
            }) ?? data.targets[0],
        );
      })
      .catch((cause) => {
        if (!cancelled) setError(String(cause));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [client, repositoryId, limit, selectedTarget]);
  function openRange(segment: GraphConnection) {
    if (!graph) return;
    const candidates = [selected, ...graph.targets].filter((item): item is ReviewBranch =>
      Boolean(item),
    );
    const source = candidates.find((item) =>
      segment.eligibleSources.some((source) => targetKey(source) === targetKey(item.target)),
    );
    if (source) setRange({ segment, source });
  }
  return (
    <ReviewDialog labelledBy="branch-graph-title" onClose={onClose} className="branch-graph-dialog">
      <header className="branch-graph__header">
        <div>
          <p className="worktree-review__step">Repository history</p>
          <h2 id="branch-graph-title">Select branch</h2>
          <p>
            Current branches and their shared history. Hover to trace a branch; open a count to
            choose a commit.
          </p>
        </div>
        <button type="button" className="worktree-review__secondary" onClick={onClose}>
          Close
        </button>
      </header>
      <div className="branch-graph__toolbar">
        <span>{graph?.targets.length ?? 0} branches and detached worktrees</span>
        <div>
          <button
            type="button"
            className="branch-graph__center"
            onClick={() => setFrameVersion((value) => value + 1)}
          >
            Center selection
          </button>
        </div>
      </div>
      {error && <p role="alert">{error}</p>}
      {loading && <p role="status">Loading branch relationships…</p>}
      {graph && (
        <BranchGraph
          graph={graph}
          selected={selected}
          preview={preview}
          frameVersion={frameVersion}
          onPreview={setPreview}
          onSelect={setSelected}
          onRange={openRange}
        />
      )}
      <footer className="branch-graph__footer">
        <div>
          <strong>{(preview ?? selected)?.displayName ?? 'Choose a branch'}</strong>
          <p>{(preview ?? selected)?.tip.subject}</p>
          {graph?.hasMore && (
            <button
              type="button"
              className="worktree-review__secondary"
              disabled={loading || limit >= 20_000}
              onClick={() => setLimit((value) => Math.min(20_000, value + 2400))}
            >
              Load earlier relationships
            </button>
          )}
        </div>
        <button
          type="button"
          className="worktree-review__primary"
          disabled={!selected}
          onClick={() => selected && onSelect(selected.target)}
        >
          Use {selected?.target.kind === 'worktree' ? 'worktree' : 'branch'}
        </button>
      </footer>
      {range && (
        <CommitRangeDialog
          client={client}
          query={{ target: range.source.target, scope: range.segment.scope }}
          source={range.source}
          connection={range.segment}
          onClose={() => setRange(undefined)}
          onSelect={onSelect}
        />
      )}
    </ReviewDialog>
  );
}
