import type { SessionGitAction } from './organization';
export const gitActionLabels: Record<SessionGitAction, string> = {
  create: 'Create branch',
  rename: 'Rename branch',
  delete: 'Delete merged branch',
  fetch: 'Fetch remotes',
  merge: 'Fast-forward merge',
};
