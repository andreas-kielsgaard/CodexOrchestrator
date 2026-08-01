export type WorktreeRuntimeEvidenceKind = 'observed' | 'projected' | 'recorded' | 'unsupported';

export interface WorktreeRuntimeMaterialBoundary {
  readonly material: string;
  readonly disposition: 'isolated' | 'shared-keyed' | 'unsupported';
  readonly detail: string;
  readonly evidence: WorktreeRuntimeEvidenceKind;
}

export interface WorktreeRuntimeLifecycleEvidence {
  readonly stage: string;
  readonly state: string;
  readonly detail: string;
  readonly evidence: WorktreeRuntimeEvidenceKind;
}

export interface WorktreeRuntimeExplorationSnapshot {
  readonly label: string;
  readonly notice: string;
  readonly checkedAt: string;
  readonly identity: {
    readonly instanceId: string;
    readonly sessionId: string;
    readonly worktreePath: string;
    readonly gitCommit: string;
    readonly sourceFingerprint: string;
    readonly tauriIdentifier: string;
  };
  readonly materials: readonly WorktreeRuntimeMaterialBoundary[];
  readonly lifecycle: readonly WorktreeRuntimeLifecycleEvidence[];
  readonly unsupported: readonly string[];
  readonly reviewPoints: readonly string[];
}

export interface WorktreeRuntimeExplorationSource {
  load(): Promise<WorktreeRuntimeExplorationSnapshot>;
}
