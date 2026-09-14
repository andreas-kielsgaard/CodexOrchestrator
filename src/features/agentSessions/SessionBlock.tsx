import { ChevronDown, ChevronRight, Folder, SquarePen } from 'lucide-react';
import type { ReactNode } from 'react';
import type { SessionNavigationFolder } from '../../application/agentSessions/navigation';
import type { SessionFolderTarget } from '../../application/agentSessions/organization';
import type { NavigationEntry } from '../../application/agentSessions/navigationView';
import type { SessionNavigationController } from './useSessionNavigation';
import type { SessionNavigationDrag } from './useSessionNavigationDrag';

export function SessionBlock({
  entry,
  node,
  tree,
  drag,
  children,
  onNew,
}: {
  entry: NavigationEntry;
  node: SessionNavigationFolder;
  tree: SessionNavigationController;
  drag: SessionNavigationDrag;
  children: ReactNode;
  onNew(target: SessionFolderTarget | null): void;
}) {
  const expanded = tree.expanded.has(node.id);
  const toggle = () => {
    if (!drag.consumeClick()) {
      tree.focus(node.id);
      tree.toggle(node.id);
    }
  };
  const indicator = drag.indicator?.id === node.id ? drag.indicator.side : undefined;
  return (
    <div
      className={`session-container session-container--${node.role}`}
      data-drop-target={indicator === 'inside' || undefined}
      {...drag.dropProps(node.id)}
      onClick={(event) => {
        event.stopPropagation();
        toggle();
      }}
    >
      <div
        ref={(element) => {
          if (element) tree.refs.current.set(node.id, element);
          else tree.refs.current.delete(node.id);
        }}
        role="treeitem"
        aria-label={node.label}
        aria-level={entry.level}
        aria-expanded={expanded}
        tabIndex={tree.activeId === node.id ? 0 : -1}
        className="session-tree-row session-block-heading"
        data-insertion={indicator === 'before' || indicator === 'after' ? indicator : undefined}
        onFocus={(event) => {
          event.stopPropagation();
          tree.setFocusedId(node.id);
        }}
        {...drag.headerProps(node)}
      >
        {expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
        {node.role === 'repository' && <Folder size={16} />}
        <span className="session-tree-label">
          {node.label}
          {!expanded && node.role === 'section' ? '…' : ''}
        </span>
        <button
          className="folder-create session-icon-button"
          aria-label={`New session in ${node.label}`}
          draggable={false}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => {
            event.stopPropagation();
            onNew(node.createTarget);
          }}
        >
          <SquarePen size={15} />
        </button>
      </div>
      {expanded && (
        <div role="group" className="session-block-content">
          {children}
        </div>
      )}
    </div>
  );
}
