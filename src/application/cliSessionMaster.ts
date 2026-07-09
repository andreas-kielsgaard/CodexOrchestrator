import type { EntityId } from '../domain/model';
import {
  CLISessionDistributor,
  type CLISessionDistributorAcquireInput,
  type CLISessionLease,
} from './cliSessionDistributor';
import type { CLIInstanceSnapshot } from './cliInstanceHandler';

export class CLISessionMaster {
  private readonly activeLeases = new Map<EntityId, CLISessionLease>();

  constructor(private readonly distributor = new CLISessionDistributor()) {}

  acquire(input: CLISessionDistributorAcquireInput): CLISessionLease {
    const lease = this.distributor.acquire(input);
    this.activeLeases.set(lease.id, lease);
    return lease;
  }

  release(lease: CLISessionLease): void {
    this.activeLeases.delete(lease.id);
    this.distributor.release(lease);
  }

  async close(lease: CLISessionLease): Promise<void> {
    this.activeLeases.delete(lease.id);
    await this.distributor.close(lease);
  }

  async closeAll(): Promise<void> {
    const leases = [...this.activeLeases.values()];
    this.activeLeases.clear();
    await Promise.all(leases.map((lease) => this.distributor.close(lease)));
  }

  snapshots(): CLIInstanceSnapshot[] {
    return this.distributor.snapshots();
  }
}
