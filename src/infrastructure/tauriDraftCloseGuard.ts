import { getCurrentWindow } from '@tauri-apps/api/window';
import { isTauri } from '@tauri-apps/api/core';
import type { DraftCloseGuard } from '../application/draftCloseGuard';

export const tauriDraftCloseGuard: DraftCloseGuard = async (isDirty) => {
  if (!isTauri()) return () => {};
  return getCurrentWindow().onCloseRequested((event) => {
    if (
      isDirty() &&
      !window.confirm('Close and discard your unsaved Workflow and Capability Profile edits?')
    )
      event.preventDefault();
  });
};
