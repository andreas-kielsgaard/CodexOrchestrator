import type { ModelAllowanceDto } from '../../application/executionConfiguration';

const allowanceKey = (profileKey: string, routeId: string, modelId: string) =>
  JSON.stringify([profileKey, routeId, modelId]);

/** Process-lifetime editor memory that never becomes persisted Capability Profile state. */
export class CapabilityProfileEditorMemory {
  private removedAllowances = new Map<string, ModelAllowanceDto>();

  remember(profileKey: string, routeId: string, allowance: ModelAllowanceDto): void {
    this.removedAllowances.set(allowanceKey(profileKey, routeId, allowance.modelId), allowance);
  }

  recall(profileKey: string, routeId: string, modelId: string): ModelAllowanceDto | undefined {
    return this.removedAllowances.get(allowanceKey(profileKey, routeId, modelId));
  }

  rekey(from: string, to: string): void {
    if (from === to) return;
    for (const [key, allowance] of this.removedAllowances) {
      const [profileKey, routeId, modelId] = JSON.parse(key) as [string, string, string];
      if (profileKey !== from) continue;
      this.removedAllowances.delete(key);
      this.removedAllowances.set(allowanceKey(to, routeId, modelId), allowance);
    }
  }

  clear(profileKey: string): void {
    for (const key of this.removedAllowances.keys()) {
      const [candidate] = JSON.parse(key) as [string, string, string];
      if (candidate === profileKey) this.removedAllowances.delete(key);
    }
  }
}
