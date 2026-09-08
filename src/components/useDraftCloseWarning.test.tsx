import { renderHook } from '@testing-library/react';
import { useDraftCloseWarning } from './useDraftCloseWarning';

it('keeps the close guard current without registering it on every edit', async () => {
  let check!: () => boolean;
  const stop = vi.fn();
  const register = vi.fn(async (isDirty: () => boolean) => {
    check = isDirty;
    return stop;
  });
  const { rerender, unmount } = renderHook(
    ({ dirty }) => useDraftCloseWarning(() => dirty, register),
    { initialProps: { dirty: false } },
  );
  expect(check()).toBe(false);
  rerender({ dirty: true });
  expect(check()).toBe(true);
  expect(register).toHaveBeenCalledOnce();
  const event = new Event('beforeunload', { cancelable: true });
  window.dispatchEvent(event);
  expect(event.defaultPrevented).toBe(true);
  await Promise.resolve();
  unmount();
  expect(stop).toHaveBeenCalledOnce();
});
