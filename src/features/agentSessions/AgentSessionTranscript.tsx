import type { ProjectedTranscript } from './transcriptProjector';
import { ProcessingDisclosure } from './ProcessingDisclosure';
import { TechnicalDiagnosticDisclosure } from './TechnicalDiagnosticDisclosure';

interface AgentSessionTranscriptProps {
  transcript: ProjectedTranscript | null;
  loading: boolean;
  expandedProcessing: ReadonlySet<string>;
  onToggleProcessing(invocationId: string): void;
}

export function AgentSessionTranscript({
  transcript,
  loading,
  expandedProcessing,
  onToggleProcessing,
}: AgentSessionTranscriptProps) {
  if (loading && !transcript) {
    return (
      <p className="session-empty" role="status">
        Loading session…
      </p>
    );
  }
  if (!transcript || transcript.invocations.length === 0) {
    return (
      <section className="session-empty">
        <h2>Start with a message</h2>
        <p>Your inputs, the agent’s work, and its final responses will stay available here.</p>
      </section>
    );
  }

  return (
    <ol className="agent-transcript" aria-label="Agent Session transcript">
      {transcript.invocations.map((invocation) => (
        <li className="transcript-invocation" key={invocation.id}>
          <article className="transcript-message user-message">
            <header>
              <span>You</span>
              <time dateTime={invocation.createdAt}>{formatTime(invocation.createdAt)}</time>
            </header>
            <p>{invocation.submittedText}</p>
          </article>
          <section
            className="agent-invocation-output"
            aria-label={`Agent response: ${invocation.outcome.label}`}
          >
            <ProcessingDisclosure
              invocationId={invocation.id}
              activity={invocation.processing}
              running={invocation.isActive}
              expanded={expandedProcessing.has(invocation.id)}
              onToggle={() => onToggleProcessing(invocation.id)}
            />
            {invocation.finalResponse && (
              <article className="transcript-message agent-final-message">
                <header>
                  <span>Agent</span>
                  <span className="outcome-label">{invocation.outcome.label}</span>
                </header>
                <p>{invocation.finalResponse}</p>
              </article>
            )}
            {!invocation.finalResponse && invocation.outcome.message && (
              <p className={`invocation-outcome ${invocation.status}`} role="status">
                <strong>{invocation.outcome.label}.</strong> {invocation.outcome.message}
              </p>
            )}
            {!invocation.finalResponse && invocation.status === 'completed' && (
              <p className="invocation-outcome completed" role="status">
                <strong>Completed without a final response.</strong>
              </p>
            )}
            <TechnicalDiagnosticDisclosure
              activity={invocation.technical}
              diagnostics={invocation.diagnostics}
            />
          </section>
        </li>
      ))}
    </ol>
  );
}

function formatTime(value: string): string {
  return new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(
    new Date(value),
  );
}
