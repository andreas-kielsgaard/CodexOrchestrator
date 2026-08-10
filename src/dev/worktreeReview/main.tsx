import { createRoot } from 'react-dom/client';
import type {
  HumanReviewLauncherClient,
  HumanReviewSource,
} from '../../application/humanReviewLauncher';
import { HumanReviewLauncherView } from '../../features/humanReviewLauncher/HumanReviewLauncherView';
import '../../styles.css';

const sources: HumanReviewSource[] = [
  source('codex/explore-harness-inspector', 'archive', false, {
    ahead: 6,
    revision: '54bd91af30c2',
    equivalentPatches: 2,
  }),
  source('main', 'branch', true, { isMain: true, isCurrent: true, ahead: 0, revision: '7c9a2fd3041b' }),
  source('codex/current-work', 'branch', true, { ahead: 4, revision: 'c837eedb2109' }),
  source('origin/review-launcher', 'remote_branch', false, { ahead: 3, revision: 'b821d4991e72' }),
];

const client = {
  listSources: async () => sources,
  sourceHistory: async (sourceRef: string) => {
    const selected = sources.find((candidate) => candidate.sourceRef === sourceRef)!;
    return {
      branch: selected.branch!,
      sourceLabel: selected.label,
      revision: selected.revision,
      forkRevision: selected.forkRevision,
      commitCount: 0,
      truncated: false,
      commits: [],
      lineageMarkers: [],
    };
  },
  attachWorktree: async (sourceRef: string) => sources.find((source) => source.sourceRef === sourceRef)!,
  listInstances: async () => [],
  listProgress: async () => [],
  prepare: async () => Promise.reject(new Error('Not available in the visual harness.')),
  build: async () => Promise.reject(new Error('Not available in the visual harness.')),
  start: async () => Promise.reject(new Error('Not available in the visual harness.')),
} as unknown as HumanReviewLauncherClient;

createRoot(document.getElementById('root')!).render(<HumanReviewLauncherView client={client} />);

function source(
  branch: string,
  refKind: HumanReviewSource['refKind'],
  attached: boolean,
  overrides: Partial<HumanReviewSource> = {},
): HumanReviewSource {
  return {
    sourceRef: `fixture-${branch}`,
    label: branch,
    branch,
    detached: false,
    isMain: false,
    isCurrent: false,
    lineageAmbiguous: false,
    relationship: 'related',
    ahead: 2,
    behind: 0,
    forkRevision: '7c9a2fd3041b',
    revision: '54bd91af30c2',
    compatibility: attached ? 'compatible' : 'unavailable',
    compatibilityMessage: attached ? 'Compatible.' : 'Create a review worktree first.',
    attached,
    refKind,
    mergedDirectly: false,
    equivalentPatches: 0,
    comparisonBranch: 'main',
    ...overrides,
  };
}
