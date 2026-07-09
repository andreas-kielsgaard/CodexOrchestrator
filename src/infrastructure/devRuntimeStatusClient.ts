import {
  unavailableRuntimeStatus,
  type RuntimeStatusClient,
  type RuntimeStatusSnapshot,
  type RuntimeStatusTarget,
} from '../application/queries/runtimeStatusClient';

const defaultStatusUrl = 'http://127.0.0.1:41415/status';

interface RawRuntimeStatus {
  stale?: unknown;
  staleTargets?: unknown;
  reason?: unknown;
  generation?: unknown;
  markedAt?: unknown;
}

export function createDevRuntimeStatusClient(
  statusUrl: string = configuredStatusUrl(),
): RuntimeStatusClient {
  return {
    async checkStatus(): Promise<RuntimeStatusSnapshot> {
      try {
        const response = await fetch(statusUrl, { cache: 'no-store' });

        if (!response.ok) {
          return unavailableRuntimeStatus();
        }

        return normalizeRuntimeStatus((await response.json()) as RawRuntimeStatus);
      } catch {
        return unavailableRuntimeStatus();
      }
    },
  };
}

function configuredStatusUrl(): string {
  const viteEnv = (import.meta as unknown as { env?: Record<string, string | undefined> }).env;
  return viteEnv?.VITE_RUNTIME_STATUS_URL ?? defaultStatusUrl;
}

function normalizeRuntimeStatus(raw: RawRuntimeStatus): RuntimeStatusSnapshot {
  return {
    available: true,
    stale: raw.stale === true,
    staleTargets: normalizeTargets(raw.staleTargets),
    ...(typeof raw.reason === 'string' && raw.reason.trim() ? { reason: raw.reason.trim() } : {}),
    ...(typeof raw.generation === 'string' ? { generation: raw.generation } : {}),
    ...(typeof raw.markedAt === 'string' ? { markedAt: raw.markedAt } : {}),
    checkedAt: new Date().toISOString(),
  };
}

function normalizeTargets(value: unknown): RuntimeStatusTarget[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value.filter(isRuntimeStatusTarget);
}

function isRuntimeStatusTarget(value: unknown): value is RuntimeStatusTarget {
  return value === 'app' || value === 'frontend' || value === 'backend';
}
