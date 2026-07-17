import { useId, useState } from 'react';
import {
  ConversationViewport,
  embeddedSessionIsWritable,
  type EmbeddedAgentSessionComposition,
  type ProjectedTranscript,
  type TranscriptAnchorRange,
  useAgentSession,
} from '../../agentSessions';
import '../styles/sharedAgentSessionPanel.css';

export interface SharedAgentSessionPresentation {
  readonly sessionId: string;
  readonly title: string;
  readonly transcript?: ProjectedTranscript;
  readonly latestAgentTurnRange?: TranscriptAnchorRange;
}

export interface SharedAgentSessionPanelProps {
  readonly ariaLabel: string;
  readonly conversationAriaLabel: string;
  readonly session: SharedAgentSessionPresentation;
  readonly composition?: EmbeddedAgentSessionComposition;
  readonly defaultExpanded?: boolean;
  readonly expanded?: boolean;
  readonly onExpandedChange?: (expanded: boolean) => void;
}

/** Shared embedded Agent Session presentation with an injected application boundary. */
export function SharedAgentSessionPanel({
  ariaLabel,
  conversationAriaLabel,
  session,
  composition,
  defaultExpanded = false,
  expanded: controlledExpanded,
  onExpandedChange,
}: SharedAgentSessionPanelProps) {
  const conversationId = useId();
  const [internalExpanded, setInternalExpanded] = useState(defaultExpanded);
  const expanded = controlledExpanded ?? internalExpanded;
  const setExpanded = (next: boolean) => {
    setInternalExpanded(next);
    onExpandedChange?.(next);
  };

  return (
    <section
      className={`shared-agent-session${expanded ? ' is-expanded' : ''}`}
      aria-label={ariaLabel}
    >
      {!expanded ? (
        <div className="shared-agent-session__compact">
          <span>Agent Session</span>
          <strong>{session.title}</strong>
          <button
            className="shared-agent-session__open"
            type="button"
            aria-expanded="false"
            aria-controls={conversationId}
            onClick={() => setExpanded(true)}
          >
            Open Agent Session
          </button>
        </div>
      ) : (
        <div id={conversationId} className="shared-agent-session__conversation">
          <button
            className="shared-agent-session__collapse"
            type="button"
            aria-expanded="true"
            aria-controls={conversationId}
            onClick={() => setExpanded(false)}
          >
            Collapse Agent Session
          </button>
          {composition ? (
            <ConnectedAgentSessionConversation
              session={session}
              composition={composition}
              ariaLabel={conversationAriaLabel}
            />
          ) : (
            <ReadOnlyAgentSessionConversation session={session} ariaLabel={conversationAriaLabel} />
          )}
        </div>
      )}
    </section>
  );
}

function ConnectedAgentSessionConversation({
  session,
  composition,
  ariaLabel,
}: {
  readonly session: SharedAgentSessionPresentation;
  readonly composition: EmbeddedAgentSessionComposition;
  readonly ariaLabel: string;
}) {
  const controller = useAgentSession(composition.client, {
    selectedSessionId: session.sessionId,
  });
  const writable = embeddedSessionIsWritable(composition, session.sessionId);
  const ready = Boolean(controller.details) && !controller.error;
  const transcript = controller.transcript;

  return (
    <ConversationViewport
      segments={transcript ? [{ id: transcript.sessionId, transcript }] : []}
      loading={controller.loading}
      expandedProcessing={controller.expandedProcessing}
      onToggleProcessing={controller.toggleProcessing}
      error={controller.error}
      onClearError={controller.clearError}
      composerTarget={
        writable && ready
          ? {
              sessionId: controller.selectedSessionId,
              draft: controller.draft,
              workingDirectory: controller.workingDirectory,
              sending: controller.sending,
              active: Boolean(transcript?.activeInvocationId),
              canceling: controller.canceling,
              setDraft: controller.setDraft,
              setWorkingDirectory: controller.setWorkingDirectory,
              send: controller.send,
              cancel: controller.cancel,
            }
          : undefined
      }
      ariaLabel={ariaLabel}
    />
  );
}

function ReadOnlyAgentSessionConversation({
  session,
  ariaLabel,
}: {
  readonly session: SharedAgentSessionPresentation;
  readonly ariaLabel: string;
}) {
  const [expandedProcessing, setExpandedProcessing] = useState<ReadonlySet<string>>(new Set());
  return (
    <ConversationViewport
      segments={
        session.transcript ? [{ id: session.sessionId, transcript: session.transcript }] : []
      }
      loading={false}
      expandedProcessing={expandedProcessing}
      onToggleProcessing={(invocationId) => {
        setExpandedProcessing((current) => {
          const next = new Set(current);
          if (next.has(invocationId)) next.delete(invocationId);
          else next.add(invocationId);
          return next;
        });
      }}
      ariaLabel={ariaLabel}
    />
  );
}
