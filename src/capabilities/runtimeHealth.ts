import type { RuntimeStatusSnapshot } from '../application/queries/runtimeStatusClient';

export type {
  RuntimeStatusSnapshot,
  RuntimeStatusTarget,
} from '../application/queries/runtimeStatusClient';

export interface RuntimeHealthCapability {
  checkStatus(): Promise<RuntimeStatusSnapshot>;
  clearStale?(): Promise<RuntimeStatusSnapshot>;
}
