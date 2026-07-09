import { RefreshCw } from 'lucide-react';

export interface OpenTasksHeaderProps {
  totalOpenTasks: number;
  projectCount: number;
  worktreeCount: number;
  busy: boolean;
  onReload(): void;
}

export function OpenTasksHeader({
  totalOpenTasks,
  projectCount,
  worktreeCount,
  busy,
  onReload,
}: OpenTasksHeaderProps) {
  return (
    <header className="topbar">
      <div>
        <p className="eyebrow">Local-first control plane</p>
        <h1>Open Tasks</h1>
      </div>
      <div className="status-strip" aria-label="Dashboard totals">
        <span>{totalOpenTasks} open</span>
        <span>{projectCount} projects</span>
        <span>{worktreeCount} worktrees</span>
        <button
          className="icon-button"
          type="button"
          onClick={onReload}
          disabled={busy}
          title="Reload dashboard"
          aria-label="Reload dashboard"
        >
          <RefreshCw size={17} aria-hidden="true" />
        </button>
      </div>
    </header>
  );
}
