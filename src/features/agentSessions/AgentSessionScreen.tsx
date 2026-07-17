import { AlertCircle, X } from 'lucide-react';
import type { AgentSessionClient } from '../../application/agentSessions';
import { AgentSessionWorkspace } from './AgentSessionWorkspace';
import { SessionSelector } from './SessionSelector';
import { useAgentSession, useAgentSessionCollection } from './useAgentSessionController';
import './agentSession.css';

export interface AgentSessionScreenProps {
  client: AgentSessionClient;
}
export function StandaloneAgentSessionScreen({ client }: AgentSessionScreenProps) {
  const collection = useAgentSessionCollection(client);
  const session = useAgentSession(client, {
    selectedSessionId: collection.selectedSessionId,
    onSessionCreated: (id) => void collection.selectSession(id).then(() => collection.reload()),
  });
  return (
    <main className="agent-session-screen">
      <SessionSelector
        summaries={collection.summaries}
        selectedSessionId={collection.selectedSessionId}
        loading={collection.loading}
        onSelect={(id) => void collection.selectSession(id)}
        onNew={collection.startNewSession}
        onReload={() => void collection.reload()}
      />
      {collection.error && (
        <section className="agent-session-error" role="alert">
          <AlertCircle size={17} aria-hidden="true" />
          <span>{collection.error}</span>
          <button type="button" onClick={collection.clearError} aria-label="Dismiss error">
            <X size={15} aria-hidden="true" />
          </button>
        </section>
      )}
      <AgentSessionWorkspace controller={session} />
    </main>
  );
}
export const AgentSessionScreen = StandaloneAgentSessionScreen;
