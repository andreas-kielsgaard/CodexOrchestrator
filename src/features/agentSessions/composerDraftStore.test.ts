import { createComposerDraftStore } from './composerDraftStore';
import { readCachedComposerDraft, writeCachedComposerDraft } from './composerDraftCache';

beforeEach(() => {
  localStorage.clear();
  vi.useFakeTimers();
});
afterEach(() => vi.useRealTimers());
it('waits for 500ms idle and checkpoints continuous typing at 15 seconds', () => {
  const store = createComposerDraftStore('session:idle');
  store.setText('a');
  vi.advanceTimersByTime(499);
  expect(readCachedComposerDraft('session:idle')).toBeNull();
  vi.advanceTimersByTime(1);
  expect(readCachedComposerDraft('session:idle')?.text).toBe('a');
  for (let i = 0; i < 38; i++) {
    store.setText(`typing ${i}`);
    vi.advanceTimersByTime(400);
  }
  expect(readCachedComposerDraft('session:idle')?.text).toBe('typing 37');
  store.flush();
});
it('flushes before navigation and hydrates only the selected conversation', () => {
  writeCachedComposerDraft('session:two', { text: 'second' });
  const store = createComposerDraftStore('session:one');
  const listener = vi.fn();
  store.subscribe(listener);
  store.setText('first');
  store.switchContext('session:two');
  expect(readCachedComposerDraft('session:one')?.text).toBe('first');
  expect(store.getSnapshot()).toBe('second');
  expect(listener).toHaveBeenCalledTimes(2);
});
