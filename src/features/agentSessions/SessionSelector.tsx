import { SquarePen, RefreshCw } from 'lucide-react';
import type { ComponentProps } from 'react';
import { SessionNavigation } from './SessionNavigation';
import './sessionNavigation.css';
export function SessionSelector({
  loading,
  onReload,
  onImport,
  ...tree
}: ComponentProps<typeof SessionNavigation> & {
  loading: boolean;
  onReload(): void;
  onImport?(): void;
}) {
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
