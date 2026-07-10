import { AlertCircle, X } from 'lucide-react';
import type { AgentSessionClient } from '../../application/agentSessions';
import { AgentSessionComposer } from './AgentSessionComposer';
import { AgentSessionTranscript } from './AgentSessionTranscript';
import { SessionSelector } from './SessionSelector';
import { useAgentSessionController } from './useAgentSessionController';
import { useTranscriptFollow } from './useTranscriptFollow';
import './agentSession.css';

export interface AgentSessionScreenProps {
  client: AgentSessionClient;
}

export function AgentSessionScreen({ client }: AgentSessionScreenProps) {
  const controller = useAgentSessionController(client);
  const active = Boolean(controller.transcript?.activeInvocationId);
  const title = controller.details?.session.title ?? 'New Agent Session';
  const transcriptRevision =
    controller.transcript?.invocations
      .map(
        (invocation) =>
          `${invocation.id}:${invocation.status}:${invocation.processing.length}:${invocation.technical.length}:${invocation.finalResponse?.length ?? 0}`,
      )
      .join('|') ?? 'empty';
  const transcriptFollow = useTranscriptFollow(controller.selectedSessionId, transcriptRevision);

  return (
    <main className="agent-session-screen">
      <SessionSelector
        summaries={controller.summaries}
        selectedSessionId={controller.selectedSessionId}
        loading={controller.loading}
        onSelect={(sessionId) => void controller.selectSession(sessionId)}
        onNew={controller.startNewSession}
        onReload={() => void controller.reload()}
      />
      <section className="agent-session-workspace" aria-label={title}>
        <header className="agent-session-header">
          <div>
            <p className="eyebrow">Agent Session</p>
            <h2>{title}</h2>
            {controller.details?.session.workingDirectory && (
              <p
                className="session-working-directory"
                title={controller.details.session.workingDirectory}
              >
                {controller.details.session.workingDirectory}
              </p>
            )}
          </div>
          {active && (
            <span className="working-status" role="status">
              Working
            </span>
          )}
        </header>
        {controller.error && (
          <section className="agent-session-error" role="alert">
            <AlertCircle size={17} aria-hidden="true" />
            <span>{controller.error}</span>
            <button type="button" onClick={controller.clearError} aria-label="Dismiss error">
              <X size={15} aria-hidden="true" />
            </button>
          </section>
        )}
        <div
          className="agent-session-scroll-region"
          ref={transcriptFollow.containerRef}
          onScroll={transcriptFollow.handleScroll}
        >
          <AgentSessionTranscript
            transcript={controller.transcript}
            loading={controller.loading}
            expandedProcessing={controller.expandedProcessing}
            onToggleProcessing={controller.toggleProcessing}
          />
        </div>
        <AgentSessionComposer
          draft={controller.draft}
          workingDirectory={controller.workingDirectory}
          isNewSession={!controller.selectedSessionId}
          sending={controller.sending}
          active={active}
          canceling={controller.canceling}
          onDraftChange={controller.setDraft}
          onWorkingDirectoryChange={controller.setWorkingDirectory}
          onSend={() => {
            transcriptFollow.requestFollow();
            void controller.send();
          }}
          onCancel={() => void controller.cancel()}
        />
      </section>
    </main>
  );
}
