import { invoke } from '@tauri-apps/api/core';

export interface WorktreeCliBuild {
  readonly executable: string;
  readonly profile: string;
}

export const tauriWorktreeCliBuilds = {
  latest: (path: string) =>
    invoke<WorktreeCliBuild | null>('worktree_review_latest_cli_build', { path }),
  open: (path: string) => invoke<void>('worktree_review_open_cli_build', { path }),
};
