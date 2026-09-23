import type {
  AgentSessionDetailsDto,
  AgentSessionHistorySnapshot,
  AgentSessionHistorySource,
  AgentSessionUpdateDto,
  AgentSessionUpdateListener,
} from '../../application/agentSessions';
import { reduceAgentSessionHistory } from '../../application/agentSessions';

interface HistorySourceDependencies {
  load(sessionId: string): Promise<AgentSessionDetailsDto>;
  subscribe(listener: AgentSessionUpdateListener): Promise<() => void>;
  schedule?(publish: () => void): void;
  maxCachedSessions?: number;
}

interface HistoryEntry {
  snapshot: AgentSessionHistorySnapshot;
  workingDetails: AgentSessionDetailsDto | null;
  listeners: Set<() => void>;
  pendingChanges: AgentSessionUpdateDto[];
  loadPromise?: Promise<AgentSessionDetailsDto>;
  publishScheduled: boolean;
  touchedAt: number;
}

const initialSnapshot = (): AgentSessionHistorySnapshot => ({
  details: null,
  loading: false,
  refreshing: false,
  error: null,
  revision: 0,
  changes: [],
});

/** One app-scoped, bounded live-history cache shared by every Agent Session view. */
export function createAgentSessionHistorySource({
  load,
  subscribe,
  schedule = defaultSchedule,
  maxCachedSessions = 5,
}: HistorySourceDependencies): AgentSessionHistorySource {
  const entries = new Map<string, HistoryEntry>();
  let updateBridge: Promise<() => void> | null = null;
  let clock = 0;

  const entryFor = (sessionId: string): HistoryEntry => {
    const existing = entries.get(sessionId);
    if (existing) {
      existing.touchedAt = ++clock;
      return existing;
    }
    const created: HistoryEntry = {
      snapshot: initialSnapshot(),
      workingDetails: null,
      listeners: new Set(),
      pendingChanges: [],
      publishScheduled: false,
      touchedAt: ++clock,
    };
    entries.set(sessionId, created);
    evictInactiveEntries();
    return created;
  };

  const evictInactiveEntries = () => {
    if (entries.size <= maxCachedSessions) return;
    const candidates = [...entries.entries()]
      .filter(([, entry]) => entry.listeners.size === 0 && !entry.loadPromise)
      .sort((left, right) => left[1].touchedAt - right[1].touchedAt);
    while (entries.size > maxCachedSessions && candidates.length) {
      entries.delete(candidates.shift()![0]);
    }
  };

  const notify = (entry: HistoryEntry) => {
    for (const listener of entry.listeners) listener();
  };

  const publishWorking = (entry: HistoryEntry) => {
    entry.publishScheduled = false;
    if (!entry.workingDetails || !entry.pendingChanges.length) return;
    entry.snapshot = {
      details: entry.workingDetails,
      loading: false,
      refreshing: false,
      error: null,
      revision: entry.snapshot.revision + 1,
      changes: entry.pendingChanges,
    };
    entry.pendingChanges = [];
    notify(entry);
  };

  const queuePublish = (entry: HistoryEntry, immediate: boolean) => {
    if (immediate) {
      publishWorking(entry);
      return;
    }
    if (entry.publishScheduled) return;
    entry.publishScheduled = true;
    schedule(() => publishWorking(entry));
  };

  const loadInto = (
    sessionId: string,
    entry: HistoryEntry,
    refreshing: boolean,
  ): Promise<AgentSessionDetailsDto> => {
    if (entry.loadPromise) return entry.loadPromise;
    entry.snapshot = {
      ...entry.snapshot,
      loading: !entry.snapshot.details,
      refreshing: Boolean(entry.snapshot.details) && refreshing,
      error: null,
      changes: [],
    };
    notify(entry);
    const request = load(sessionId).then(
      (details) => {
        entry.loadPromise = undefined;
        entry.workingDetails = details;
        entry.pendingChanges = [];
        entry.snapshot = {
          details,
          loading: false,
          refreshing: false,
          error: null,
          revision: entry.snapshot.revision + 1,
          changes: [],
        };
        notify(entry);
        return details;
      },
      (cause: unknown) => {
        entry.loadPromise = undefined;
        entry.snapshot = {
          ...entry.snapshot,
          loading: false,
          refreshing: false,
          error: cause instanceof Error ? cause.message : String(cause),
          changes: [],
        };
        notify(entry);
        throw cause;
      },
    );
    entry.loadPromise = request;
    void request
      .finally(() => {
        if (entry.loadPromise === request) entry.loadPromise = undefined;
        evictInactiveEntries();
      })
      .catch(() => undefined);
    return request;
  };

  const refreshEntry = (sessionId: string, entry: HistoryEntry) => loadInto(sessionId, entry, true);

  const applyUpdate = (update: AgentSessionUpdateDto) => {
    const entry = entries.get(update.sessionId);
    if (!entry?.workingDetails) return;
    const reduction = reduceAgentSessionHistory(entry.workingDetails, update);
    if (reduction.kind === 'unchanged') return;
    if (reduction.kind === 'reconcile') {
      void refreshEntry(update.sessionId, entry).catch(() => undefined);
      return;
    }
    entry.workingDetails = reduction.details;
    entry.pendingChanges.push(update);
    queuePublish(
      entry,
      update.kind === 'invocation_terminal' || update.kind === 'diagnostic_recorded',
    );
  };

  const ensureUpdateBridge = () => {
    updateBridge ??= subscribe(applyUpdate).catch((cause) => {
      updateBridge = null;
      throw cause;
    });
    return updateBridge;
  };

  return {
    getSnapshot(sessionId) {
      return entryFor(sessionId).snapshot;
    },
    subscribe(sessionId, listener) {
      const entry = entryFor(sessionId);
      entry.listeners.add(listener);
      void ensureUpdateBridge().catch((cause: unknown) => {
        entry.snapshot = {
          ...entry.snapshot,
          error: cause instanceof Error ? cause.message : String(cause),
        };
        notify(entry);
      });
      if (!entry.workingDetails) void loadInto(sessionId, entry, false).catch(() => undefined);
      return () => {
        entry.listeners.delete(listener);
        evictInactiveEntries();
      };
    },
    ensure(sessionId) {
      const entry = entryFor(sessionId);
      void ensureUpdateBridge().catch(() => undefined);
      return entry.workingDetails
        ? Promise.resolve(entry.workingDetails)
        : loadInto(sessionId, entry, false);
    },
    refresh(sessionId) {
      const entry = entryFor(sessionId);
      void ensureUpdateBridge().catch(() => undefined);
      return refreshEntry(sessionId, entry);
    },
  };
}

function defaultSchedule(publish: () => void) {
  if (typeof requestAnimationFrame === 'function') requestAnimationFrame(publish);
  else setTimeout(publish, 0);
}
