import { useId } from 'react';
import { EventDeliveryList } from './EventDeliveryList';
import type {
  EventDeliveryRecord,
  EventGroupRecord,
  SessionEventSource,
  SessionEventTrigger,
} from './types';
import { referenceIdentityLabel } from './types';
import './sessionEvents.css';

export interface EventGroupInspectorProps {
  readonly group: EventGroupRecord;
  readonly deliveries: readonly EventDeliveryRecord[];
}

function triggerLabel(trigger: SessionEventTrigger): string {
  switch (trigger.kind) {
    case 'user_request':
      return `User request · ${referenceIdentityLabel(trigger.request)}`;
    case 'invocation_completed':
      return `Invocation completed · ${referenceIdentityLabel(trigger.invocation)}`;
    case 'mcp_call':
      return `MCP call · ${referenceIdentityLabel(trigger.tool)}`;
    case 'application_event':
      return `Application event · ${referenceIdentityLabel(trigger.event)}`;
    case 'event_group_completed':
      return `Event group completed · ${referenceIdentityLabel(trigger.eventGroup)}`;
  }
}

function sourceLabel(source: SessionEventSource): string {
  switch (source.kind) {
    case 'user_request':
      return referenceIdentityLabel(source.request);
    case 'application':
      return referenceIdentityLabel(source.component);
    case 'session_invocation':
      return referenceIdentityLabel(source.invocation);
    case 'mcp_call':
      return referenceIdentityLabel(source.call);
    case 'application_event':
      return referenceIdentityLabel(source.event);
    case 'event_group':
      return referenceIdentityLabel(source.eventGroup);
    case 'external_reference':
      return referenceIdentityLabel(source.reference);
  }
}

function outcomeLabel(outcome: EventGroupRecord['outcome']): string {
  switch (outcome) {
    case 'delivered':
      return 'Delivered';
    case 'partially_delivered':
      return 'Partially delivered';
    case 'delivery_failed':
      return 'Delivery failed';
    case 'no_target':
      return 'No target';
    case 'noop':
      return 'No action';
  }
}

export function EventGroupInspector({ group, deliveries }: EventGroupInspectorProps) {
  const headingId = useId();
  return (
    <section className="session-event-record" aria-labelledby={headingId}>
      <header className="session-event-record__header">
        <div>
          <p className="session-event-record__eyebrow">Session event</p>
          <h3 id={headingId}>{referenceIdentityLabel(group.eventGroupId)}</h3>
        </div>
        <span
          className={`session-event-record__status is-${group.outcome}`}
          aria-label={`Outcome: ${outcomeLabel(group.outcome)}`}
        >
          {outcomeLabel(group.outcome)}
        </span>
      </header>
      <dl className="session-event-record__facts">
        <div>
          <dt>Definition</dt>
          <dd>{referenceIdentityLabel(group.definitionRef)}</dd>
        </div>
        <div>
          <dt>Trigger</dt>
          <dd>{triggerLabel(group.trigger)}</dd>
        </div>
        <div>
          <dt>Source</dt>
          <dd>{sourceLabel(group.source)}</dd>
        </div>
        <div>
          <dt>Resolved Sessions</dt>
          <dd>{group.resolvedSessions.length}</dd>
        </div>
        {group.createdSession && (
          <div>
            <dt>Created Session</dt>
            <dd>{referenceIdentityLabel(group.createdSession)}</dd>
          </div>
        )}
      </dl>
      <h3 className="session-event-record__subheading">
        Deliveries <span>{group.deliveryCount}</span>
      </h3>
      <EventDeliveryList deliveries={deliveries} />
    </section>
  );
}
