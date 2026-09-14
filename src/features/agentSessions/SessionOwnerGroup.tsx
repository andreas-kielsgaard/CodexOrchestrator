import { ChevronDown, ChevronRight } from 'lucide-react';
import type { ReactNode } from 'react';
import type { NavigationEntry } from '../../application/agentSessions/navigationView';
import type { SessionNavigationController } from './useSessionNavigation';
import type { SessionNavigationDrag } from './useSessionNavigationDrag';

export function SessionOwnerGroup({
  entry,
  node,
  tree,
  drag,
  children,
}: {
  entry: NavigationEntry;
  node: Extract<NavigationEntry['node'], { kind: 'group' }>;
  tree: SessionNavigationController;
  drag: SessionNavigationDrag;
  children: ReactNode;
}) {
  const expanded = tree.expanded.has(entry.id);
  return (
    <div
      className="session-owner-group"
      aria-label={node.label}
      onClick={(event) => {
        event.stopPropagation();
        if (!drag.consumeClick()) {
          tree.focus(entry.id);
          tree.toggle(entry.id);
        }
      }}
    >
      <div
        id={entry.id}
        data-session-group={node.group}
        data-insertion={drag.indicator?.id === entry.id ? drag.indicator.side : undefined}
        ref={(element) => {
          if (element) tree.refs.current.set(entry.id, element);
          else tree.refs.current.delete(entry.id);
        }}
        role="treeitem"
        aria-level={entry.level}
        aria-label={node.label}
        aria-expanded={expanded}
        tabIndex={tree.activeId === entry.id ? 0 : -1}
        onFocus={(event) => {
          event.stopPropagation();
          tree.setFocusedId(entry.id);
        }}
        className={`session-tree-row session-owner-heading${!expanded && tree.selectedAncestors.has(entry.id) ? ' contains-selection' : ''}`}
      >
        {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        <span>
          {node.label}
          {expanded ? '' : '…'}
        </span>
      </div>
      {expanded && <div role="group">{children}</div>}
    </div>
  );
}
