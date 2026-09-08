import type { EventDeliveryRecord, PromptSource } from './types';
import { promptSourceText, referenceIdentityLabel } from './types';
import './sessionEvents.css';

export interface EventDeliveryListProps {
  readonly deliveries: readonly EventDeliveryRecord[];
  readonly emptyMessage?: string;
}

function contributionLabel(source: PromptSource): string {
  switch (source.kind) {
    case 'literal':
      return 'Fixed text';
    case 'user_request_text':
      return 'User request';
    case 'invocation_output':
      return 'Invocation output';
    case 'mcp_argument':
      return `MCP argument: ${source.name}`;
    case 'application_event_field':
      return `Application event field: ${source.field}`;
    case 'referenced_content':
      return `Referenced content: ${referenceIdentityLabel(source.reference)}`;
  }
}

function PromptContributions({
  sources,
  emptyMessage,
}: {
  readonly sources: readonly PromptSource[];
  readonly emptyMessage: string;
}) {
  if (sources.length === 0) return <p className="session-event-record__muted">{emptyMessage}</p>;
  return (
    <ol className="session-event-record__contributions">
      {sources.map((source, index) => (
        <li key={index}>
          <strong>{contributionLabel(source)}</strong>
          <pre>{promptSourceText(source)}</pre>
        </li>
      ))}
    </ol>
  );
}

export function EventDeliveryList({
  deliveries,
  emptyMessage = 'No deliveries were recorded.',
}: EventDeliveryListProps) {
  if (deliveries.length === 0) {
    return <p className="session-event-record__empty">{emptyMessage}</p>;
  }

  return (
    <ol className="event-delivery-list" aria-label="Event deliveries">
      {deliveries.map((delivery) => (
        <li className="event-delivery-list__item" key={referenceIdentityLabel(delivery.deliveryId)}>
          <details>
            <summary>
              <span title={referenceIdentityLabel(delivery.targetSession)}>
                Delivery {delivery.ordinal} · {delivery.targetSession.id}
              </span>
              <span
                className={`session-event-record__status is-${delivery.outcome.kind}`}
                aria-label={`Outcome: ${delivery.outcome.kind}`}
              >
                {delivery.outcome.kind === 'dispatched' ? 'Dispatched' : 'Failed'}
              </span>
            </summary>
            <dl className="session-event-record__facts">
              <div>
                <dt>Delivery</dt>
                <dd>{referenceIdentityLabel(delivery.deliveryId)}</dd>
              </div>
              <div>
                <dt>Target</dt>
                <dd>{referenceIdentityLabel(delivery.targetSession)}</dd>
              </div>
              <div>
                <dt>Session created</dt>
                <dd>{delivery.targetCreated ? 'Yes' : 'No'}</dd>
              </div>
              {delivery.addressedSequence !== null && (
                <div>
                  <dt>Address sequence</dt>
                  <dd>{delivery.addressedSequence}</dd>
                </div>
              )}
              {delivery.outcome.kind === 'dispatched' && (
                <div>
                  <dt>Invocation</dt>
                  <dd>{referenceIdentityLabel(delivery.outcome.invocation)}</dd>
                </div>
              )}
            </dl>
            {delivery.addressingError && (
              <p className="session-event-record__error" role="alert">
                Addressing record failed: {delivery.addressingError}
              </p>
            )}
            {delivery.outcome.kind === 'failed' && (
              <p className="session-event-record__error" role="alert">
                Delivery failed: {delivery.outcome.message}
              </p>
            )}
            <h4>Prompt contributions</h4>
            <PromptContributions
              sources={delivery.promptContributions}
              emptyMessage="No message prompt contributions were recorded."
            />
            {delivery.targetCreated && (
              <>
                <h4>Created Session contributions</h4>
                <PromptContributions
                  sources={delivery.includedCreatedSessionContributions}
                  emptyMessage="No created-Session contributions were included."
                />
              </>
            )}
          </details>
        </li>
      ))}
    </ol>
  );
}
