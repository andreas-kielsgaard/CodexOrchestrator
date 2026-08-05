import {
  projectedTranscriptContent,
  type ProjectedTranscript,
  type ProjectedTranscriptContent,
} from './transcriptProjector';
import { AgentMarkdown } from './AgentMarkdown';
import { ProcessingDisclosure } from './ProcessingDisclosure';
import { TechnicalDiagnosticDisclosure } from './TechnicalDiagnosticDisclosure';
import type { AgentIdentity } from '../../application/agentSessions';
import { AgentIdentityBadge } from '../../components/AgentIdentityBadge';

interface AgentSessionTranscriptProps {
  transcript: ProjectedTranscript | null;
  content?: readonly ProjectedTranscriptContent[];
  loading: boolean;
  expandedProcessing: ReadonlySet<string>;
  onToggleProcessing(invocationId: string): void;
  emptyState?: Readonly<{ heading: string; guidance: string }>;
  agentIdentity?: AgentIdentity;
  safeActivityDetails?: boolean;
  showTechnicalDetails?: boolean;
  processingHeading?: string;
}

export function AgentSessionTranscript({
  transcript,
  content,
  loading,
  expandedProcessing,
  onToggleProcessing,
  emptyState,
  agentIdentity,
  safeActivityDetails = false,
  showTechnicalDetails = true,
  processingHeading,
}: AgentSessionTranscriptProps) {
  if (loading && !transcript) {
    return (
      <p className="session-empty" role="status">
        Loading session…
      </p>
    );
  }
  const visibleInvocations = transcript
    ? projectVisibleInvocations(transcript, content ?? projectedTranscriptContent(transcript))
    : [];
  if (transcript && content && !visibleInvocations.length) return null;
  if (!transcript || !visibleInvocations.length) {
    return (
      <section className={`session-empty${emptyState ? ' session-empty--custom' : ''}`}>
        <h2>{emptyState?.heading ?? 'Start with a message'}</h2>
        {emptyState && <p className="session-empty__guidance">{emptyState.guidance}</p>}
        <p>Your inputs, the agent’s work, and its final responses will stay available here.</p>
      </section>
    );
  }

  return (
    <ol className="agent-transcript" aria-label="Agent Session transcript">
      {visibleInvocations.map((invocation) => (
        <li
          className="transcript-invocation"
          key={invocation.id}
          data-invocation-id={invocation.id}
          tabIndex={-1}
        >
          {invocation.showInput && (
            <article className="transcript-message user-message">
              <header>
                <span>
                  {invocation.inputProvenance === 'application'
                    ? 'Plan Builder / Application'
                    : 'You'}
                </span>
              </header>
              <p>{invocation.submittedText}</p>
            </article>
          )}
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
              safeOnly={safeActivityDetails}
              heading={processingHeading}
            />
            {invocation.finalResponse && (
              <article className="transcript-message agent-final-message">
                <header>
                  <span className="transcript-message__agent">
                    {agentIdentity && <AgentIdentityBadge identity={agentIdentity} compact />}
                    <span>{agentIdentity?.name ?? 'Agent'}</span>
                  </span>
                  <span className="outcome-label">{invocation.outcome.label}</span>
                </header>
                <AgentMarkdown>{invocation.finalResponse.text}</AgentMarkdown>
              </article>
            )}
            {invocation.showOutcome && !invocation.finalResponse && invocation.outcome.message && (
              <p className={`invocation-outcome ${invocation.status}`} role="status">
                <strong>{invocation.outcome.label}.</strong> {invocation.outcome.message}
              </p>
            )}
            {invocation.showOutcome &&
              !invocation.finalResponse &&
              invocation.status === 'completed' && (
                <p className="invocation-outcome completed" role="status">
                  <strong>Completed without a final response.</strong>
                </p>
              )}
            {showTechnicalDetails && (
              <TechnicalDiagnosticDisclosure
                activity={invocation.technical}
                diagnostics={invocation.diagnostics}
                running={invocation.isActive}
                safeOnly={safeActivityDetails}
              />
            )}
          </section>
        </li>
      ))}
    </ol>
  );
}

type VisibleInvocation = ProjectedTranscript['invocations'][number] & {
  showInput: boolean;
  showOutcome: boolean;
};

function projectVisibleInvocations(
  transcript: ProjectedTranscript | null,
  content: readonly ProjectedTranscriptContent[],
): VisibleInvocation[] {
  if (!transcript) return [];
  const visible = new Map<string, ProjectedTranscriptContent[]>();
  for (const item of content) {
    const items = visible.get(item.anchor.invocationId) ?? [];
    items.push(item);
    visible.set(item.anchor.invocationId, items);
  }
  return transcript.invocations.flatMap((invocation) => {
    const items = visible.get(invocation.id);
    if (!items) return [];
    const activityIds = new Set(
      items.filter((item) => item.kind === 'activity').map((item) => item.activity.id),
    );
    const final = items.find((item) => item.kind === 'final_response');
    return [
      {
        ...invocation,
        processing: invocation.processing.filter((item) => activityIds.has(item.id)),
        technical: invocation.technical.filter((item) => activityIds.has(item.id)),
        finalResponse: final?.kind === 'final_response' ? final.response : null,
        showInput: items.some((item) => item.kind === 'submitted_input'),
        showOutcome: items.some((item) => item.kind === 'outcome'),
      },
    ];
  });
}
