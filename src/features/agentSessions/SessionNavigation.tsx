import { SessionOwnerGroup } from './SessionOwnerGroup';
import type { SessionWorkflowTarget } from '../../application/agentSessions/workflowNavigation';
import { SquarePen } from 'lucide-react';
import { useCallback, useRef, useState, type KeyboardEvent } from 'react';
import type {
  SessionNavigationModel,
  SessionNavigationRow,
} from '../../application/agentSessions/navigation';
import type {
  SessionFolderTarget,
  SessionPlacement,
} from '../../application/agentSessions/organization';
import {
  navigationSiblings,
  type NavigationContent,
} from '../../application/agentSessions/navigationView';
import {
  insertNavigationSibling,
  type NavigationOrderScope,
} from '../../application/agentSessions/navigationOrder';
import { formatSessionDeepLink } from '../../application/agentSessions/deepLinks';
import { SessionContextMenu } from './SessionContextMenu';
import { browserAgentSessionClipboard } from './sessionClipboard';
import type { SessionNavigationController } from './useSessionNavigation';
import { useSessionNavigationDrag } from './useSessionNavigationDrag';
import { SessionBlock } from './SessionBlock';
import { SessionEntry } from './SessionEntry';

export function SessionNavigation({
  model,
  selectedSessionId,
  tree,
  onSelect,
  onNew,
  onOpenWorkflow,
  onMove,
  onPin,
  onReorder,
  organizing = true,
}: {
  model: SessionNavigationModel;
  selectedSessionId: string | null;
  tree: SessionNavigationController;
  onSelect(id: string): void;
  onOpenWorkflow?(target: SessionWorkflowTarget): void;
  onNew(target: SessionFolderTarget | null): void;
  onMove(id: string, target: SessionPlacement, orderedIds?: readonly string[]): Promise<void>;
  onPin(id: string, pinned: boolean): Promise<void>;
  onReorder(scope: NavigationOrderScope, ids: readonly string[]): Promise<void>;
  organizing?: boolean;
}) {
  const [menu, setMenu] = useState<{ row: SessionNavigationRow; x: number; y: number } | null>(
    null,
  );
  const menuOrigin = useRef<HTMLElement | null>(null);
  const drag = useSessionNavigationDrag(model, organizing, onMove, onReorder);
  const openMenu = (row: SessionNavigationRow, x: number, y: number) => {
    menuOrigin.current = tree.refs.current.get(row.id) ?? null;
    setMenu({ row, x, y });
  };
  const closeMenu = useCallback(() => {
    setMenu(null);
    menuOrigin.current?.focus();
  }, []);
  const keyDown = (event: KeyboardEvent) => {
    if (event.target instanceof HTMLButtonElement) return;
    const index = tree.rows.findIndex((row) => row.id === tree.activeId),
      row = tree.rows[index];
    if (!row) return;
    if (
      event.altKey &&
      (event.key === 'ArrowDown' || event.key === 'ArrowUp') &&
      row.node.kind === 'folder' &&
      organizing
    ) {
      const order = row.node.order,
        siblings = navigationSiblings(model, order.scope);
      const target = siblings[siblings.indexOf(order.id) + (event.key === 'ArrowDown' ? 1 : -1)];
      if (target)
        void onReorder(
          order.scope,
          insertNavigationSibling(
            siblings,
            order.id,
            target,
            event.key === 'ArrowDown' ? 'after' : 'before',
          ),
        );
    } else if (event.key === 'ArrowDown') tree.focus(tree.rows[index + 1]?.id);
    else if (event.key === 'ArrowUp') tree.focus(tree.rows[index - 1]?.id);
    else if (event.key === 'Home') tree.focus(tree.rows[0]?.id);
    else if (event.key === 'End') tree.focus(tree.rows.at(-1)?.id);
    else if (
      event.key === 'ArrowRight' &&
      (row.node.kind === 'folder' || row.node.kind === 'group')
    ) {
      if (!tree.expanded.has(row.id)) tree.toggle(row.id);
      else
        tree.focus(tree.rows[index + 1]?.parentId === row.id ? tree.rows[index + 1].id : undefined);
    } else if (event.key === 'ArrowLeft') {
      if ((row.node.kind === 'folder' || row.node.kind === 'group') && tree.expanded.has(row.id))
        tree.toggle(row.id);
      else tree.focus(row.parentId ?? undefined);
    } else if (
      (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) &&
      row.node.kind === 'session'
    ) {
      const bounds = tree.refs.current.get(row.id)!.getBoundingClientRect();
      openMenu(row.node, bounds.left + 20, bounds.bottom);
    } else if (event.key === 'Enter' || event.key === ' ') {
      if (row.node.kind === 'folder' || row.node.kind === 'group') tree.toggle(row.id);
      else if (row.node.kind === 'more') tree.showMore(row.node.folderId);
      else {
        if (row.sectionId === 'pinned') tree.preserveDisclosureFor(row.node.summary.id);
        onSelect(row.node.summary.id);
      }
    } else return;
    event.preventDefault();
    event.stopPropagation();
  };
  const render = (content: readonly NavigationContent[]): React.ReactNode =>
    content.map((entry) => {
      const node = entry.node;
      if (node.kind === 'group')
        return (
          <SessionOwnerGroup key={entry.id} entry={entry} node={node} tree={tree} drag={drag}>
            {render(entry.children)}
          </SessionOwnerGroup>
        );
      if (node.kind === 'folder')
        return (
          <SessionBlock
            key={entry.id}
            entry={entry}
            node={node}
            tree={tree}
            drag={drag}
            onNew={onNew}
            onOpenWorkflow={onOpenWorkflow}
          >
            {render(entry.children)}
          </SessionBlock>
        );
      if (node.kind === 'session')
        return (
          <SessionEntry
            key={entry.id}
            entry={entry}
            node={node}
            tree={tree}
            drag={drag}
            selected={node.summary.id === selectedSessionId}
            organizing={organizing}
            onSelect={onSelect}
            onPin={onPin}
            onMenu={openMenu}
            onOpenWorkflow={onOpenWorkflow}
          />
        );
      return (
        <div
          key={entry.id}
          ref={(element) => {
            if (element) tree.refs.current.set(entry.id, element);
            else tree.refs.current.delete(entry.id);
          }}
          role="treeitem"
          aria-label={`Show more in ${node.label}`}
          aria-level={entry.level}
          tabIndex={tree.activeId === entry.id ? 0 : -1}
          className="session-tree-row session-show-more"
          onFocus={() => tree.setFocusedId(entry.id)}
          onClick={(event) => {
            event.stopPropagation();
            tree.showMore(node.folderId);
          }}
        >
          Show more ({node.remaining})
        </div>
      );
    });
  return (
    <>
      {tree.view.sections.map((section) => (
        <section
          key={section.id}
          className="session-tree-section"
          {...(section.id !== 'repositories' ? drag.dropProps(section.id) : {})}
        >
          <div
            className="session-section-heading"
            data-insertion={drag.indicator?.id === section.id ? drag.indicator.side : undefined}
          >
            <h2>{section.label}</h2>
            {section.id === 'unfiled' && (
              <button
                className="folder-create session-icon-button"
                aria-label="New session in Unfiled"
                onClick={() => onNew(null)}
              >
                <SquarePen size={15} />
              </button>
            )}
          </div>
          <div role="tree" aria-label={`${section.label} sessions`} onKeyDown={keyDown}>
            {render(section.content)}
            {section.children.length === 0 && (
              <p className="session-list-empty">
                {section.id === 'pinned'
                  ? 'No pinned sessions.'
                  : section.id === 'repositories'
                    ? 'No registered repositories.'
                    : 'No unfiled sessions.'}
              </p>
            )}
          </div>
        </section>
      ))}
      {menu && (
        <SessionContextMenu
          session={menu.row}
          model={model}
          position={menu}
          organizing={organizing}
          onClose={closeMenu}
          onMove={(placement) => {
            void onMove(menu.row.summary.id, placement);
            closeMenu();
          }}
          onPin={() => {
            void onPin(menu.row.summary.id, !menu.row.pinned);
            closeMenu();
          }}
          onCopy={() =>
            browserAgentSessionClipboard.writeText(formatSessionDeepLink(menu.row.summary.id))
          }
        />
      )}
    </>
  );
}
