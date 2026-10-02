import { readCachedComposerDraft, writeCachedComposerDraft } from './composerDraftCache';

/** Editing is local to the composer; orchestration reads this store only on submission. */
export function createComposerDraftStore(key: string) {
  let text = readCachedComposerDraft(key)?.text ?? '';
  let directory = readCachedComposerDraft(key)?.workingDirectory ?? '';
  let dirty = false;
  let idle: ReturnType<typeof setTimeout> | undefined;
  let checkpoint: ReturnType<typeof setTimeout> | undefined;
  const listeners = new Set<() => void>();
  const flush = () => {
    clearTimeout(idle);
    clearTimeout(checkpoint);
    idle = checkpoint = undefined;
    if (!dirty) return;
    writeCachedComposerDraft(key, { text, workingDirectory: directory });
    dirty = false;
  };
  const schedule = () => {
    dirty = true;
    clearTimeout(idle);
    idle = setTimeout(flush, 500);
    checkpoint ??= setTimeout(flush, 15_000);
  };
  return {
    getSnapshot: () => text,
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    setText(value: string) {
      if (text === value) return;
      text = value;
      schedule();
      listeners.forEach((listener) => listener());
    },
    setDirectory(value: string) {
      if (directory === value) return;
      directory = value;
      schedule();
    },
    switchContext(next: string) {
      if (key === next) return;
      flush();
      key = next;
      const cached = readCachedComposerDraft(key);
      text = cached?.text ?? '';
      directory = cached?.workingDirectory ?? '';
      listeners.forEach((listener) => listener());
    },
    flush,
  };
}
export type ComposerDraftStore = ReturnType<typeof createComposerDraftStore>;
