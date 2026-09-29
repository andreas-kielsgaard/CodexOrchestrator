import { act, fireEvent, renderHook, waitFor } from '@testing-library/react';
import type {
  CompletionNotification,
  CompletionNotificationSource,
  CompletionSoundPlayer,
} from '../application/completionNotifications';
import { useCompletionNotificationSound } from './useCompletionNotificationSound';

it('unlocks on user interaction and plays for live completion notifications', async () => {
  let notify: (notification: CompletionNotification) => void = () => undefined;
  const stop = vi.fn();
  const source: CompletionNotificationSource = {
    subscribe: vi.fn(async (listener) => {
      notify = listener;
      return stop;
    }),
  };
  const sound: CompletionSoundPlayer = { unlock: vi.fn(), play: vi.fn() };
  const { unmount } = renderHook(() => useCompletionNotificationSound(source, sound));
  await waitFor(() => expect(source.subscribe).toHaveBeenCalledOnce());

  fireEvent.pointerDown(window);
  act(() => notify({ kind: 'agent_turn', id: 'turn-one' }));

  expect(sound.unlock).toHaveBeenCalledOnce();
  expect(sound.play).toHaveBeenCalledOnce();
  unmount();
  expect(stop).toHaveBeenCalledOnce();
});
