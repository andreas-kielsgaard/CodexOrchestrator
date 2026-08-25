import { invoke } from '@tauri-apps/api/core';
import type {
  WorktreeReviewClient,
  WorktreeReviewInstance,
  WorktreeReviewReadiness,
  WorktreeReviewSettings,
  WorktreeReviewSource,
  WorktreeReviewSourceHistory,
} from '../application/worktreeReview';
import { assertCompleteFileReviewFile, type FileReviewSnapshot } from '../application/fileReview';
import type { WorktreeBuildDetail } from '../application/worktreeBuild';

export const tauriWorktreeReviewClient: WorktreeReviewClient = {
  readiness: () => invoke<WorktreeReviewReadiness>('worktree_review_readiness'),
  selectRepository: (repositoryRoot) =>
    invoke<WorktreeReviewReadiness>('select_worktree_review_repository', {
      input: { repositoryRoot },
    }),
  listSources: (input = {}) =>
    invoke<WorktreeReviewSource[]>('list_human_review_worktrees', { input }),
  listRepositoryHistory: () =>
    invoke<WorktreeReviewSource[]>('list_human_review_repository_history'),
  sourceHistory: (sourceRef) =>
    invoke<WorktreeReviewSourceHistory>('human_review_source_history', { input: { sourceRef } }),
  attachWorktree: (sourceRef) =>
    invoke<WorktreeReviewSource>('attach_human_review_worktree', { input: { sourceRef } }),
  listInstances: () => invoke<WorktreeReviewInstance[]>('list_human_review_instances'),
  settings: () => invoke<WorktreeReviewSettings>('human_review_settings'),
  updateSettings: (input) =>
    invoke<WorktreeReviewSettings>('update_human_review_settings', { input }),
  prepare: (operationRef, sourceRef, name) =>
    invoke('prepare_human_review_instance', { input: { operationRef, sourceRef, name } }),
  build: (operationRef, instanceRef) =>
    invoke('build_human_review_instance', { input: { operationRef, instanceRef } }),
  start: (operationRef, instanceRef) =>
    invoke('start_human_review_instance', { input: { operationRef, instanceRef } }),
  progress: (operationRef) =>
    invoke('human_review_operation_progress', { input: { operationRef } }),
  listProgress: () => invoke('list_human_review_operation_progress'),
  detail: (instanceRef) =>
    invoke<WorktreeBuildDetail>('human_review_instance_detail', { input: { instanceRef } }),
  comparison: (instanceRef) => ({
    async load() {
      const snapshot = await invoke<FileReviewSnapshot>('human_review_instance_comparison', {
        input: { instanceRef },
      });
      snapshot.files.forEach(assertCompleteFileReviewFile);
      return snapshot;
    },
  }),
  status: instanceAction('status_human_review_instance'),
  focus: instanceAction('focus_human_review_instance'),
  stop: instanceAction('stop_human_review_instance'),
  recover: instanceAction('recover_human_review_instance'),
};

function instanceAction(command: string) {
  return (instanceRef: string) =>
    invoke<WorktreeReviewInstance>(command, { input: { instanceRef } });
}
