import type { Conversation, EntityId, IsoDateTime } from './model';

export interface ConversationStoreIdProvider {
  nextId(): EntityId;
}

export interface ConversationStoreTimeProvider {
  now(): IsoDateTime;
}

export interface CreateConversationInput {
  provider: Conversation['provider'];
  title: string;
  taskId?: EntityId;
  taskRunId?: EntityId;
  externalThreadId?: string;
  summary?: string;
}

export interface UpdateConversationInput {
  taskId?: EntityId | null;
  taskRunId?: EntityId | null;
  externalThreadId?: string | null;
  title?: string;
  summary?: string | null;
}

export interface ConversationQuery {
  provider?: Conversation['provider'];
  taskId?: EntityId;
  taskRunId?: EntityId;
  externalThreadId?: string;
  limit?: number;
}

export interface ConversationStore {
  createConversation(input: CreateConversationInput): Promise<Conversation>;
  updateConversation(
    conversationId: EntityId,
    input: UpdateConversationInput,
  ): Promise<Conversation>;
  queryConversations(query?: ConversationQuery): Promise<Conversation[]>;
}

export class ConversationNotFoundError extends Error {
  constructor(conversationId: EntityId) {
    super(`Conversation not found: ${conversationId}`);
    this.name = 'ConversationNotFoundError';
  }
}

export class InMemoryConversationStore implements ConversationStore {
  private conversations: Conversation[];

  constructor(
    private readonly ids: ConversationStoreIdProvider,
    private readonly clock: ConversationStoreTimeProvider,
    conversations: readonly Conversation[] = [],
  ) {
    this.conversations = conversations.map(cloneConversation);
  }

  async createConversation(input: CreateConversationInput): Promise<Conversation> {
    const now = this.clock.now();
    const conversation = createConversationRecord(input, this.ids.nextId(), now);

    this.conversations = [...this.conversations, conversation];

    return cloneConversation(conversation);
  }

  async updateConversation(
    conversationId: EntityId,
    input: UpdateConversationInput,
  ): Promise<Conversation> {
    const conversationIndex = this.conversations.findIndex(
      (conversation) => conversation.id === conversationId,
    );

    if (conversationIndex === -1) {
      throw new ConversationNotFoundError(conversationId);
    }

    const updatedConversation = applyConversationUpdate(
      this.conversations[conversationIndex],
      input,
      this.clock.now(),
    );

    this.conversations = this.conversations.map((conversation, index) =>
      index === conversationIndex ? updatedConversation : conversation,
    );

    return cloneConversation(updatedConversation);
  }

  async queryConversations(query: ConversationQuery = {}): Promise<Conversation[]> {
    return queryStoredConversations(this.conversations, query).map(cloneConversation);
  }

  snapshot(): Conversation[] {
    return this.conversations.map(cloneConversation);
  }
}

export function createConversationRecord(
  input: CreateConversationInput,
  id: EntityId,
  now: IsoDateTime,
): Conversation {
  return {
    id,
    ...(input.taskId === undefined ? {} : { taskId: input.taskId }),
    ...(input.taskRunId === undefined ? {} : { taskRunId: input.taskRunId }),
    provider: input.provider,
    ...(input.externalThreadId === undefined ? {} : { externalThreadId: input.externalThreadId }),
    title: input.title,
    ...(input.summary === undefined ? {} : { summary: input.summary }),
    createdAt: now,
    updatedAt: now,
  };
}

export function applyConversationUpdate(
  conversation: Conversation,
  input: UpdateConversationInput,
  updatedAt: IsoDateTime,
): Conversation {
  const updatedConversation: Conversation = {
    ...conversation,
    ...(input.title === undefined ? {} : { title: input.title }),
    updatedAt,
  };

  applyOptionalField(updatedConversation, 'taskId', input.taskId);
  applyOptionalField(updatedConversation, 'taskRunId', input.taskRunId);
  applyOptionalField(updatedConversation, 'externalThreadId', input.externalThreadId);
  applyOptionalField(updatedConversation, 'summary', input.summary);

  return updatedConversation;
}

export function queryStoredConversations(
  conversations: readonly Conversation[],
  query: ConversationQuery = {},
): Conversation[] {
  assertValidLimit(query.limit);

  const queriedConversations = conversations
    .filter((conversation) => conversationMatchesQuery(conversation, query))
    .sort(compareConversationsChronologically);

  return query.limit === undefined
    ? queriedConversations
    : queriedConversations.slice(0, query.limit);
}

export function cloneConversation(conversation: Conversation): Conversation {
  return { ...conversation };
}

function conversationMatchesQuery(conversation: Conversation, query: ConversationQuery): boolean {
  return (
    matchesOptionalFilter(conversation.provider, query.provider) &&
    matchesOptionalFilter(conversation.taskId, query.taskId) &&
    matchesOptionalFilter(conversation.taskRunId, query.taskRunId) &&
    matchesOptionalFilter(conversation.externalThreadId, query.externalThreadId)
  );
}

function matchesOptionalFilter<T>(value: T | undefined, filter: T | undefined): boolean {
  return filter === undefined || value === filter;
}

function compareConversationsChronologically(left: Conversation, right: Conversation): number {
  const createdAtComparison = left.createdAt.localeCompare(right.createdAt);

  if (createdAtComparison !== 0) {
    return createdAtComparison;
  }

  return left.id.localeCompare(right.id);
}

function applyOptionalField<T extends keyof Conversation>(
  conversation: Conversation,
  field: T,
  value: Conversation[T] | null | undefined,
): void {
  if (value === undefined) {
    return;
  }

  if (value === null) {
    delete conversation[field];
    return;
  }

  conversation[field] = value;
}

function assertValidLimit(limit: number | undefined): void {
  if (limit === undefined) {
    return;
  }

  if (!Number.isInteger(limit) || limit < 0) {
    throw new Error(`Conversation query limit must be a non-negative integer: ${limit}`);
  }
}
