import { useEffect, useRef } from 'react';

import type { DraftCloseGuard } from '../application/draftCloseGuard';

/** Navigation within the app retains drafts. Browser close/reload warns before losing them. */
export function useDraftCloseWarning(
  isDirty: () => boolean,
  registerNativeGuard?: DraftCloseGuard,
) {
  const dirty = useRef(isDirty);
  dirty.current = isDirty;
  useEffect(() => {
    let active = true;
    let stopNative: (() => void) | undefined;
    const warn = (event: BeforeUnloadEvent) => {
      if (!dirty.current()) return;
      event.preventDefault();
      event.returnValue = '';
    };
    window.addEventListener('beforeunload', warn);
    void registerNativeGuard?.(() => dirty.current())
      .then((stop) => {
        if (active) stopNative = stop;
        else stop();
      })
      .catch((error) => console.error('Could not register the unsaved draft close warning', error));
    return () => {
      active = false;
      stopNative?.();
      window.removeEventListener('beforeunload', warn);
    };
  }, [registerNativeGuard]);
}
