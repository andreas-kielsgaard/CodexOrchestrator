import { invoke } from '@tauri-apps/api/core';
import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
  HumanReviewSource,
} from '../application/humanReviewLauncher';
import { assertCompleteFileReviewFile, type FileReviewSnapshot } from '../application/fileReview';
import type { WorktreeBuildDetail } from '../application/worktreeBuild';

export const tauriHumanReviewLauncher: HumanReviewLauncherClient = {
  listSources: () => invoke<HumanReviewSource[]>('list_human_review_worktrees'),
  listInstances: () => invoke<HumanReviewInstance[]>('list_human_review_instances'),
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
  proofNavigation: () => invoke('human_review_launcher_proof_navigation'),
  proofDetailNavigation: () => invoke('human_review_launcher_detail_navigation'),
  proofPresentation: () => invoke('human_review_launcher_proof_presentation'),
  status: action('status_human_review_instance'),
  focus: action('focus_human_review_instance'),
  stop: action('stop_human_review_instance'),
  recover: action('recover_human_review_instance'),
};

function action(command: string) {
  return (instanceRef: string) => invoke<HumanReviewInstance>(command, { input: { instanceRef } });
}
