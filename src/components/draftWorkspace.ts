/** In-app editing state. The composition root owns its lifetime; persistence stays in clients. */
export class DraftWorkspace<T> {
  selectedKey: string | null = null;
  private entries = new Map<string, { saved: T; working: T; past: T[]; future: T[] }>();

  load(key: string, saved: T): T {
    const existing = this.entries.get(key);
    if (existing && this.dirty(key)) return existing.working;
    this.entries.set(key, { saved, working: saved, past: [], future: [] });
    return saved;
  }

  read(key: string): T | undefined {
    return this.entries.get(key)?.working;
  }
  baseline(key: string): T | undefined {
    return this.entries.get(key)?.saved;
  }

  edit(key: string, working: T): void {
    const entry = this.entries.get(key);
    if (entry) {
      if (entry.working === working) return;
      entry.past = [...entry.past.slice(-49), entry.working];
      entry.future = [];
      entry.working = working;
    } else this.entries.set(key, { saved: working, working, past: [], future: [] });
  }

  acceptSave(key: string, submitted: T, saved: T, updateRevision: (working: T, saved: T) => T): T {
    const current = this.entries.get(key)?.working ?? submitted;
    const working = current === submitted ? saved : updateRevision(current, saved);
    const entry = this.entries.get(key);
    this.entries.set(key, { saved, working, past: entry?.past ?? [], future: entry?.future ?? [] });
    return working;
  }

  dirty(key?: string): boolean {
    const entries = key ? [this.entries.get(key)] : [...this.entries.values()];
    return entries.some(
      (entry) => entry && JSON.stringify(entry.saved) !== JSON.stringify(entry.working),
    );
  }

  discard(key: string): void {
    this.entries.delete(key);
  }

  canUndo(key: string): boolean {
    return Boolean(this.entries.get(key)?.past.length);
  }
  canRedo(key: string): boolean {
    return Boolean(this.entries.get(key)?.future.length);
  }

  restore(
    key: string,
    direction: 'undo' | 'redo',
    retainRevision: (value: T, saved: T) => T,
  ): T | undefined {
    const entry = this.entries.get(key);
    if (!entry) return undefined;
    const from = direction === 'undo' ? entry.past : entry.future;
    const to = direction === 'undo' ? entry.future : entry.past;
    const previous = from.pop();
    if (previous === undefined) return entry.working;
    to.push(entry.working);
    entry.working = retainRevision(previous, entry.saved);
    return entry.working;
  }
}
