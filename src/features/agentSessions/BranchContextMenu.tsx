import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { SessionNavigationFolder } from '../../application/agentSessions/navigation';
import type {
  NavigationWorktree,
  SessionGitAction,
  SessionNavigationClient,
  SessionWorktreeInspection,
} from '../../application/agentSessions/organization';
import { gitActionLabels } from '../../application/agentSessions/gitActions';

export function BranchContextMenu({
  node,
  position,
  client,
  onClose,
  onAction,
}: {
  node: SessionNavigationFolder;
  position: { x: number; y: number };
  client: SessionNavigationClient;
  onClose(): void;
  onAction(action: SessionGitAction): void;
}) {
  const menu = useRef<HTMLDivElement>(null),
    card = useRef<HTMLDivElement>(null);
  const [submenu, setSubmenu] = useState<'git' | 'worktrees' | null>(null);
  const [preview, setPreview] = useState<{
    worktree: NavigationWorktree;
    x: number;
    y: number;
    pinned: boolean;
  } | null>(null);
  const [facts, setFacts] = useState<SessionWorktreeInspection | null>(null);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const cancelClose = () => {
    if (timer.current) clearTimeout(timer.current);
  };
  const scheduleClose = () => {
    cancelClose();
    timer.current = setTimeout(() => setPreview((p) => (p?.pinned ? p : null)), 200);
  };
  useEffect(() => {
    const outside = (e: PointerEvent) => {
      if (!menu.current?.contains(e.target as Node) && !card.current?.contains(e.target as Node))
        onClose();
    };
    document.addEventListener('pointerdown', outside);
    const escape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', escape);
    menu.current?.querySelector('button')?.focus();
    return () => {
      document.removeEventListener('pointerdown', outside);
      document.removeEventListener('keydown', escape);
    };
  }, [onClose]);
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );
  const worktree = preview?.worktree;
  useEffect(() => {
    setFacts(null);
    setError(null);
    if (!worktree || !node.branch || !client.inspectWorktree) return;
    let current = true;
    void client.inspectWorktree(node.branch.repositoryId, node.branch.branchRef, worktree).then(
      (result) => {
        if (current) setFacts(result);
      },
      (cause) => {
        if (current) setError(String(cause));
      },
    );
    return () => {
      current = false;
    };
  }, [worktree, client, node.branch]);
  const show = (worktree: NavigationWorktree, element: HTMLElement, pinned = false) => {
    cancelClose();
    const rect = element.getBoundingClientRect();
    setPreview((p) =>
      p?.pinned && !pinned ? p : { worktree, pinned, x: rect.right + 8, y: rect.top },
    );
  };
  const worktrees = node.branch?.worktrees ?? [];
  return createPortal(
    <>
      {!preview?.pinned && (
        <div
          ref={menu}
          role="menu"
          aria-label={`Branch actions for ${node.label}`}
          className="session-context-menu session-branch-menu"
          style={{
            left: Math.max(8, Math.min(position.x, window.innerWidth - 285)),
            top: Math.max(8, Math.min(position.y, window.innerHeight - 320)),
          }}
        >
          <button
            role="menuitem"
            disabled={!client.previewGitAction}
            onClick={() => onAction('create')}
          >
            Create branch…
          </button>
          {node.branch && (
            <>
              <button
                role="menuitem"
                aria-haspopup="menu"
                aria-expanded={submenu === 'git'}
                onMouseEnter={() => setSubmenu('git')}
                onClick={() => setSubmenu(submenu === 'git' ? null : 'git')}
              >
                Git commands ›
              </button>
              {submenu === 'git' && (
                <div role="menu" aria-label="Git commands" className="session-branch-submenu">
                  {(['rename', 'delete', 'fetch', 'merge'] as const).map((action) => (
                    <button
                      key={action}
                      role="menuitem"
                      disabled={!client.previewGitAction}
                      onClick={() => onAction(action)}
                    >
                      {gitActionLabels[action]}…
                    </button>
                  ))}
                </div>
              )}
              <button
                role="menuitem"
                aria-haspopup="menu"
                aria-expanded={submenu === 'worktrees'}
                onMouseEnter={() => setSubmenu('worktrees')}
                onClick={() => setSubmenu(submenu === 'worktrees' ? null : 'worktrees')}
              >
                Worktrees ›
              </button>
              {submenu === 'worktrees' && (
                <div role="menu" aria-label="Worktrees" className="session-branch-submenu">
                  {worktrees.map((w) => (
                    <button
                      key={`${w.deviceId}:${w.path}`}
                      role="menuitem"
                      onMouseEnter={(e) => show(w, e.currentTarget)}
                      onMouseLeave={scheduleClose}
                      onFocus={(e) => show(w, e.currentTarget)}
                      onClick={(e) => show(w, e.currentTarget, true)}
                    >
                      {w.deviceName} · {w.path.split(/[\\/]/).filter(Boolean).at(-1)}
                    </button>
                  ))}
                  {!worktrees.length && <p>No worktree instances available.</p>}
                </div>
              )}
            </>
          )}
        </div>
      )}
      {preview && (
        <div
          ref={card}
          role="dialog"
          aria-label="Worktree information"
          tabIndex={-1}
          className="session-worktree-card"
          onMouseEnter={cancelClose}
          onMouseLeave={scheduleClose}
          style={{
            left: Math.max(8, Math.min(preview.x, window.innerWidth - 380)),
            top: Math.max(8, Math.min(preview.y, window.innerHeight - 440)),
          }}
        >
          <div className="session-worktree-card-heading">
            <h3>{node.label}</h3>
            <button aria-label="Close worktree information" onClick={onClose}>
              ×
            </button>
          </div>
          <dl>
            <dt>Device</dt>
            <dd>{preview.worktree.deviceName}</dd>
            <dt>Worktree</dt>
            <dd>{preview.worktree.path}</dd>
            <dt>Created</dt>
            <dd>
              {preview.worktree.createdAt
                ? new Date(preview.worktree.createdAt).toLocaleString()
                : 'Unavailable'}
            </dd>
            <dt>HEAD</dt>
            <dd>{facts?.head ?? preview.worktree.head ?? 'Checking…'}</dd>
            <dt>Uncommitted work</dt>
            <dd>
              {facts
                ? `${facts.staged.files} staged · ${facts.unstaged.files} unstaged · ${facts.untracked.files} untracked files`
                : preview.worktree.dirty == null
                  ? 'Checking…'
                  : preview.worktree.dirty
                    ? 'Present · checking details…'
                    : 'Clean · checking details…'}
            </dd>
          </dl>
          {error && <p role="alert">Could not refresh: {error}</p>}
          {!preview.pinned && (
            <button onClick={() => setPreview({ ...preview, pinned: true })}>Keep open</button>
          )}
        </div>
      )}
    </>,
    document.body,
  );
}
