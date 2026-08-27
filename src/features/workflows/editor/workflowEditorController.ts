import type {
  WorkflowConnectionConfig,
  WorkflowElementRef,
  WorkflowNodeConfig,
} from '../../../application/workflows';

export type WorkflowEditableConfig = WorkflowNodeConfig | WorkflowConnectionConfig;

export interface WorkflowEditorElementSnapshot {
  readonly target: WorkflowElementRef;
  readonly value: WorkflowEditableConfig | null;
}

export interface WorkflowEditorControllerPorts {
  readElement(target: WorkflowElementRef): WorkflowEditableConfig | null;
  isPersisted(target: WorkflowElementRef): boolean;
  dependentConnections(nodeId: string): readonly WorkflowConnectionConfig[];
  saveElement(
    target: WorkflowElementRef,
    value: WorkflowEditableConfig,
    options?: { readonly blocking?: boolean },
  ): void;
  removeLocalElement(target: WorkflowElementRef): void;
  deletePersistedElement(target: WorkflowElementRef): Promise<void>;
}

interface WorkflowEditorHistoryEntry {
  readonly before: readonly WorkflowEditorElementSnapshot[];
  readonly after: readonly WorkflowEditorElementSnapshot[];
}

interface FieldTransaction {
  readonly target: WorkflowElementRef;
  readonly before: WorkflowEditableConfig | null;
}

export class WorkflowEditorController {
  private ports: WorkflowEditorControllerPorts | null = null;
  private undoEntries: WorkflowEditorHistoryEntry[] = [];
  private redoEntries: WorkflowEditorHistoryEntry[] = [];
  private fieldTransaction: FieldTransaction | null = null;
  private replaying = false;
  private busy = false;

  configure(ports: WorkflowEditorControllerPorts): void {
    this.ports = ports;
  }

  reset(): void {
    this.undoEntries = [];
    this.redoEntries = [];
    this.fieldTransaction = null;
    this.replaying = false;
    this.busy = false;
  }

  beginFieldEdit(target: WorkflowElementRef): void {
    const ports = this.requirePorts();
    if (this.fieldTransaction && sameTarget(this.fieldTransaction.target, target)) return;
    this.commitFieldEdit();
    this.fieldTransaction = { target, before: cloneConfig(ports.readElement(target)) };
  }

  commitFieldEdit(target?: WorkflowElementRef): void {
    const transaction = this.fieldTransaction;
    if (!transaction || (target && !sameTarget(transaction.target, target))) return;
    this.fieldTransaction = null;
    const after = cloneConfig(this.requirePorts().readElement(transaction.target));
    this.record({
      before: [{ target: transaction.target, value: transaction.before }],
      after: [{ target: transaction.target, value: after }],
    });
  }

  changeElement(
    target: WorkflowElementRef,
    value: WorkflowEditableConfig,
    options: { readonly blocking?: boolean } = {},
  ): void {
    const ports = this.requirePorts();
    const before = cloneConfig(ports.readElement(target));
    ports.saveElement(target, value, options);
    if (this.fieldTransaction && sameTarget(this.fieldTransaction.target, target)) return;
    this.record({
      before: [{ target, value: before }],
      after: [{ target, value: cloneConfig(value) }],
    });
  }

  async deleteElement(target: WorkflowElementRef): Promise<void> {
    if (this.busy) return;
    this.commitFieldEdit(target);
    const ports = this.requirePorts();
    const targets =
      target.kind === 'node'
        ? [
            ...ports
              .dependentConnections(target.id)
              .map((connection): WorkflowElementRef => ({ kind: 'connection', id: connection.id })),
            target,
          ]
        : [target];
    const before = targets.map((candidate) => ({
      target: candidate,
      value: cloneConfig(ports.readElement(candidate)),
    }));
    if (before.every((snapshot) => snapshot.value === null)) return;

    this.busy = true;
    try {
      for (const candidate of targets) await this.removeElement(candidate);
      this.record({
        before,
        after: targets.map((candidate) => ({ target: candidate, value: null })),
      });
    } finally {
      this.busy = false;
    }
  }

  async undo(): Promise<void> {
    if (this.busy) return;
    this.commitFieldEdit();
    const entry = this.undoEntries.pop();
    if (!entry) return;
    this.busy = true;
    try {
      await this.apply(entry.before);
      this.redoEntries.push(entry);
    } catch (error) {
      this.undoEntries.push(entry);
      throw error;
    } finally {
      this.busy = false;
    }
  }

  async redo(): Promise<void> {
    if (this.busy) return;
    this.commitFieldEdit();
    const entry = this.redoEntries.pop();
    if (!entry) return;
    this.busy = true;
    try {
      await this.apply(entry.after);
      this.undoEntries.push(entry);
    } catch (error) {
      this.redoEntries.push(entry);
      throw error;
    } finally {
      this.busy = false;
    }
  }

  private record(entry: WorkflowEditorHistoryEntry): void {
    if (this.replaying || snapshotsEqual(entry.before, entry.after)) return;
    this.undoEntries.push(entry);
    this.redoEntries = [];
  }

  private async apply(snapshots: readonly WorkflowEditorElementSnapshot[]): Promise<void> {
    const removals = snapshots.filter((snapshot) => snapshot.value === null);
    const restorations = snapshots.filter((snapshot) => snapshot.value !== null);
    this.replaying = true;
    try {
      for (const snapshot of removals.filter(({ target }) => target.kind === 'connection'))
        await this.removeElement(snapshot.target);
      for (const snapshot of removals.filter(({ target }) => target.kind === 'node'))
        await this.removeElement(snapshot.target);
      for (const snapshot of restorations.filter(({ target }) => target.kind === 'node'))
        this.requirePorts().saveElement(snapshot.target, cloneConfig(snapshot.value)!);
      for (const snapshot of restorations.filter(({ target }) => target.kind === 'connection'))
        this.requirePorts().saveElement(snapshot.target, cloneConfig(snapshot.value)!);
    } finally {
      this.replaying = false;
    }
  }

  private async removeElement(target: WorkflowElementRef): Promise<void> {
    const ports = this.requirePorts();
    if (ports.readElement(target) === null) return;
    if (ports.isPersisted(target)) await ports.deletePersistedElement(target);
    else ports.removeLocalElement(target);
  }

  private requirePorts(): WorkflowEditorControllerPorts {
    if (!this.ports) throw new Error('Workflow editor controller is not configured.');
    return this.ports;
  }
}

function sameTarget(left: WorkflowElementRef, right: WorkflowElementRef): boolean {
  return left.kind === right.kind && left.id === right.id;
}

function cloneConfig<T extends WorkflowEditableConfig | null>(value: T): T {
  return value === null ? value : (structuredClone(value) as T);
}

function snapshotsEqual(
  left: readonly WorkflowEditorElementSnapshot[],
  right: readonly WorkflowEditorElementSnapshot[],
): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}
