import { invoke } from '@tauri-apps/api/core';
import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
  HumanReviewSource,
} from '../application/humanReviewLauncher';

export const tauriHumanReviewLauncher: HumanReviewLauncherClient = {
  listSources: () => invoke<HumanReviewSource[]>('list_human_review_worktrees'),
  listInstances: () => invoke<HumanReviewInstance[]>('list_human_review_instances'),
  prepare: (sourceRef, name) =>
    invoke('prepare_human_review_instance', { input: { sourceRef, name } }),
  build: action('build_human_review_instance'),
  start: action('start_human_review_instance'),
  status: action('status_human_review_instance'),
  focus: action('focus_human_review_instance'),
  stop: action('stop_human_review_instance'),
  recover: action('recover_human_review_instance'),
};

function action(command: string) {
  return (instanceRef: string) =>
    invoke<HumanReviewInstance>(command, { input: { instanceRef } });
}
