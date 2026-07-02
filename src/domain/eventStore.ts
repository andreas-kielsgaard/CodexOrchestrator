import type { EntityId, Event, EventKind, IsoDateTime } from './model';

export interface EventStoreIdProvider {
  nextId(): EntityId;
}

export interface EventStoreTimeProvider {
  now(): IsoDateTime;
}

export interface AppendEventInput {
  kind: EventKind;
  projectId?: EntityId;
  taskId?: EntityId;
  taskRunId?: EntityId;
  conversationId?: EntityId;
  artifactId?: EntityId;
  validationRunId?: EntityId;
  payload?: Record<string, unknown>;
}

export interface EventQuery {
  kind?: EventKind;
  projectId?: EntityId;
  taskId?: EntityId;
  taskRunId?: EntityId;
  conversationId?: EntityId;
  artifactId?: EntityId;
  validationRunId?: EntityId;
  limit?: number;
}

export interface EventStore {
  appendEvent(input: AppendEventInput): Promise<Event>;
  queryEvents(query?: EventQuery): Promise<Event[]>;
}

export class InMemoryEventStore implements EventStore {
  private events: Event[];

  constructor(
    private readonly ids: EventStoreIdProvider,
    private readonly clock: EventStoreTimeProvider,
    events: readonly Event[] = [],
  ) {
    this.events = events.map(cloneEvent);
  }

  async appendEvent(input: AppendEventInput): Promise<Event> {
    const event: Event = {
      id: this.ids.nextId(),
      kind: input.kind,
      occurredAt: this.clock.now(),
      ...(input.projectId === undefined ? {} : { projectId: input.projectId }),
      ...(input.taskId === undefined ? {} : { taskId: input.taskId }),
      ...(input.taskRunId === undefined ? {} : { taskRunId: input.taskRunId }),
      ...(input.conversationId === undefined ? {} : { conversationId: input.conversationId }),
      ...(input.artifactId === undefined ? {} : { artifactId: input.artifactId }),
      ...(input.validationRunId === undefined ? {} : { validationRunId: input.validationRunId }),
      payload: clonePayload(input.payload ?? {}),
    };

    this.events = [...this.events, event];

    return cloneEvent(event);
  }

  async queryEvents(query: EventQuery = {}): Promise<Event[]> {
    return queryStoredEvents(this.events, query).map(cloneEvent);
  }

  snapshot(): Event[] {
    return this.events.map(cloneEvent);
  }
}

export function queryStoredEvents(events: readonly Event[], query: EventQuery = {}): Event[] {
  assertValidLimit(query.limit);

  const queriedEvents = events
    .filter((event) => eventMatchesQuery(event, query))
    .sort(compareEventsChronologically);

  return query.limit === undefined ? queriedEvents : queriedEvents.slice(0, query.limit);
}

export function cloneEvent(event: Event): Event {
  return {
    ...event,
    payload: clonePayload(event.payload),
  };
}

function clonePayload(payload: Record<string, unknown>): Record<string, unknown> {
  let json: string;

  try {
    json = JSON.stringify(payload);
  } catch (error) {
    throw new Error(`Event payload is not JSON serializable: ${(error as Error).message}`);
  }

  if (json === undefined) {
    throw new Error('Event payload is not JSON serializable');
  }

  const parsed = JSON.parse(json) as unknown;

  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Event payload must be a JSON object');
  }

  return parsed as Record<string, unknown>;
}

function eventMatchesQuery(event: Event, query: EventQuery): boolean {
  return (
    matchesOptionalFilter(event.kind, query.kind) &&
    matchesOptionalFilter(event.projectId, query.projectId) &&
    matchesOptionalFilter(event.taskId, query.taskId) &&
    matchesOptionalFilter(event.taskRunId, query.taskRunId) &&
    matchesOptionalFilter(event.conversationId, query.conversationId) &&
    matchesOptionalFilter(event.artifactId, query.artifactId) &&
    matchesOptionalFilter(event.validationRunId, query.validationRunId)
  );
}

function matchesOptionalFilter<T>(value: T | undefined, filter: T | undefined): boolean {
  return filter === undefined || value === filter;
}

function compareEventsChronologically(left: Event, right: Event): number {
  const occurredAtComparison = left.occurredAt.localeCompare(right.occurredAt);

  if (occurredAtComparison !== 0) {
    return occurredAtComparison;
  }

  return left.id.localeCompare(right.id);
}

function assertValidLimit(limit: number | undefined): void {
  if (limit === undefined) {
    return;
  }

  if (!Number.isInteger(limit) || limit < 0) {
    throw new Error(`Event query limit must be a non-negative integer: ${limit}`);
  }
}
