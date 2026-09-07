/** Registers a window-close check. The editor remains the owner of its dirty state. */
export type DraftCloseGuard = (isDirty: () => boolean) => Promise<() => void>;
