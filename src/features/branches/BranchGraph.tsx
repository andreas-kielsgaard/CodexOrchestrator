import {
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  Fragment,
  type CSSProperties,
  type KeyboardEvent,
} from 'react';
import { GitBranch, GitCommitHorizontal } from 'lucide-react';
import {
  targetKey,
  type BranchGraphData,
  type GraphConnection,
  type ReviewBranch,
} from '../../application/branches';
import {
  branchGraphLayout,
  grid,
  laneColors,
  rangeControl,
  targetControl,
  type Direction,
  type Point,
} from './branchGraphLayout';
import { branchLineage } from './branchLineage';

const path = (route: readonly Point[]) => {
  const points = route.filter((p, i) => !i || p.x !== route[i - 1].x || p.y !== route[i - 1].y);
  let d = `M ${points[0].x} ${points[0].y}`;
  for (let i = 1; i < points.length - 1; i++) {
    const previous = points[i - 1],
      p = points[i],
      next = points[i + 1];
    const before = Math.hypot(p.x - previous.x, p.y - previous.y),
      after = Math.hypot(next.x - p.x, next.y - p.y);
    const radius = Math.min(5, before / 2, after / 2);
    d += ` L ${p.x + ((previous.x - p.x) * radius) / before} ${p.y + ((previous.y - p.y) * radius) / before}`;
    d += ` Q ${p.x} ${p.y} ${p.x + ((next.x - p.x) * radius) / after} ${p.y + ((next.y - p.y) * radius) / after}`;
  }
  const last = points.at(-1)!;
  return `${d} L ${last.x} ${last.y}`;
};
const tint = (color: string): CSSProperties => ({ '--lane-color': color }) as CSSProperties;
export function BranchGraph({
  graph,
  selected,
  preview,
  frameVersion,
  onPreview,
  onSelect,
  onRange,
}: {
  readonly graph: BranchGraphData;
  readonly selected: ReviewBranch | undefined;
  readonly preview: ReviewBranch | undefined;
  readonly frameVersion: number;
  readonly onPreview: (target?: ReviewBranch) => void;
  readonly onSelect: (target: ReviewBranch) => void;
  readonly onRange?: (range: GraphConnection) => void;
}) {
  const layout = useMemo(() => branchGraphLayout(graph), [graph]);
  const lineage = useMemo(
    () => branchLineage(graph, (preview ?? selected)?.tip.objectId),
    [graph, preview, selected],
  );
  const viewport = useRef<HTMLDivElement>(null);
  const controls = useRef(new Map<string, HTMLButtonElement>());
  const [focused, setFocused] = useState<string>();
  const [tooltip, setTooltip] = useState<{
    target: ReviewBranch;
    left: number;
    top: number;
    above: boolean;
  }>();
  const [traced, setTraced] = useState<string>();
  const selectedKey = selected && targetKey(selected.target);
  const entry =
    focused && layout.neighbors.has(focused)
      ? focused
      : selectedKey
        ? targetControl(selectedKey)
        : layout.neighbors.keys().next().value;
  const previous = useRef<{ selectedKey?: string; frameVersion: number; y: number } | undefined>(
    undefined,
  );
  useLayoutEffect(() => {
    const element = viewport.current,
      label = selectedKey && layout.labels.get(selectedKey);
    if (!element || !label) return;
    const prior = previous.current;
    if (!prior || prior.selectedKey !== selectedKey || prior.frameVersion !== frameVersion) {
      element.scrollTop = Math.max(0, label.y - element.clientHeight * 0.35);
    } else element.scrollTop += label.y - prior.y;
    previous.current = { selectedKey, frameVersion, y: label.y };
  }, [layout, selectedKey, frameVersion]);
  function keyboard(event: KeyboardEvent<HTMLDivElement>) {
    if (!event.key.startsWith('Arrow')) return;
    const id = (event.target as HTMLElement).dataset.graphControl;
    const next = id && layout.neighbors.get(id)?.[event.key as Direction];
    if (!next) return;
    event.preventDefault();
    const button = controls.current.get(next);
    button?.focus({ preventScroll: true });
    if (!button || !viewport.current) return;
    const box = button.getBoundingClientRect(),
      view = viewport.current.getBoundingClientRect();
    if (box.top < view.top + 36) viewport.current.scrollTop -= view.top + 36 - box.top + 8;
    if (box.bottom > view.bottom) viewport.current.scrollTop += box.bottom - view.bottom + 8;
    if (box.left < view.left) viewport.current.scrollLeft -= view.left - box.left + 8;
    if (box.right > view.right) viewport.current.scrollLeft += box.right - view.right + 8;
  }
  function showDetails(target: ReviewBranch, button: HTMLButtonElement) {
    onPreview(target);
    const box = button.getBoundingClientRect();
    const view = viewport.current?.getBoundingClientRect();
    const above = box.bottom + 130 > (view?.bottom ?? window.innerHeight);
    setTooltip({
      target,
      above,
      left: Math.min(box.left, (view?.right ?? window.innerWidth) - 330),
      top: above ? box.top - 6 : box.bottom + 6,
    });
  }
  function clearDetails() {
    onPreview();
    setTooltip(undefined);
  }
  const controlProps = (id: string) => ({
    'data-graph-control': id,
    tabIndex: id === entry ? 0 : -1,
    ref: (element: HTMLButtonElement | null) => {
      if (element) controls.current.set(id, element);
      else controls.current.delete(id);
    },
  });
  return (
    <div
      ref={viewport}
      className="branch-graph__viewport"
      role="group"
      aria-label="Repository branch graph"
      aria-description="Use arrow keys to explore branch names and commit ranges. Hover or focus a branch for worktree details."
      onKeyDown={keyboard}
      onScroll={() => {
        const active = document.activeElement as HTMLButtonElement | null;
        const target = graph.targets.find(
          (t) => targetControl(targetKey(t.target)) === active?.dataset.graphControl,
        );
        if (active && target) showDetails(target, active);
        else setTooltip(undefined);
      }}
    >
      <div className="branch-graph__grid" style={{ minWidth: layout.width, height: layout.height }}>
        <div className="branch-graph__headings" aria-hidden="true">
          <span style={{ width: layout.graphWidth }}>RECENT ↓ EARLIER</span>
          <span style={{ width: grid.count + 20 }}>COMMITS</span>
          <span>BRANCH / WORKTREE</span>
        </div>
        <svg
          width={layout.graphWidth}
          height={layout.height}
          className="branch-graph__canvas"
          aria-hidden="true"
        >
          {graph.connections.map((edge) => (
            <Fragment key={edge.id}>
              <path
                d={path(layout.routes.get(edge.id)!)}
                className="branch-graph__track-clearance"
              />
              <path
                d={path(layout.routes.get(edge.id)!)}
                style={tint(layout.colors.get(edge.id)!)}
                className={`branch-graph__connection${lineage.edges.has(edge.id) ? ' is-highlighted' : ''}${traced === edge.id ? ' is-traced' : ''}`}
              />
            </Fragment>
          ))}
          {graph.anchors.map((node) => {
            const p = layout.positions.get(node.objectId)!;
            return (
              <g
                key={node.objectId}
                style={tint(laneColors[p.lane % laneColors.length])}
                className={`branch-graph__anchor${lineage.nodes.has(node.objectId) ? ' is-highlighted' : ''}`}
              >
                <circle
                  cx={p.x}
                  cy={p.y}
                  r={graph.targets.some((t) => t.tip.objectId === node.objectId) ? 5 : 3.5}
                  className={node.boundary ? 'is-boundary' : ''}
                />
                <title>{node.boundary ? 'Earlier relationship not loaded' : node.objectId}</title>
              </g>
            );
          })}
        </svg>
        {graph.anchors
          .filter((node) => !graph.targets.some((t) => t.tip.objectId === node.objectId))
          .map((node) => (
            <span
              key={node.objectId}
              className="branch-graph__base"
              style={{ left: layout.labelX, top: layout.positions.get(node.objectId)!.y - 16 }}
            >
              {node.boundary ? 'Earlier relationship' : 'Shared base'} ·{' '}
              <code>{node.objectId.slice(0, 8)}</code>
            </span>
          ))}
        {graph.connections.map((edge) => {
          const id = rangeControl(edge.id),
            rect = layout.ranges.get(edge.id)!;
          const count = `${edge.commitCount}${edge.incomplete ? '+' : ''} ${edge.commitCount === 1 && !edge.incomplete ? 'commit' : 'commits'}`;
          return (
            <button
              key={id}
              {...controlProps(id)}
              type="button"
              className={`branch-graph__range${lineage.edges.has(edge.id) ? ' is-highlighted' : ''}`}
              style={{
                ...tint(layout.colors.get(edge.id)!),
                left: rect.x,
                top: rect.y,
                width: rect.width,
                height: rect.height,
              }}
              aria-label={`${count} from ${edge.from.slice(0, 8)} to ${edge.to.slice(0, 8)}`}
              title={`${edge.from.slice(0, 8)} → ${edge.to.slice(0, 8)}. ${edge.incomplete ? 'Loaded history only. ' : ''}Includes merged history; excludes the older state and its ancestors.`}
              onFocus={() => {
                setFocused(id);
                clearDetails();
                setTraced(edge.id);
              }}
              onBlur={() => setTraced(undefined)}
              onMouseEnter={() => setTraced(edge.id)}
              onMouseLeave={() => setTraced(undefined)}
              onClick={() => onRange?.(edge)}
              disabled={!onRange}
            >
              <span aria-hidden="true" className="branch-graph__range-dot" />
              {count}
            </button>
          );
        })}
        {graph.targets.map((target) => {
          const key = targetKey(target.target),
            id = targetControl(key),
            rect = layout.labels.get(key)!;
          const color =
            laneColors[layout.positions.get(target.tip.objectId)!.lane % laneColors.length];
          const Icon = target.target.kind === 'worktree' ? GitCommitHorizontal : GitBranch;
          return (
            <button
              key={id}
              {...controlProps(id)}
              type="button"
              aria-pressed={key === selectedKey}
              aria-describedby={tooltip?.target === target ? 'branch-graph-details' : undefined}
              className={`branch-graph__target${key === selectedKey ? ' is-selected' : ''}${lineage.nodes.has(target.tip.objectId) ? ' is-related' : ''}`}
              style={{
                ...tint(color),
                left: rect.x,
                top: rect.y,
                width: 'max-content',
                maxWidth: `calc(100% - ${rect.x + 16}px)`,
                height: rect.height,
              }}
              onMouseEnter={(e) => showDetails(target, e.currentTarget)}
              onMouseLeave={clearDetails}
              onFocus={(e) => {
                setFocused(id);
                showDetails(target, e.currentTarget);
              }}
              onBlur={clearDetails}
              onClick={() => onSelect(target)}
            >
              <Icon size={16} aria-hidden="true" />
              <span>{target.displayName}</span>
            </button>
          );
        })}
      </div>
      {tooltip && (
        <div
          id="branch-graph-details"
          role="tooltip"
          className="branch-graph__tooltip"
          style={{
            left: tooltip.left,
            top: tooltip.top,
            transform: tooltip.above ? 'translateY(-100%)' : undefined,
          }}
        >
          <strong>{tooltip.target.displayName}</strong>
          <span>
            {tooltip.target.availableWorktreeCount}{' '}
            {tooltip.target.availableWorktreeCount === 1
              ? 'worktree instance'
              : 'worktree instances'}
            {tooltip.target.target.kind === 'worktree' ? ' · detached' : ''}
          </span>
          <span>
            {tooltip.target.activity
              ? `${tooltip.target.activity.stagedFiles + tooltip.target.activity.unstagedFiles} changed · ${tooltip.target.activity.untrackedFiles} new`
              : tooltip.target.availableWorktreeCount
                ? 'Changes not checked'
                : 'Committed state'}
          </span>
          <code>{tooltip.target.tip.abbreviatedObjectId}</code>
        </div>
      )}
    </div>
  );
}
