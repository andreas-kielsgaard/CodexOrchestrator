import { Pin } from 'lucide-react';
import type { SessionNavigationRow } from '../../application/agentSessions/navigation';
import type { NavigationEntry } from '../../application/agentSessions/navigationView';
import type { SessionNavigationController } from './useSessionNavigation';
import type { SessionNavigationDrag } from './useSessionNavigationDrag';
import { pendingRequestLabel } from './sessionAttention';

export function SessionEntry({
  entry,
  node,
  tree,
  drag,
  selected,
  organizing,
  onSelect,
  onPin,
  onMenu,
}: {
  entry: NavigationEntry;
  node: SessionNavigationRow;
  tree: SessionNavigationController;
  drag: SessionNavigationDrag;
  selected: boolean;
  organizing: boolean;
  onSelect(id: string): void;
  onPin(id: string, pinned: boolean): Promise<void>;
  onMenu(row: SessionNavigationRow, x: number, y: number): void;
}) {
  return (
    <div
      ref={(element) => {
        if (element) tree.refs.current.set(node.id, element);
        else tree.refs.current.delete(node.id);
      }}
      role="treeitem"
      aria-label={node.summary.title}
      aria-level={entry.level}
      aria-selected={selected}
      tabIndex={tree.activeId === node.id ? 0 : -1}
      className={`session-tree-row session-entry${selected ? ' is-selected' : ''}`}
      title={node.ownerLabel ? `${node.summary.title}\n${node.ownerLabel}` : node.summary.title}
      onFocus={() => tree.setFocusedId(node.id)}
      onClick={(event) => {
        event.stopPropagation();
        if (drag.consumeClick()) return;
        tree.setFocusedId(node.id);
        onSelect(node.summary.id);
      }}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onMenu(node, event.clientX, event.clientY);
      }}
      {...drag.sessionProps(node.summary.id)}
    >
      <span className="session-tree-label">{node.summary.title}</span>
      {organizing && (
        <button
          className={`session-icon-button session-pin${node.pinned ? ' is-pinned' : ''}`}
          aria-label={`${node.pinned ? 'Unpin' : 'Pin'} ${node.summary.title}`}
          aria-pressed={node.pinned}
          draggable={false}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => {
            event.stopPropagation();
            void onPin(node.summary.id, !node.pinned);
          }}
        >
          <Pin size={14} />
        </button>
      )}
      {node.summary.pendingRequestCount > 0 ? (
        <span
          className="session-attention"
          title={pendingRequestLabel}
          aria-label={pendingRequestLabel}
        />
      ) : node.summary.hasActiveInvocation ? (
        <span className="session-active" aria-label="Working" />
      ) : null}
    </div>
  );
}
