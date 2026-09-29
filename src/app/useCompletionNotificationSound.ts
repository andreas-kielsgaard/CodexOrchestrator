import { useEffect } from 'react';
import type {
  CompletionNotificationSource,
  CompletionSoundPlayer,
} from '../application/completionNotifications';
import { browserCompletionSound } from '../infrastructure/completionNotifications';

export function useCompletionNotificationSound(
  source: CompletionNotificationSource | undefined,
  sound: CompletionSoundPlayer = browserCompletionSound,
) {
  useEffect(() => {
    if (!source) return;
    let active = true;
    let stop: (() => void) | undefined;
    void source.subscribe(() => sound.play()).then((unsubscribe) => {
      if (active) stop = unsubscribe;
      else unsubscribe();
    });
    return () => {
      active = false;
      stop?.();
    };
  }, [sound, source]);

  useEffect(() => {
    if (!source) return;
    const unlock = () => sound.unlock();
    window.addEventListener('pointerdown', unlock, { once: true });
    window.addEventListener('keydown', unlock, { once: true });
    return () => {
      window.removeEventListener('pointerdown', unlock);
      window.removeEventListener('keydown', unlock);
    };
  }, [sound, source]);
}
