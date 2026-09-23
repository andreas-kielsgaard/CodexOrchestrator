import { SquarePen, RefreshCw } from 'lucide-react';
import { memo, type ComponentProps } from 'react';
import { SessionNavigation } from './SessionNavigation';
import './sessionNavigation.css';
type SessionSelectorProps = ComponentProps<typeof SessionNavigation> & {
  loading: boolean;
  onReload(): void;
  onImport?(): void;
};

function SessionSelectorView({ loading, onReload, onImport, ...tree }: SessionSelectorProps) {
  return (
    <nav className="agent-session-selector" aria-label="Session list">
      <header>
        <div>
          <h1>Agent Sessions</h1>
        </div>
        <button
          className="session-icon-button"
          aria-label="New session"
          onClick={() => tree.onNew(null)}
        >
          <SquarePen size={17} />
        </button>
      </header>
      {onImport && (
        <button className="session-refresh-button" onClick={onImport}>
          Import from Codex
        </button>
      )}
      <div className="session-tree" aria-busy={loading}>
        <SessionNavigation {...tree} />
      </div>
      <button className="session-refresh-button" onClick={onReload} disabled={loading}>
        <RefreshCw size={15} className={loading ? 'spin' : undefined} />
        Refresh
      </button>
    </nav>
  );
}

/** Conversation updates must not rerender the independently-owned navigation tree. */
export const SessionSelector = memo(
  SessionSelectorView,
  (previous, next) =>
    previous.model === next.model &&
    previous.selectedSessionId === next.selectedSessionId &&
    previous.tree.view === next.tree.view &&
    previous.tree.activeId === next.tree.activeId &&
    previous.loading === next.loading &&
    previous.organizing === next.organizing &&
    previous.onImport === next.onImport &&
    previous.onOpenWorkflow === next.onOpenWorkflow,
);
