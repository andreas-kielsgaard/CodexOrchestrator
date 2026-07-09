import { GitBranch, Inbox, LoaderCircle, RefreshCw, Terminal, X } from 'lucide-react';
import type { BackendMaintenanceViewModel } from '../viewModels/backendMaintenanceViewModel';

export type AppShellView = 'orchestrations' | 'tasks' | 'agent-session';

export interface StartupScreenProps {
  loading: boolean;
  error: string | null;
  onRetry(): void;
}

export function StartupScreen({ loading, error, onRetry }: StartupScreenProps) {
  return (
    <main className="startup-screen" aria-busy={loading}>
      <section className="startup-panel" aria-label="App startup">
        <div className="brand-mark">CO</div>
        <div>
          <p className="eyebrow">Codex Orchestrator</p>
          <h1>{error ? 'Backend unavailable' : 'Starting local backend'}</h1>
          <p>{error ?? 'Waiting for the Tauri command layer to answer.'}</p>
        </div>
        {loading ? (
          <LoaderCircle className="spin" size={28} aria-hidden="true" />
        ) : (
          <button className="primary-action" type="button" onClick={onRetry}>
            <RefreshCw size={17} aria-hidden="true" />
            Retry
          </button>
        )}
      </section>
    </main>
  );
}

export interface AppSidebarProps {
  activeView: AppShellView;
  backendMaintenance?: BackendMaintenanceViewModel;
  onViewChange(view: AppShellView): void;
  onCheckBackend?(): void;
}

export function AppSidebar({
  activeView,
  backendMaintenance,
  onViewChange,
  onCheckBackend,
}: AppSidebarProps) {
  return (
    <aside className="sidebar" aria-label="Primary navigation">
      <div>
        <div className="brand-mark">CO</div>
        <nav>
          <button
            className={`nav-item${activeView === 'orchestrations' ? ' active' : ''}`}
            type="button"
            onClick={() => onViewChange('orchestrations')}
          >
            <GitBranch size={18} aria-hidden="true" />
            Orchestrations
          </button>
          <button
            className={`nav-item${activeView === 'tasks' ? ' active' : ''}`}
            type="button"
            onClick={() => onViewChange('tasks')}
          >
            <Inbox size={18} aria-hidden="true" />
            Open Tasks
          </button>
          <button
            className={`nav-item${activeView === 'agent-session' ? ' active' : ''}`}
            type="button"
            onClick={() => onViewChange('agent-session')}
          >
            <Terminal size={18} aria-hidden="true" />
            Agent Session View
          </button>
        </nav>
      </div>

      {backendMaintenance && (
        <div className={`backend-maintenance ${backendMaintenance.status}`} title={backendMaintenance.title}>
          <span>{backendMaintenance.label}</span>
          <button
            className="icon-button"
            type="button"
            onClick={onCheckBackend}
            disabled={backendMaintenance.disabled}
            title={backendMaintenance.message}
            aria-label="Check and reopen Rust backend"
          >
            <RefreshCw
              className={backendMaintenance.busy ? 'spin' : undefined}
              size={16}
              aria-hidden="true"
            />
          </button>
        </div>
      )}
    </aside>
  );
}

export interface RuntimeStaleNoticeProps {
  message: string;
  onRefresh(): void;
  onDismiss(): void;
}

export function RuntimeStaleNotice({ message, onRefresh, onDismiss }: RuntimeStaleNoticeProps) {
  return (
    <section className="notice stale" role="status" aria-label="App notifications">
      <RefreshCw size={18} aria-hidden="true" />
      <span>{message}</span>
      <button className="text-button" type="button" onClick={onRefresh}>
        Refresh
      </button>
      <button
        className="icon-button"
        type="button"
        onClick={onDismiss}
        title="Dismiss stale notice"
        aria-label="Dismiss stale notice"
      >
        <X size={16} aria-hidden="true" />
      </button>
    </section>
  );
}
