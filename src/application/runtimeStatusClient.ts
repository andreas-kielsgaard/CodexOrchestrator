export type RuntimeStatusTarget = 'app' | 'frontend' | 'backend';

export interface RuntimeStatusSnapshot {
  available: boolean;
  stale: boolean;
  staleTargets: RuntimeStatusTarget[];
  reason?: string;
  generation?: string;
  markedAt?: string;
  checkedAt: string;
}

export interface RuntimeStatusClient {
  checkStatus(): Promise<RuntimeStatusSnapshot>;
}

export function unavailableRuntimeStatus(checkedAt: string = new Date().toISOString()) {
  return {
    available: false,
    stale: false,
    staleTargets: [],
    checkedAt,
  } satisfies RuntimeStatusSnapshot;
}
