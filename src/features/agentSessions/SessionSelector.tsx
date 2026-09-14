import { MessageSquarePlus, RefreshCw } from 'lucide-react';
import type { ComponentProps } from 'react';
import { SessionTree } from './SessionTree';
import './sessionNavigation.css';
export function SessionSelector({
  loading,
  onReload,
  ...tree
}: ComponentProps<typeof SessionTree> & { loading: boolean; onReload(): void }) {
  return (
    <nav className="agent-session-selector" aria-label="Session list">
      <header>
        <div>
          <p className="eyebrow">Workspace</p>
          <h1>Agent Sessions</h1>
        </div>
        <button className="icon-button" aria-label="New session" onClick={() => tree.onNew(null)}>
          <MessageSquarePlus size={17} />
        </button>
      </header>
      <div className="session-tree" aria-busy={loading}>
        <SessionTree {...tree} />
      </div>
      <button className="session-refresh-button" onClick={onReload} disabled={loading}>
        <RefreshCw size={15} className={loading ? 'spin' : undefined} />
        Refresh
      </button>
    </nav>
  );
}
