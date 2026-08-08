import { useEffect, useId, useRef, useState } from 'react';
import {
  ConversationViewport,
  embeddedSessionIsWritable,
  AgentSessionTurnInspector,
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
  readonly onOpenStandalone?: (sessionId: string) => void;
  readonly displayMode?: 'collapsible' | 'always_open';
  readonly focusInvocationId?: string;
  readonly focusRequest?: number;
  /** Explicitly inspects one Session turn and suppresses all continuation controls. */
  readonly inspection?: { readonly invocationId: string };
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
  onOpenStandalone,
  displayMode = 'collapsible',
  focusInvocationId,
  focusRequest,
  inspection,
}: SharedAgentSessionPanelProps) {
  const conversationId = useId();
  const [internalExpanded, setInternalExpanded] = useState(defaultExpanded);
  const hostRef = useRef<HTMLElement>(null);
  const expanded = displayMode === 'always_open' || (controlledExpanded ?? internalExpanded);
  const setExpanded = (next: boolean) => {
    setInternalExpanded(next);
    onExpandedChange?.(next);
  };

  useEffect(() => {
    if (!focusInvocationId) return;
    const target = Array.from(
      hostRef.current?.querySelectorAll<HTMLElement>('[data-invocation-id]') ?? [],
    ).find((candidate) => candidate.dataset.invocationId === focusInvocationId);
    target?.scrollIntoView?.({ block: 'center' });
    target?.focus({ preventScroll: true });
  }, [focusInvocationId, focusRequest]);

  return (
    <section
      ref={hostRef}
      className={`shared-agent-session${expanded ? ' is-expanded' : ''}`}
      aria-label={ariaLabel}
      data-session-id={session.sessionId}
    >
      {!expanded ? (
        <div className="shared-agent-session__compact">
          <span>Agent Session</span>
          <strong>{session.title}</strong>
          <div className="shared-agent-session__actions">
            <button
              className="shared-agent-session__open"
              type="button"
              aria-expanded="false"
              aria-controls={conversationId}
              onClick={() => setExpanded(true)}
            >
              Open Agent Session
            </button>
            {onOpenStandalone && (
              <button
                className="shared-agent-session__standalone"
                type="button"
                onClick={() => onOpenStandalone(session.sessionId)}
              >
                Open in Agent Sessions
              </button>
            )}
          </div>
        </div>
      ) : (
        <div
          id={conversationId}
          className="shared-agent-session__conversation"
          data-display-mode={displayMode}
        >
          <header className="shared-agent-session__heading">
            <span>Agent Session</span>
            <strong>{session.title}</strong>
            <div className="shared-agent-session__actions">
              {displayMode === 'collapsible' ? (
                <button
                  className="shared-agent-session__collapse"
                  type="button"
                  aria-expanded="true"
                  aria-controls={conversationId}
                  onClick={() => setExpanded(false)}
                >
                  Collapse Agent Session
                </button>
              ) : null}
              {onOpenStandalone ? (
                <button
                  className="shared-agent-session__standalone"
                  type="button"
                  onClick={() => onOpenStandalone(session.sessionId)}
                >
                  Open in Agent Sessions
                </button>
              ) : null}
            </div>
          </header>
          {composition ? (
            <ConnectedAgentSessionConversation
              session={session}
              composition={composition}
              ariaLabel={conversationAriaLabel}
              inspection={inspection}
            />
          ) : (
            <ReadOnlyAgentSessionConversation
              session={session}
              ariaLabel={conversationAriaLabel}
              inspection={inspection}
            />
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
  inspection,
}: {
  readonly session: SharedAgentSessionPresentation;
  readonly composition: EmbeddedAgentSessionComposition;
  readonly ariaLabel: string;
  readonly inspection?: { readonly invocationId: string };
}) {
  const controller = useAgentSession(composition.client, {
    selectedSessionId: session.sessionId,
  });
  const writable = embeddedSessionIsWritable(composition, session.sessionId);
  const ready = Boolean(controller.details) && !controller.error;
  const transcript = controller.transcript;

  if (inspection) {
    return (
      <AgentSessionTurnInspector
        sessionId={session.sessionId}
        invocationId={inspection.invocationId}
        transcript={transcript}
        loading={controller.loading}
        error={controller.error}
      />
    );
  }

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
  inspection,
}: {
  readonly session: SharedAgentSessionPresentation;
  readonly ariaLabel: string;
  readonly inspection?: { readonly invocationId: string };
}) {
  const [expandedProcessing, setExpandedProcessing] = useState<ReadonlySet<string>>(new Set());
  if (inspection) {
    return (
      <AgentSessionTurnInspector
        sessionId={session.sessionId}
        invocationId={inspection.invocationId}
        transcript={session.transcript ?? null}
      />
    );
  }
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
