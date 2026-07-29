import { invoke } from '@tauri-apps/api/core';
import { assertCompleteFileReviewFile, type FileReviewSnapshot } from '../application/fileReview';
import type { WorktreeBuildClient, WorktreeBuildContext } from '../application/worktreeBuild';

export const tauriWorktreeBuild: WorktreeBuildClient = {
  context: () => invoke<WorktreeBuildContext>('worktree_build_context'),
  comparison: {
    async load() {
      const snapshot = await invoke<FileReviewSnapshot>('worktree_build_comparison');
      snapshot.files.forEach(assertCompleteFileReviewFile);
      return snapshot;
    },
  },
  markReady: () => invoke('mark_worktree_build_ready'),
  proofNavigation: () => invoke('worktree_review_proof_navigation'),
};
