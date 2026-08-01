import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
  HumanReviewOperationProgress,
  HumanReviewProofPresentation,
} from '../../application/humanReviewLauncher';
import type { WorktreeBuildDetail } from '../../application/worktreeBuild';
import { HumanReviewLauncherView } from './HumanReviewLauncherView';

const launcherCss = readFileSync(
  'src/features/humanReviewLauncher/humanReviewLauncher.css',
  'utf8',
);

describe('HumanReviewLauncherView', () => {
  it('keeps the typed progress ledger shrink-safe at the standard review width', () => {
    expect(launcherCss).toMatch(/\.human-review\s*{[^}]*max-width:\s*100vw;/s);
    expect(launcherCss).toMatch(/\.human-review\s*{[^}]*overflow-x:\s*hidden;/s);
    expect(launcherCss).toMatch(/\.human-review\s*>\s*\*\s*{[^}]*max-width:\s*100%;/s);
    expect(launcherCss).toMatch(/\.human-review\s*>\s*\*\s*{[^}]*min-width:\s*0;/s);
    expect(launcherCss).toMatch(/\.human-review__progress\s*{[^}]*min-width:\s*0;/s);
    expect(launcherCss).toMatch(/\.human-review__progress\s*{[^}]*overflow:\s*hidden;/s);
    expect(launcherCss).toMatch(
      /@media\s*\(max-width:\s*1400px\)[\s\S]*?\.human-review__progress dl\s*{[^}]*grid-template-columns:\s*repeat\(2, minmax\(0, 1fr\)\);/,
    );
    expect(launcherCss).toMatch(/\.human-review__progress dd\s*{[^}]*overflow-wrap:\s*anywhere;/s);
  });

  it('prepares and opens a named instance through semantic controls without infrastructure details', async () => {
    const client = new FakeClient();
    render(<HumanReviewLauncherView client={client} />);

    await screen.findByRole('option', { name: /codex\/feature/ });
    fireEvent.change(screen.getByLabelText('Window name'), {
      target: { value: 'Checkout review' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Prepare' }));

    const card = await screen.findByRole('heading', { name: 'Checkout review' });
    const review = card.closest('article');
    expect(review).not.toBeNull();
    expect(review).not.toHaveTextContent('C:\\repos');
    expect(review).not.toHaveTextContent('18200');
    fireEvent.click(within(review as HTMLElement).getByRole('button', { name: 'Build' }));
    await waitFor(() =>
      expect(within(review as HTMLElement).getByText('passed')).toBeInTheDocument(),
    );
    fireEvent.click(within(review as HTMLElement).getByRole('button', { name: 'Open' }));
    await waitFor(() =>
      expect(within(review as HTMLElement).getByText('running')).toBeInTheDocument(),
    );
    expect(
      within(review as HTMLElement).getByRole('button', { name: 'Focus window' }),
    ).toBeEnabled();

    client.interrupt();
    fireEvent.click(within(review as HTMLElement).getByRole('button', { name: 'Check status' }));
    await waitFor(() =>
      expect(within(review as HTMLElement).getByText('closed')).toBeInTheDocument(),
    );
    const recover = within(review as HTMLElement).getByRole('button', { name: 'Recover' });
    expect(recover).toBeEnabled();
    fireEvent.click(recover);
    await waitFor(() =>
      expect(within(review as HTMLElement).getByText('recovered')).toBeInTheDocument(),
    );
  });

  it('renders typed long-build progress and quiet evidence without calling it stalled', async () => {
    const client = new LongBuildClient();
    render(<HumanReviewLauncherView client={client} />);
    const card = (await screen.findByRole('heading', { name: 'Long build' })).closest('article')!;
    fireEvent.click(within(card).getByRole('button', { name: 'Build' }));
    expect(await within(card).findByText('Checking TypeScript')).toBeVisible();
    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 550));
    });
    expect(within(card).getByText('Building the application interface')).toBeVisible();
    expect(within(card).getByText(/No new evidence/)).toBeVisible();
    expect(within(card).queryByText(/stalled/i)).toBeNull();
    expect(within(card).getByText('Compiling application crate')).toBeVisible();
    client.finish();
    await waitFor(() => expect(within(card).getByText('passed')).toBeVisible());
  });

  it('reflects an application-owned background operation without button-history inference', async () => {
    const client = new FakeClient();
    client.listProgress = async () => [
      {
        operationRef: 'review-operation-background-proof',
        operation: 'start',
        state: 'pending',
        stage: 'waiting-for-window',
        stageLabel: 'Waiting for a usable worktree-build window',
        activity: 'working',
        elapsedMs: 12_000,
        evidenceAgeMs: 200,
        recentOutput: ['Owned services are ready; waiting for the review window.'],
        condition: 'Waiting for the exact owned application surface.',
        expectedWait: 'Normally under a minute.',
        actionRequired: false,
        actionGuidance: 'No action is required.',
        reusableSummary: 'The verified build remains reusable.',
        missingReadinessFact: 'A rendered application readiness marker.',
      },
    ];
    render(<HumanReviewLauncherView client={client} />);

    const operation = await screen.findByRole('region', {
      name: 'Current application-owned review operation',
    });
    expect(within(operation).getByText('Waiting for a usable worktree-build window')).toBeVisible();
    expect(within(operation).getByText(/Owned services are ready/)).toBeVisible();
  });

  it('explains multiple retained instances and opens the shared build detail with full output', async () => {
    const client = new FakeClient();
    const alpha = instance('Alpha build', 'stopped', 'passed');
    const beta = { ...instance('Beta build', 'prepared', 'not-built'), instanceRef: 'beta' };
    const historical = {
      ...instance('Historical build', 'stopped', 'superseded'),
      instanceRef: 'historical',
      currentUse: 'Source changed since this build',
      actionRequired: true,
      actionSummary: 'Prepare a fresh instance for the selected worktree.',
    };
    client.listInstances = async () => [alpha, beta, historical];
    client.detail = async (instanceRef) => detail(instanceRef === 'beta' ? beta : alpha);
    render(<HumanReviewLauncherView client={client} />);

    expect(await screen.findByRole('heading', { name: 'Alpha build' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Beta build' })).toBeVisible();
    expect(screen.getAllByText(/Stop closes the owned process tree/)).toHaveLength(3);
    const historicalCard = screen
      .getByRole('heading', { name: 'Historical build' })
      .closest('article')!;
    expect(within(historicalCard).getByText('Source changed since this build')).toBeVisible();
    expect(within(historicalCard).getByRole('button', { name: 'Build' })).toBeDisabled();
    expect(within(historicalCard).getByRole('button', { name: 'Open' })).toBeDisabled();
    const alphaCard = screen.getByRole('heading', { name: 'Alpha build' }).closest('article')!;
    fireEvent.click(within(alphaCard).getByRole('button', { name: 'Build details' }));
    const buildDetail = await screen.findByRole('main', { name: 'Worktree build details' });
    expect(buildDetail).toHaveTextContent('Why it exists');
    expect(buildDetail).toHaveTextContent('safe output 24');
    expect(buildDetail).toHaveTextContent('Retained until deliberate cleanup');
    fireEvent.click(within(buildDetail).getByRole('button', { name: 'Review files and changes' }));
    expect(await screen.findByRole('main', { name: 'Files and diffs' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Build details' }));
    expect(await screen.findByRole('main', { name: 'Worktree build details' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Back' }));
    expect(await screen.findByRole('heading', { name: 'Retained review builds' })).toBeVisible();
  });

  it('uses typed proof presentation for legacy selection and retained full-output drill-down', async () => {
    const client = new FakeClient();
    const legacy = {
      ...instance('Legacy build', 'prepared', 'not-built'),
      compatibility: 'incompatible' as const,
      actionRequired: true,
      actionSummary: 'Update to a compatible worktree lineage before Build or Open.',
    };
    client.listSources = async () => [
      {
        sourceRef: 'compatible-source',
        label: 'codex/compatible',
        revision: 'abcdef012345',
        compatibility: 'compatible' as const,
        compatibilityMessage: 'Compatible.',
      },
      {
        sourceRef: 'legacy-source',
        label: 'legacy branch',
        revision: '123456789abc',
        compatibility: 'incompatible' as const,
        compatibilityMessage:
          'This branch predates the Worktree Review child contract. Update it before Build or Open.',
      },
    ];
    client.listInstances = async () => [legacy];
    client.detail = async () => detail(legacy);
    let presentation: HumanReviewProofPresentation = {
      route: 'overview',
      origin: 'selected-worktree',
      sourceRef: 'legacy-source',
      sequence: '0123456789abcdef0123456789abcdef',
    };
    client.proofPresentation = vi.fn(async () => presentation);
    render(<HumanReviewLauncherView client={client} />);

    expect(await screen.findByText(/predates the Worktree Review child contract/)).toBeVisible();
    const card = screen.getByRole('heading', { name: 'Legacy build' }).closest('article')!;
    expect(within(card).getByRole('button', { name: 'Build' })).toBeDisabled();
    expect(within(card).getByRole('button', { name: 'Open' })).toBeDisabled();

    presentation = {
      route: 'details',
      origin: 'retained-operation-output',
      instanceRef: legacy.instanceRef,
      operationRef: 'operation-build-fixture',
      sequence: '1123456789abcdef0123456789abcdef',
    };
    const detailView = await screen.findByRole('main', { name: 'Worktree build details' });
    expect(within(detailView).getByText(/24 safe lines/)).toBeVisible();
    expect(detailView).toHaveTextContent('safe output 24');
    expect(
      within(detailView)
        .getByText(/build .* Finished .* succeeded/)
        .closest('details'),
    ).toHaveAttribute('open');
  });
});

class LongBuildClient implements HumanReviewLauncherClient {
  private progressCalls = 0;
  private resolveBuild!: (value: HumanReviewInstance) => void;
  private instance = instance('Long build', 'prepared', 'not-built');
  listSources = async () => [
    {
      sourceRef: 'opaque',
      label: 'codex/long',
      revision: 'abcdef012345',
      compatibility: 'compatible' as const,
      compatibilityMessage: 'Compatible.',
    },
  ];
  listInstances = async () => [this.instance];
  prepare = async () => this.instance;
  build = async () =>
    new Promise<HumanReviewInstance>((resolve) => {
      this.resolveBuild = resolve;
    });
  start = async () => this.instance;
  status = async () => this.instance;
  focus = async () => this.instance;
  stop = async () => this.instance;
  recover = async () => this.instance;
  listProgress = async (): Promise<readonly HumanReviewOperationProgress[]> => [];
  progress = async (operationRef: string) => {
    const first = this.progressCalls++ === 0;
    return {
      operationRef,
      operation: 'build' as const,
      state: 'pending' as const,
      stage: first ? 'typecheck' : 'frontend-build',
      stageLabel: first ? 'Checking TypeScript' : 'Building the application interface',
      activity: first ? ('working' as const) : ('quiet' as const),
      elapsedMs: first ? 3_000 : 25_000,
      evidenceAgeMs: first ? 100 : 21_000,
      recentOutput: first ? ['Type checking application'] : ['Compiling application crate'],
      condition: first ? 'Checking source.' : 'Compiling source.',
      expectedWait: first ? 'Usually seconds.' : 'Cold builds take several minutes.',
      actionRequired: false,
      actionGuidance: 'No action is required.',
      reusableSummary: 'Prepared isolation remains reusable.',
    };
  };
  detail = async () => detail(this.instance);
  comparison = () => comparison;
  finish() {
    this.instance = instance('Long build', 'prepared', 'passed');
    this.resolveBuild(this.instance);
  }
}

class FakeClient implements HumanReviewLauncherClient {
  private instance: HumanReviewInstance | undefined;
  proofPresentation?: HumanReviewLauncherClient['proofPresentation'];
  listSources: HumanReviewLauncherClient['listSources'] = async () => [
    {
      sourceRef: 'opaque',
      label: 'codex/feature - review',
      revision: 'abcdef012345',
      compatibility: 'compatible' as const,
      compatibilityMessage: 'Compatible.',
    },
  ];
  listInstances = async () => (this.instance ? [this.instance] : []);
  prepare = async (_operationRef: string, _sourceRef: string, name: string) =>
    this.set(name, 'prepared', 'not-built');
  build = async () => this.set(this.instance!.name, 'prepared', 'passed');
  start = async () => this.set(this.instance!.name, 'running', 'passed');
  progress = async (operationRef: string) => ({
    operationRef,
    operation: operationRef.startsWith('prepare')
      ? ('prepare' as const)
      : operationRef.startsWith('start')
        ? ('start' as const)
        : ('build' as const),
    state: 'succeeded' as const,
    stage: 'complete',
    stageLabel: 'Finished',
    activity: 'finished' as const,
    elapsedMs: 1_000,
    evidenceAgeMs: 0,
    recentOutput: ['Safe progress'],
    condition: 'The operation completed.',
    expectedWait: 'No waiting required.',
    actionRequired: false,
    actionGuidance: 'Continue when ready.',
    reusableSummary: 'Private outputs remain reusable.',
  });
  status = async () => this.instance!;
  focus = async () => this.instance!;
  stop = async () => this.set(this.instance!.name, 'stopped', 'passed');
  recover = async () => this.set(this.instance!.name, 'recovered', 'passed');
  listProgress = async (): Promise<readonly HumanReviewOperationProgress[]> => [];
  detail = async (instanceRef: string) => detail({ ...this.instance!, instanceRef });
  comparison = () => comparison;

  interrupt() {
    this.instance = { ...this.instance!, health: 'closed', canFocus: false };
  }

  private set(name: string, phase: string, build: HumanReviewInstance['build']) {
    this.instance = instance(name, phase, build);
    return this.instance;
  }
}

function instance(
  name: string,
  phase: string,
  build: HumanReviewInstance['build'],
): HumanReviewInstance {
  return {
    instanceRef: 'opaque-instance',
    name,
    sourceLabel: 'codex/feature - review',
    phase,
    health: phase === 'running' ? 'healthy' : 'unknown',
    stale: false,
    build,
    canFocus: phase === 'running',
    purpose: 'A retained isolated build for human review.',
    currentUse: phase === 'running' ? 'Human review window open' : 'Prepared, not running',
    retention: 'Retained',
    cleanup: 'Stop closes the owned process tree; cleanup is manual.',
    actionRequired: false,
    actionSummary: build === 'passed' ? 'Open the verified build.' : 'Build before Open.',
    compatibility: 'compatible',
  };
}

const comparison = {
  load: async () => ({ files: [] }),
};

function detail(value: HumanReviewInstance): WorktreeBuildDetail {
  return {
    instanceRef: value.instanceRef,
    name: value.name,
    sourceLabel: value.sourceLabel,
    purpose: value.purpose,
    phase: value.phase,
    health: value.health,
    stale: value.stale,
    build: value.build,
    compatibility: value.compatibility,
    compatibilityMessage: 'Compatible.',
    orientation: 'One retained source identity and its owned lifecycle.',
    prepareProduced: 'Prepare reserved isolated state.',
    buildProduced: 'Build produced private artifacts.',
    openProduced: 'Open creates an exact owned window.',
    currentCondition: value.currentUse,
    actionRequired: value.actionRequired,
    actionSummary: value.actionSummary,
    reusableSummary: 'Private outputs remain reusable only for the exact identity.',
    retention: {
      policy: 'Retained until deliberate cleanup',
      cleanup: 'Stop is process-only; cleanup is manual.',
      automatic: false,
      actionRequired: false,
    },
    artifacts: [
      {
        label: 'Private application executable',
        state: value.build === 'passed' ? 'available' : 'not-produced',
        summary: 'Private to this instance.',
      },
    ],
    lifecycleHistory: [
      { occurredAtMs: 1_750_000_000_000, kind: 'Prepared', summary: 'Reserved isolation.' },
    ],
    operations: [
      {
        operationRef: 'operation-build-fixture',
        operation: 'build',
        state: 'succeeded',
        stageLabel: 'Finished',
        startedAtMs: 1_750_000_000_000,
        updatedAtMs: 1_750_000_001_000,
        output: Array.from({ length: 24 }, (_, index) => `safe output ${index + 1}`),
        outputComplete: true,
      },
    ],
    context: {
      name: value.name,
      branch: 'codex/feature',
      detached: false,
      head: {
        id: 'abcdef0123456789',
        abbreviatedId: 'abcdef0',
        message: 'Feature',
        committedAt: '2026-07-30T10:00:00Z',
      },
      dirty: { dirty: true, staged: 1, unstaged: 1, untracked: 1 },
      main: {
        branch: 'main',
        detached: false,
        head: {
          id: '1234567890abcdef',
          abbreviatedId: '1234567',
          message: 'Main',
          committedAt: '2026-07-29T10:00:00Z',
        },
        dirty: { dirty: false, staged: 0, unstaged: 0, untracked: 0 },
      },
      relationship: { ahead: 2, behind: 1, mergeBase: '1111111', summary: '2 ahead, 1 behind' },
      relatedBranches: [
        {
          name: 'codex/parent',
          ahead: 1,
          behind: 0,
          mergeBase: '2222222',
          summary: '1 ahead, 0 behind this local branch',
        },
      ],
      history: [],
      comparisonBasis: 'Machine main HEAD to complete selected state.',
    },
  };
}
