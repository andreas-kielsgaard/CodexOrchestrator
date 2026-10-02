import {
  projectSessionQueueTargets,
  type ProjectedQueueEntry,
  type SessionQueueEntryViewDto,
  type SessionQueueViewDto,
} from './sessionQueue';

export interface SessionQueuePresentation {
  readonly futureEntries: readonly ProjectedQueueEntry[];
  readonly deviceActivities: readonly SessionQueueEntryViewDto[];
  readonly queuedInvocationIds: ReadonlySet<string>;
  readonly activeInvocationId: string | null;
}

const terminalStates = new Set(['completed', 'canceled']);

export function presentSessionQueue(
  queue: SessionQueueViewDto | null | undefined,
): SessionQueuePresentation {
  if (!queue)
    return {
      futureEntries: [],
      deviceActivities: [],
      queuedInvocationIds: new Set(),
      activeInvocationId: null,
    };

  const projected = projectSessionQueueTargets(queue);
  const futureEntries = projected.filter(({ item }) => {
    if (terminalStates.has(item.entry.state)) return false;
    if (item.entry.item.kind === 'compaction') return true;
    if (item.message) return item.entry.state !== 'executing';
    return item.targetOperation?.phase !== 'activating';
  });
  const deviceActivities = queue.entries.filter(({ targetOperation }) =>
    targetOperation
      ? targetOperation.phase === 'pending' ||
        targetOperation.phase === 'staging' ||
        targetOperation.phase === 'activating'
      : false,
  );
  const queuedInvocationIds = new Set(
    queue.entries.flatMap(({ entry, message }) =>
      message && (entry.state === 'pending' || entry.state === 'blocked')
        ? [message.invocation.id]
        : [],
    ),
  );
  const activeInvocationId =
    queue.entries.find(({ entry, message }) => message && entry.state === 'executing')?.message
      ?.invocation.id ?? null;

  return { futureEntries, deviceActivities, queuedInvocationIds, activeInvocationId };
}
