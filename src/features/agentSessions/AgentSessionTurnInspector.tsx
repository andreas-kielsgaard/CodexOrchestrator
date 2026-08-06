import { useState } from 'react';
import type { AgentIdentity } from '../../application/agentSessions';
import { AgentSessionTranscript } from './AgentSessionTranscript';
import {
  projectedTranscriptContent,
  selectTranscriptRange,
  selectTranscriptInvocation,
  type ProjectedTranscript,
  type TranscriptAnchorRange,
} from './transcriptProjector';

export interface AgentSessionTurnInspectorProps {
  readonly sessionId: string;
  readonly invocationId: string;
  readonly transcript: ProjectedTranscript | null;
  readonly loading?: boolean;
  readonly error?: string | null;
  readonly agentIdentity?: AgentIdentity;
  readonly ariaLabel?: string;
  /** Optional exact passage inside the selected durable turn. */
  readonly transcriptRange?: TranscriptAnchorRange;
  /** Optional explicit prior invocation supplied by the owning product context. */
  readonly precedingInput?: Readonly<{
    readonly invocationId: string;
    readonly text: string;
    readonly provenance: 'user' | 'application';
  }>;
}

/** Read-only presentation of exactly one durable Session/invocation turn. */
export function AgentSessionTurnInspector({
  sessionId,
  invocationId,
  transcript,
  loading = false,
  error = null,
  agentIdentity,
  ariaLabel = 'Agent Session turn inspector',
  transcriptRange,
  precedingInput,
}: AgentSessionTurnInspectorProps) {
  const [expandedProcessing, setExpandedProcessing] = useState<ReadonlySet<string>>(new Set());
  const invocation = selectTranscriptInvocation(transcript, sessionId, invocationId);

  if (loading && !transcript && !error) {
    return (
      <section className="agent-session-turn-inspector" aria-label={ariaLabel}>
        <p className="session-empty" role="status">
          Loading selected Agent Session turn…
        </p>
      </section>
    );
  }

  const content = transcript
    ? transcriptRange
      ? selectTranscriptRange(transcript, transcriptRange)
      : projectedTranscriptContent(transcript).filter(
          (item) => item.anchor.invocationId === invocationId,
        )
    : [];

  if (!invocation || (transcriptRange && content.length === 0)) {
    return (
      <section className="agent-session-turn-inspector" aria-label={ariaLabel} role="alert">
        <h2>Agent Session turn unavailable</h2>
        <p>The selected Session and invocation are not available in the current recorded view.</p>
        {error && <p>{error}</p>}
      </section>
    );
  }

  const startLabel = formatDateTime(invocation.startedAt);
  const durationLabel = formatDuration(invocation.startedAt, invocation.completedAt);

  return (
    <section
      className="agent-session-turn-inspector"
      aria-label={ariaLabel}
      data-session-id={sessionId}
      data-invocation-id={invocationId}
      tabIndex={-1}
    >
      <header className="agent-session-turn-inspector__heading">
        <div>
          <span>Agent turn</span>
          <strong>Complete recorded turn</strong>
        </div>
        {(startLabel || durationLabel) && (
          <dl className="agent-session-turn-inspector__timing">
            {startLabel && (
              <div>
                <dt>Started</dt>
                <dd>
                  <time dateTime={invocation.startedAt!}>{startLabel}</time>
                </dd>
              </div>
            )}
            {durationLabel && (
              <div>
                <dt>Duration</dt>
                <dd>{durationLabel}</dd>
              </div>
            )}
          </dl>
        )}
      </header>
      {precedingInput ? (
        <article className="agent-session-turn-inspector__preceding-input">
          <header>
            {precedingInput.provenance === 'application' ? 'Application input' : 'Previous input'}
          </header>
          <p>{precedingInput.text}</p>
        </article>
      ) : null}
      <AgentSessionTranscript
        transcript={transcript}
        content={content}
        loading={false}
        expandedProcessing={expandedProcessing}
        onToggleProcessing={(nextInvocationId) => {
          setExpandedProcessing((current) => {
            const next = new Set(current);
            if (next.has(nextInvocationId)) next.delete(nextInvocationId);
            else next.add(nextInvocationId);
            return next;
          });
        }}
        agentIdentity={agentIdentity}
        safeActivityDetails
        showTechnicalDetails={false}
        processingHeading="Recorded steps"
      />
    </section>
  );
}

function formatDateTime(value: string | null): string | null {
  if (!value || Number.isNaN(Date.parse(value))) return null;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(value));
}

function formatDuration(startedAt: string | null, completedAt: string | null): string | null {
  if (!startedAt || !completedAt) return null;
  const elapsedMs = Date.parse(completedAt) - Date.parse(startedAt);
  if (!Number.isFinite(elapsedMs) || elapsedMs < 0) return null;
  if (elapsedMs < 1000) return '<1s';
  const totalSeconds = Math.round(elapsedMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return minutes ? `${minutes}m ${seconds}s` : `${seconds}s`;
}
