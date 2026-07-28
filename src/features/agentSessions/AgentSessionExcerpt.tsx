import type { MouseEvent } from 'react';
import type { AgentIdentityDto } from '../../application/agentSessions';
import { AgentSessionTranscript } from './AgentSessionTranscript';
import {
  selectTranscriptRange,
  type ProjectedTranscript,
  type TranscriptAnchorRange,
} from './transcriptProjector';

export interface AgentSessionExcerptProps {
  readonly transcript: ProjectedTranscript;
  readonly agentIdentity?: AgentIdentityDto | null;
  readonly range: TranscriptAnchorRange;
  readonly actionLabel: string;
  readonly expanded?: boolean;
  readonly controls?: string;
  readonly onActivate: () => void;
}

/** Read-only anchored Agent Session projection for compact or contextual surfaces. */
export function AgentSessionExcerpt({
  transcript,
  agentIdentity,
  range,
  actionLabel,
  expanded,
  controls,
  onActivate,
}: AgentSessionExcerptProps) {
  const handleSurfaceClick = (event: MouseEvent<HTMLElement>) => {
    const target = event.target as HTMLElement;
    if (target.closest('a, button, summary, input, textarea, select')) return;
    onActivate();
  };

  return (
    <section className="agent-session-excerpt" onClick={handleSurfaceClick}>
      <div className="agent-session-excerpt__transcript">
        <AgentSessionTranscript
          transcript={transcript}
          agentIdentity={agentIdentity}
          content={selectTranscriptRange(transcript, range)}
          loading={false}
          expandedProcessing={new Set()}
          onToggleProcessing={() => undefined}
        />
      </div>
      <button
        className="agent-session-excerpt__action"
        type="button"
        aria-expanded={expanded}
        aria-controls={controls}
        onClick={onActivate}
      >
        {actionLabel}
      </button>
    </section>
  );
}
