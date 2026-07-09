import type { EntityId } from '../domain/model';
import { CLIInstanceHandler, type CLIInstanceSnapshot } from './cliInstanceHandler';

export interface CLISessionDistributorAcquireInput {
  purpose: string;
  createHandler(): CLIInstanceHandler;
}

export interface CLISessionLease {
  id: EntityId;
  purpose: string;
  handler: CLIInstanceHandler;
}

interface CLISessionEntry {
  id: EntityId;
  purpose: string;
  handler: CLIInstanceHandler;
  leased: boolean;
}

export class CLISessionDistributor {
  private readonly sessions = new Map<EntityId, CLISessionEntry>();

  acquire(input: CLISessionDistributorAcquireInput): CLISessionLease {
    const freeEntry = [...this.sessions.values()].find(
      (entry) =>
        entry.purpose === input.purpose &&
        !entry.leased &&
        isReusableSnapshot(entry.handler.getSnapshot()),
    );

    const entry =
      freeEntry ??
      this.register({
        id: `cli-distributor-session-${crypto.randomUUID()}` as EntityId,
        purpose: input.purpose,
        handler: input.createHandler(),
        leased: false,
      });

    entry.leased = true;

    return {
      id: entry.id,
      purpose: entry.purpose,
      handler: entry.handler,
    };
  }

  release(lease: CLISessionLease): void {
    const entry = this.sessions.get(lease.id);

    if (entry) {
      entry.leased = false;
    }
  }

  async close(lease: CLISessionLease): Promise<void> {
    const entry = this.sessions.get(lease.id);

    if (!entry) {
      return;
    }

    await entry.handler.close('CLI session returned to distributor.');
    entry.leased = false;
  }

  snapshots(): CLIInstanceSnapshot[] {
    return [...this.sessions.values()].map((entry) => entry.handler.getSnapshot());
  }

  private register(entry: CLISessionEntry): CLISessionEntry {
    this.sessions.set(entry.id, entry);
    return entry;
  }
}

function isReusableSnapshot(snapshot: CLIInstanceSnapshot): boolean {
  return snapshot.status === 'idle' || snapshot.status === 'closed';
}
