import { useEffect, useRef, useState } from 'react';
import {
  targetKey,
  sourceTarget,
  type BranchGraphData,
  type GraphConnection,
  type ReviewBranch,
  type ReviewTarget,
  type BranchReadSource,
} from '../../application/branches';
import { BranchGraph } from './BranchGraph';
import './branchGraph.css';
export function BranchGraphBrowser({
  source,
  repositoryId,
  selectedTarget,
  onSelect,
  onRange,
  branchesOnly = false,
}: {
  readonly source: BranchReadSource;
  readonly repositoryId: string;
  readonly selectedTarget: ReviewTarget | null;
  readonly branchesOnly?: boolean;
  readonly onSelect: (target: ReviewTarget) => void;
  readonly onRange?: (segment: GraphConnection, source: ReviewBranch) => void;
}) {
  const [graph, setGraph] = useState<BranchGraphData>();
  const [selected, setSelected] = useState<ReviewBranch>();
  const [preview, setPreview] = useState<ReviewBranch>();
  const [limit, setLimit] = useState(1200);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();
  const [frameVersion, setFrameVersion] = useState(0);
  const snapshot = useRef<{ repositoryId: string; id: string } | undefined>(undefined);
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(undefined);
    source
      .branchGraph(
        repositoryId,
        limit,
        snapshot.current?.repositoryId === repositoryId ? snapshot.current.id : undefined,
      )
      .then((data) => {
        if (cancelled) return;
        if (data.snapshotId) snapshot.current = { repositoryId, id: data.snapshotId };
        const next = branchesOnly
          ? { ...data, targets: data.targets.filter((item) => item.target.kind === 'branch') }
          : data;
        setGraph(next);
        setSelected(
          (previous) =>
            next.targets.find((item) => {
              const source = previous?.target ?? (selectedTarget && sourceTarget(selectedTarget));
              return (
                source &&
                (targetKey(item.target) === targetKey(source) ||
                  (source.kind === 'worktree' && item.worktreeIds.includes(source.worktreeId)))
              );
            }) ?? next.targets[0],
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
  }, [source, repositoryId, limit, selectedTarget, branchesOnly]);
  return (
    <section className="branch-graph-browser" aria-label="Branch history">
      <div className="branch-graph__toolbar">
        <span>
          {graph?.targets.length ?? 0}{' '}
          {branchesOnly ? 'branches' : 'branches and detached worktrees'}
        </span>
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
          onRange={
            onRange
              ? (segment) => {
                  const candidates = [selected, ...graph.targets].filter(
                    (item): item is ReviewBranch => Boolean(item),
                  );
                  const branch = candidates.find((item) =>
                    segment.eligibleSources.some(
                      (target) => targetKey(target) === targetKey(item.target),
                    ),
                  );
                  if (branch) onRange(segment, branch);
                }
              : undefined
          }
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
    </section>
  );
}
