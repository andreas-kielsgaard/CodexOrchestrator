import { fireEvent, render, screen } from '@testing-library/react';
import { EventGroupInspector } from './EventGroupInspector';
import type { EventDeliveryRecord, EventGroupRecord, ReferenceIdentity } from './types';

const reference = (kind: string, id: string): ReferenceIdentity => ({
  namespace: 'test',
  kind,
  id,
});

const group: EventGroupRecord = {
  eventGroupId: reference('event_group', 'group-1'),
  definitionRef: reference('event_definition', 'definition-1'),
  trigger: { kind: 'user_request', request: reference('request', 'request-1') },
  source: { kind: 'user_request', request: reference('request', 'request-1') },
  promptSources: [
    { kind: 'user_request_text', request: reference('request', 'request-1'), text: 'Review this.' },
  ],
  createdSessionPromptSources: [],
  targetSelection: {
    target: { kind: 'exact', session: reference('session', 'session-1') },
    cardinality: 'first',
    ordering: 'newest',
    running: 'any',
    createdBy: null,
    missing: 'fail',
  },
  resolvedSessions: [reference('session', 'session-1')],
  createdSession: null,
  outcome: 'delivered',
  deliveryCount: 1,
};

const deliveries: EventDeliveryRecord[] = [
  {
    deliveryId: reference('delivery', 'delivery-1'),
    eventGroupId: group.eventGroupId,
    ordinal: 1,
    targetSession: reference('session', 'session-1'),
    logicalAddress: null,
    targetCreated: false,
    promptContributions: group.promptSources,
    includedCreatedSessionContributions: [],
    addressedSequence: 3,
    addressingError: null,
    outcome: { kind: 'dispatched', invocation: reference('invocation', 'invocation-1') },
  },
];

describe('EventGroupInspector', () => {
  it('shows group truth and reveals delivery provenance', () => {
    render(<EventGroupInspector group={group} deliveries={deliveries} />);

    expect(screen.getByRole('heading', { name: 'test:event_group/group-1' })).toBeVisible();
    expect(screen.getByLabelText('Outcome: Delivered')).toBeVisible();
    expect(screen.getByText('User request · test:request/request-1')).toBeVisible();

    fireEvent.click(screen.getByText('Delivery 1 · session-1'));
    expect(screen.getByText('test:invocation/invocation-1')).toBeVisible();
    expect(screen.getByText('Review this.')).toBeVisible();
  });
});
