import {
  ChevronDown,
  ChevronRight,
  Folder,
  MessageSquare,
  MessageSquarePlus,
  Pin,
} from 'lucide-react';
import { useCallback, useRef, useState, type CSSProperties, type KeyboardEvent } from 'react';
import type {
  SessionNavigationModel,
  SessionNavigationRow,
} from '../../application/agentSessions/navigation';
import type {
  SessionFolderTarget,
  SessionPlacement,
} from '../../application/agentSessions/organization';
import { SessionContextMenu } from './SessionContextMenu';
import type { SessionTreeController } from './useSessionTree';
import { pendingRequestLabel } from './sessionAttention';
import { formatSessionDeepLink } from '../../application/agentSessions/deepLinks';
import { browserAgentSessionClipboard } from './sessionClipboard';
const sessionDragType = 'application/x-orchestrator-session';
export function SessionTree({
  model,
  selectedSessionId,
  tree,
  onSelect,
  onNew,
  onMove,
  onPin,
  organizing = true,
}: {
  model: SessionNavigationModel;
  selectedSessionId: string | null;
  tree: SessionTreeController;
  onSelect(id: string): void;
  onNew(target: SessionFolderTarget | null): void;
  onMove(id: string, target: SessionPlacement): Promise<void>;
  onPin(id: string, pinned: boolean): Promise<void>;
  organizing?: boolean;
}) {
  const [menu, setMenu] = useState<{ row: SessionNavigationRow; x: number; y: number } | null>(
    null,
  );
  const [dropId, setDropId] = useState<string | null>(null);
  const menuOrigin = useRef<HTMLElement | null>(null);
  if (menu) menuOrigin.current = tree.refs.current.get(menu.row.id) ?? null;
  const closeMenu = useCallback(() => {
    setMenu(null);
    menuOrigin.current?.focus();
  }, []);
  const dropProps = (id: string, placement: SessionPlacement) => ({
    onDragOver: (event: React.DragEvent) => {
      if (organizing && event.dataTransfer.types.includes(sessionDragType)) {
        event.preventDefault();
        event.stopPropagation();
        event.dataTransfer.dropEffect = 'move';
        setDropId(id);
      }
    },
    onDragLeave: (event: React.DragEvent) => {
      if (!event.currentTarget.contains(event.relatedTarget as Node)) setDropId(null);
    },
    onDrop: (event: React.DragEvent) => {
      const sessionId = event.dataTransfer.getData(sessionDragType);
      if (sessionId && organizing) {
        event.preventDefault();
        event.stopPropagation();
        void onMove(sessionId, placement);
      }
      setDropId(null);
    },
  });
  const keyDown = (event: KeyboardEvent) => {
    if (event.target instanceof HTMLButtonElement) return;
    const index = tree.rows.findIndex((row) => row.id === tree.activeId),
      row = tree.rows[index];
    if (!row) return;
    if (event.key === 'ArrowDown') tree.focus(tree.rows[index + 1]?.id);
    else if (event.key === 'ArrowUp') tree.focus(tree.rows[index - 1]?.id);
    else if (event.key === 'Home') tree.focus(tree.rows[0]?.id);
    else if (event.key === 'End') tree.focus(tree.rows.at(-1)?.id);
    else if (event.key === 'ArrowRight' && row.node.kind === 'folder') {
      if (!tree.expanded.has(row.id)) tree.toggle(row.id);
      else
        tree.focus(tree.rows[index + 1]?.parentId === row.id ? tree.rows[index + 1].id : undefined);
    } else if (event.key === 'ArrowLeft') {
      if (row.node.kind === 'folder' && tree.expanded.has(row.id)) tree.toggle(row.id);
      else tree.focus(row.parentId ?? undefined);
    } else if (
      (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) &&
      row.node.kind === 'session'
    ) {
      const bounds = tree.refs.current.get(row.id)!.getBoundingClientRect();
      setMenu({ row: row.node, x: bounds.left + 20, y: bounds.bottom });
    } else if (event.key === 'Enter' || event.key === ' ') {
      if (row.node.kind === 'folder') tree.toggle(row.id);
      else if (row.node.kind === 'more') tree.showMore(row.node.folderId);
      else onSelect(row.node.summary.id);
    } else return;
    event.preventDefault();
  };
  return (
    <>
      {model.sections.map((section) => (
        <section
          key={section.id}
          className="session-tree-section"
          {...(section.id === 'unfiled' ? dropProps('unfiled', { kind: 'unfiled' }) : {})}
          data-drop-target={dropId === section.id || undefined}
        >
          <div className="session-section-heading">
            <h2>{section.label}</h2>
            {section.id === 'unfiled' && (
              <button
                className="folder-create icon-button"
                aria-label="New session in Unfiled"
                onClick={() => onNew(null)}
              >
                <MessageSquarePlus size={15} />
              </button>
            )}
          </div>
          <div role="tree" aria-label={`${section.label} sessions`} onKeyDown={keyDown}>
            {tree.rows
              .filter((row) => row.sectionId === section.id)
              .map((row, index, sectionRows) => {
                const node = row.node;
                const label =
                  node.kind === 'folder'
                    ? node.label
                    : node.kind === 'more'
                      ? `Show more in ${node.label}`
                      : node.summary.title;
                const groupLabel =
                  node.kind === 'session' &&
                  node.group &&
                  (index === 0 ||
                    sectionRows[index - 1].node.kind !== 'session' ||
                    (sectionRows[index - 1].node as SessionNavigationRow).group !== node.group);
                return (
                  <div key={row.id}>
                    {groupLabel && (
                      <div
                        className="session-owner-group"
                        style={{ '--tree-level': row.level } as CSSProperties}
                      >
                        {node.group === 'owned' ? 'Workflow sessions' : 'Added sessions'}
                      </div>
                    )}
                    <div
                      ref={(element) => {
                        if (element) tree.refs.current.set(row.id, element);
                        else tree.refs.current.delete(row.id);
                      }}
                      role="treeitem"
                      aria-label={label}
                      aria-level={row.level}
                      aria-expanded={node.kind === 'folder' ? tree.expanded.has(row.id) : undefined}
                      aria-selected={
                        node.kind === 'session' ? node.summary.id === selectedSessionId : undefined
                      }
                      tabIndex={tree.activeId === row.id ? 0 : -1}
                      className={`session-tree-row session-tree-row--${node.kind}${node.kind === 'session' && node.summary.id === selectedSessionId ? ' is-selected' : ''}`}
                      data-drop-target={dropId === row.id || undefined}
                      style={{ '--tree-level': row.level } as CSSProperties}
                      onFocus={() => tree.setFocusedId(row.id)}
                      onClick={() => {
                        tree.setFocusedId(row.id);
                        if (node.kind === 'folder') tree.toggle(row.id);
                        else if (node.kind === 'more') tree.showMore(node.folderId);
                        else onSelect(node.summary.id);
                      }}
                      {...(node.kind === 'folder' ? dropProps(row.id, node.placement) : {})}
                      draggable={node.kind === 'session' && organizing}
                      onDragStart={(event) => {
                        if (node.kind === 'session') {
                          event.dataTransfer.setData(sessionDragType, node.summary.id);
                          event.dataTransfer.effectAllowed = 'move';
                        }
                      }}
                      onDragEnd={() => setDropId(null)}
                      onContextMenu={(event) => {
                        if (node.kind === 'session') {
                          event.preventDefault();
                          setMenu({ row: node, x: event.clientX, y: event.clientY });
                        }
                      }}
                    >
                      {node.kind === 'folder' ? (
                        <>
                          {tree.expanded.has(row.id) ? (
                            <ChevronDown size={14} />
                          ) : (
                            <ChevronRight size={14} />
                          )}
                          <Folder size={15} />
                          <span className="session-tree-label">{node.label}</span>
                          <button
                            className="folder-create icon-button"
                            aria-label={`New session in ${node.label}`}
                            onClick={(event) => {
                              event.stopPropagation();
                              onNew(node.createTarget);
                            }}
                          >
                            <MessageSquarePlus size={15} />
                          </button>
                        </>
                      ) : node.kind === 'more' ? (
                        <span className="session-show-more">Show more ({node.remaining})</span>
                      ) : (
                        <>
                          <MessageSquare size={15} />
                          <span className="session-tree-label">
                            {node.summary.title}
                            {node.ownerLabel && (
                              <small title={node.ownerLabel}>{node.ownerLabel}</small>
                            )}
                          </span>
                          {node.pinned && <Pin size={12} aria-label="Pinned" />}
                          {node.summary.pendingRequestCount > 0 ? (
                            <span
                              className="session-attention"
                              title={pendingRequestLabel}
                              aria-label={pendingRequestLabel}
                            />
                          ) : node.summary.hasActiveInvocation ? (
                            <span className="session-active" aria-label="Working" />
                          ) : null}
                        </>
                      )}
                    </div>
                  </div>
                );
              })}
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
            tree.focus(menu.row.id);
          }}
          onPin={() => {
            void onPin(menu.row.summary.id, !menu.row.pinned);
            closeMenu();
            tree.focus(menu.row.id);
          }}
          onCopy={() =>
            browserAgentSessionClipboard.writeText(formatSessionDeepLink(menu.row.summary.id))
          }
        />
      )}
    </>
  );
}
