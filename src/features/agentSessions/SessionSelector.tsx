import { MessageSquarePlus, RefreshCw } from 'lucide-react';
import type { AgentIdentity, AgentSessionSummaryDto } from '../../application/agentSessions';
import { AgentIdentityBadge } from '../../components/AgentIdentityBadge';

interface SessionSelectorProps {
  summaries: AgentSessionSummaryDto[];
  selectedSessionId: string | null;
  loading: boolean;
  onSelect(sessionId: string): void;
  onNew(): void;
  onReload(): void;
  agentIdentityForSession?: (sessionId: string) => AgentIdentity | undefined;
}

export function SessionSelector({
  summaries,
  selectedSessionId,
  loading,
  onSelect,
  onNew,
  onReload,
  agentIdentityForSession,
}: SessionSelectorProps) {
  return (
    <aside className="agent-session-selector" aria-label="Agent Sessions">
      <header>
        <div>
          <p className="eyebrow">Workspace</p>
          <h1>Agent Sessions</h1>
        </div>
        <button className="icon-button" type="button" onClick={onNew} aria-label="New session">
          <MessageSquarePlus size={17} aria-hidden="true" />
        </button>
      </header>
      <button className="session-new-button" type="button" onClick={onNew}>
        New session
      </button>
      <nav aria-label="Session list">
        {summaries.length === 0 ? (
          <p className="session-list-empty">No saved sessions yet.</p>
        ) : (
          summaries.map((summary) => {
            const identity = agentIdentityForSession?.(summary.id);
            return (
              <button
                className={`session-list-item${summary.id === selectedSessionId ? ' active' : ''}`}
                type="button"
                key={summary.id}
                onClick={() => onSelect(summary.id)}
                aria-current={summary.id === selectedSessionId ? 'page' : undefined}
              >
                <span className="session-list-item__identity">
                  {identity && <AgentIdentityBadge identity={identity} compact />}
                  <span className="session-list-item__title">
                    {identity ? `${identity.name}: ${identityRoleLabel(identity)}` : summary.title}
                  </span>
                </span>
                <small>
                  {summary.hasActiveInvocation ? 'Working' : formatDate(summary.updatedAt)}
                </small>
              </button>
            );
          })
        )}
      </nav>
      <button
        className="session-refresh-button"
        type="button"
        onClick={onReload}
        disabled={loading}
      >
        <RefreshCw className={loading ? 'spin' : ''} size={15} aria-hidden="true" />
        Refresh
      </button>
    </aside>
  );
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(
    new Date(value),
  );
}

function identityRoleLabel(identity: AgentIdentity): string {
  return identity.harnessRole
    .split('_')
    .filter(Boolean)
    .map((part) => `${part.charAt(0).toLocaleUpperCase()}${part.slice(1)}`)
    .join(' ');
}
