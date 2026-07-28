import { AlertCircle, X } from 'lucide-react';
import type { AgentIdentityDto } from '../../application/agentSessions';
import { AgentSessionComposer } from './AgentSessionComposer';
import { AgentSessionTranscript } from './AgentSessionTranscript';
import {
  projectedTranscriptContent,
  selectTranscriptRange,
  type ProjectedTranscript,
  type TranscriptAnchorRange,
} from './transcriptProjector';
import { useTranscriptFollow } from './useTranscriptFollow';

export interface ConversationViewportSegment {
  /** Caller-assigned causal position. Segments are rendered in this exact array order. */
  id: string;
  transcript: ProjectedTranscript;
  range?: TranscriptAnchorRange;
}

export interface ConversationViewportComposerTarget {
  sessionId: string | null;
  draft: string;
  workingDirectory: string;
  sending: boolean;
  active: boolean;
  canceling: boolean;
  setDraft(value: string): void;
  setWorkingDirectory(value: string): void;
  send(): Promise<void>;
  cancel(): Promise<void>;
}

export interface ConversationEmptyStatePresentation {
  readonly heading: string;
  readonly guidance: string;
}

export interface ConversationComposerPresentation {
  readonly messageLabel?: string;
  readonly messagePlaceholder?: string;
  readonly showWorkingDirectory?: boolean;
  readonly keyboardHint?: 'tooltip' | 'hidden';
}

export interface ConversationViewportProps {
  segments: readonly ConversationViewportSegment[];
  agentIdentity?: AgentIdentityDto | null;
  loading: boolean;
  expandedProcessing: ReadonlySet<string>;
  onToggleProcessing(invocationId: string): void;
  /** Omit for a read-only view. A composite must provide this target explicitly. */
  composerTarget?: ConversationViewportComposerTarget;
  error?: string | null;
  onClearError?(): void;
  ariaLabel?: string;
  emptyState?: ConversationEmptyStatePresentation;
  composerPresentation?: ConversationComposerPresentation;
}

export function ConversationViewport({
  segments,
  agentIdentity,
  loading,
  expandedProcessing,
  onToggleProcessing,
  composerTarget,
  error,
  onClearError,
  ariaLabel = 'Conversation',
  emptyState,
  composerPresentation,
}: ConversationViewportProps) {
  const revision = segments
    .map(
      (segment) =>
        `${segment.id}:${segment.transcript.invocations.map((item) => `${item.id}:${item.status}:${item.processing.length}:${item.technical.length}:${item.finalResponse?.eventId ?? ''}`).join(',')}`,
    )
    .join('|');
  const follow = useTranscriptFollow(segments.map((segment) => segment.id).join('|'), revision);

  return (
    <div className="agent-session-conversation">
      {error && (
        <section className="agent-session-error" role="alert">
          <AlertCircle size={17} aria-hidden="true" />
          <span>{error}</span>
          {onClearError && (
            <button type="button" onClick={onClearError} aria-label="Dismiss error">
              <X size={15} aria-hidden="true" />
            </button>
          )}
        </section>
      )}
      <div
        className="agent-session-scroll-region"
        aria-label={ariaLabel}
        ref={follow.containerRef}
        onScroll={follow.handleScroll}
      >
        {segments.map((segment) => (
          <AgentSessionTranscript
            key={segment.id}
            transcript={segment.transcript}
            agentIdentity={agentIdentity}
            content={
              segment.range
                ? selectTranscriptRange(segment.transcript, segment.range)
                : projectedTranscriptContent(segment.transcript)
            }
            loading={loading}
            expandedProcessing={expandedProcessing}
            onToggleProcessing={onToggleProcessing}
            emptyState={emptyState}
          />
        ))}
        {!segments.length && (
          <AgentSessionTranscript
            transcript={null}
            agentIdentity={agentIdentity}
            loading={loading}
            emptyState={emptyState}
            expandedProcessing={expandedProcessing}
            onToggleProcessing={onToggleProcessing}
          />
        )}
      </div>
      {composerTarget && (
        <AgentSessionComposer
          draft={composerTarget.draft}
          workingDirectory={composerTarget.workingDirectory}
          isNewSession={!composerTarget.sessionId}
          sending={composerTarget.sending}
          active={composerTarget.active}
          canceling={composerTarget.canceling}
          messageLabel={composerPresentation?.messageLabel}
          messagePlaceholder={composerPresentation?.messagePlaceholder}
          showWorkingDirectory={composerPresentation?.showWorkingDirectory ?? false}
          keyboardHint={composerPresentation?.keyboardHint ?? 'tooltip'}
          onDraftChange={composerTarget.setDraft}
          onWorkingDirectoryChange={composerTarget.setWorkingDirectory}
          onSend={() => {
            follow.requestFollow();
            void composerTarget.send();
          }}
          onCancel={() => void composerTarget.cancel()}
        />
      )}
    </div>
  );
}
