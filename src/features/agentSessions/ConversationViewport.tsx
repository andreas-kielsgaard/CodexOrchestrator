import type { ReactNode } from 'react';
import { AlertCircle, X } from 'lucide-react';
import { AgentSessionComposer } from './AgentSessionComposer';
import type { ComposerQuickFeatures } from './composerQuickActions';
import type { ComposerTargetSource } from './composerTargetActions';
import type { RuntimeInteractionResponseDto, SessionInteractionDto } from '../../application/agentSessions';
import { AgentSessionTranscript } from './AgentSessionTranscript';
import {
  projectedTranscriptContent,
  selectTranscriptRange,
  type ProjectedTranscript,
  type TranscriptAnchorRange,
} from './transcriptProjector';
import { useTranscriptFollow } from './useTranscriptFollow';
import { pendingRequestLabel, pendingSessionRequests } from './sessionAttention';
import type { AgentIdentity } from '../../application/agentSessions';

export interface ConversationViewportSegment {
  /** Caller-assigned causal position. Segments are rendered in this exact array order. */
  id: string;
  transcript: ProjectedTranscript;
  range?: TranscriptAnchorRange;
}

export interface ConversationViewportComposerTarget {
  toolbar?: ReactNode;
  preparationPanel?: ReactNode;
  quickFeatures?: ComposerQuickFeatures;
  targetSource?: ComposerTargetSource;
  steeringAvailable?: boolean;
  needsWorkingDirectory?: boolean;
  interactions?: readonly SessionInteractionDto[];
  respondToRequest?(invocationId: string, requestId: string, response: RuntimeInteractionResponseDto): Promise<void>;
  sessionId: string | null;
  draft: string;
  workingDirectory: string;
  sending: boolean;
  sendUnavailableReason?: string;
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
  agentIdentity?: AgentIdentity;
}

export function ConversationViewport({
  segments,
  loading,
  expandedProcessing,
  onToggleProcessing,
  composerTarget,
  error,
  onClearError,
  ariaLabel = 'Conversation',
  emptyState,
  composerPresentation,
  agentIdentity,
}: ConversationViewportProps) {
  const requests = composerTarget?.respondToRequest
    ? pendingSessionRequests(composerTarget.interactions)
    : [];
  const revision = segments.reduce(
    (latest, segment) => Math.max(latest, segment.transcript.presentationRevision),
    0,
  );
  const follow = useTranscriptFollow(
    segments.map((segment) => segment.id).join('|'),
    revision,
    requests[0]?.id,
  );

  return (
    <div className="agent-session-conversation">
      {composerTarget?.preparationPanel}
      {requests.length > 0 && (
        <section className="session-request-notice" aria-label="Pending agent requests">
          <span role="status">
            {pendingRequestLabel} ·{' '}
            {requests.length === 1 ? '1 request' : `${requests.length} requests`}
          </span>
          <button type="button" onClick={follow.reviewRequest}>
            Review request
          </button>
        </section>
      )}
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
            onRespondToRequest={
              segment.transcript.sessionId === composerTarget?.sessionId
                ? composerTarget?.respondToRequest
                : undefined
            }
            transcript={segment.transcript}
            content={
              segment.range
                ? selectTranscriptRange(segment.transcript, segment.range)
                : projectedTranscriptContent(segment.transcript)
            }
            loading={loading}
            expandedProcessing={expandedProcessing}
            onToggleProcessing={onToggleProcessing}
            emptyState={emptyState}
            agentIdentity={agentIdentity}
          />
        ))}
        {!segments.length && (
          <AgentSessionTranscript
            transcript={null}
            loading={loading}
            emptyState={emptyState}
            expandedProcessing={expandedProcessing}
            onToggleProcessing={onToggleProcessing}
            agentIdentity={agentIdentity}
          />
        )}
      </div>
      {composerTarget && (
        <AgentSessionComposer
          toolbar={composerTarget.toolbar}
          quickFeatures={composerTarget.quickFeatures}
          targetSource={composerTarget.targetSource}
          draft={composerTarget.draft}
          workingDirectory={composerTarget.workingDirectory}
          isNewSession={!composerTarget.sessionId}
          sending={composerTarget.sending}
          sendUnavailableReason={composerTarget.sendUnavailableReason}
          active={composerTarget.active}
          steeringAvailable={composerTarget.steeringAvailable}
          needsWorkingDirectory={composerTarget.needsWorkingDirectory}
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
