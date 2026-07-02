import type { Artifact, ArtifactKind, EntityId, IsoDateTime } from './model';

export interface ArtifactStoreIdProvider {
  nextId(): EntityId;
}

export interface ArtifactStoreTimeProvider {
  now(): IsoDateTime;
}

export interface CreateArtifactInput {
  kind: ArtifactKind;
  title: string;
  taskId?: EntityId;
  taskRunId?: EntityId;
  conversationId?: EntityId;
  uri?: string;
  content?: string;
}

export interface ArtifactQuery {
  kind?: ArtifactKind;
  taskId?: EntityId;
  taskRunId?: EntityId;
  conversationId?: EntityId;
  limit?: number;
}

export interface ArtifactStore {
  createArtifact(input: CreateArtifactInput): Promise<Artifact>;
  queryArtifacts(query?: ArtifactQuery): Promise<Artifact[]>;
}

export class InMemoryArtifactStore implements ArtifactStore {
  private artifacts: Artifact[];

  constructor(
    private readonly ids: ArtifactStoreIdProvider,
    private readonly clock: ArtifactStoreTimeProvider,
    artifacts: readonly Artifact[] = [],
  ) {
    this.artifacts = artifacts.map(cloneArtifact);
  }

  async createArtifact(input: CreateArtifactInput): Promise<Artifact> {
    const artifact = createArtifactRecord(input, this.ids.nextId(), this.clock.now());

    this.artifacts = [...this.artifacts, artifact];

    return cloneArtifact(artifact);
  }

  async queryArtifacts(query: ArtifactQuery = {}): Promise<Artifact[]> {
    return queryStoredArtifacts(this.artifacts, query).map(cloneArtifact);
  }

  snapshot(): Artifact[] {
    return this.artifacts.map(cloneArtifact);
  }
}

export function createArtifactRecord(
  input: CreateArtifactInput,
  id: EntityId,
  createdAt: IsoDateTime,
): Artifact {
  return {
    id,
    ...(input.taskId === undefined ? {} : { taskId: input.taskId }),
    ...(input.taskRunId === undefined ? {} : { taskRunId: input.taskRunId }),
    ...(input.conversationId === undefined ? {} : { conversationId: input.conversationId }),
    kind: input.kind,
    title: input.title,
    ...(input.uri === undefined ? {} : { uri: input.uri }),
    ...(input.content === undefined ? {} : { content: input.content }),
    createdAt,
  };
}

export function queryStoredArtifacts(
  artifacts: readonly Artifact[],
  query: ArtifactQuery = {},
): Artifact[] {
  assertValidLimit(query.limit);

  const queriedArtifacts = artifacts
    .filter((artifact) => artifactMatchesQuery(artifact, query))
    .sort(compareArtifactsChronologically);

  return query.limit === undefined ? queriedArtifacts : queriedArtifacts.slice(0, query.limit);
}

export function cloneArtifact(artifact: Artifact): Artifact {
  return { ...artifact };
}

function artifactMatchesQuery(artifact: Artifact, query: ArtifactQuery): boolean {
  return (
    matchesOptionalFilter(artifact.kind, query.kind) &&
    matchesOptionalFilter(artifact.taskId, query.taskId) &&
    matchesOptionalFilter(artifact.taskRunId, query.taskRunId) &&
    matchesOptionalFilter(artifact.conversationId, query.conversationId)
  );
}

function matchesOptionalFilter<T>(value: T | undefined, filter: T | undefined): boolean {
  return filter === undefined || value === filter;
}

function compareArtifactsChronologically(left: Artifact, right: Artifact): number {
  const createdAtComparison = left.createdAt.localeCompare(right.createdAt);

  if (createdAtComparison !== 0) {
    return createdAtComparison;
  }

  return left.id.localeCompare(right.id);
}

function assertValidLimit(limit: number | undefined): void {
  if (limit === undefined) {
    return;
  }

  if (!Number.isInteger(limit) || limit < 0) {
    throw new Error(`Artifact query limit must be a non-negative integer: ${limit}`);
  }
}
