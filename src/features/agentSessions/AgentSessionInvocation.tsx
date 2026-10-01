import { ImportedTurnContent } from './ImportedTurnContent';
import { SessionInteractions } from './SessionInteractions';
import { AgentMarkdown } from './AgentMarkdown';
import { InvocationActivityTimeline } from './InvocationActivityTimeline';
import type { AgentIdentity, RuntimeInteractionResponseDto } from '../../application/agentSessions';
import { AgentIdentityBadge } from '../../components/AgentIdentityBadge';
import { RuntimeFailureGuidance } from '../agentProviders/RuntimeFailureGuidance';

import type { VisibleInvocation } from './AgentSessionTranscript';
import { SessionTransitionIndicator } from './SessionTransitionIndicator';
import type { SessionTransition } from './sessionHistoryProjector';
export function AgentSessionInvocation({
  invocation,
  expandedProcessing,
  onToggleProcessing,
  safeActivityDetails = false,
  processingHeading,
  agentIdentity,
  onRespondToRequest,
  providerChange,
  onOpenTransition,
}: {
  invocation: VisibleInvocation;
  expandedProcessing: ReadonlySet<string>;
  onToggleProcessing(invocationId: string): void;
  safeActivityDetails?: boolean;
  processingHeading?: string;
  agentIdentity?: AgentIdentity;
  onRespondToRequest?(
    invocationId: string,
    requestId: string,
    response: RuntimeInteractionResponseDto,
  ): Promise<void>;
  providerChange?: Extract<SessionTransition, { kind: 'provider' }>;
  onOpenTransition(transition: SessionTransition): void;
}) {
  return (
    <li
      className="transcript-invocation"
      key={invocation.id}
      data-invocation-id={invocation.id}
      tabIndex={-1}
    >
      {invocation.imported ? (
        <ImportedTurnContent
          history={invocation.imported}
          status={invocation.status}
          safeOnly={safeActivityDetails}
          showOutcome={invocation.showOutcome}
        />
      ) : (
        <>
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
              {invocation.attachments.length > 0 && (
                <div className="transcript-image-attachments">
                  {invocation.attachments.map((attachment) => (
                    <img
                      key={attachment.id}
                      src={`data:${attachment.mediaType};base64,${attachment.dataBase64}`}
                      alt={attachment.name}
                    />
                  ))}
                </div>
              )}
              {invocation.preparing && (
                <small className="waiting-status" role="status">
                  Waiting for setup
                </small>
              )}
            </article>
          )}
          <section
            className="agent-invocation-output"
            aria-label={`Agent response: ${invocation.outcome.label}`}
          >
            {providerChange && (
              <SessionTransitionIndicator transition={providerChange} onOpen={onOpenTransition} />
            )}
            <InvocationActivityTimeline
              invocation={invocation}
              expandedProcessing={expandedProcessing}
              onToggleProcessing={onToggleProcessing}
              safeActivityDetails={safeActivityDetails}
              processingHeading={processingHeading}
            />
            {invocation.interactions?.length ? (
              <SessionInteractions
                interactions={invocation.interactions.filter(({ kind }) => kind === 'request')}
                onRespond={onRespondToRequest}
              />
            ) : null}
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
              <>
                <p className={`invocation-outcome ${invocation.status}`} role="status">
                  <strong>{invocation.outcome.label}.</strong> {invocation.outcome.message}
                </p>
                <RuntimeFailureGuidance failure={invocation.runtimeFailure} />
              </>
            )}
            {invocation.showOutcome &&
              !invocation.finalResponse &&
              invocation.status === 'completed' && (
                <p className="invocation-outcome completed" role="status">
                  <strong>Completed without a final response.</strong>
                </p>
              )}
          </section>
        </>
      )}
    </li>
  );
}
