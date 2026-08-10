import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
  HumanReviewOperationProgress,
  HumanReviewProofPresentation,
  HumanReviewSource,
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
    expect(launcherCss).toMatch(
      /@media\s*\(max-width:\s*900px\)[\s\S]*?\.commit-history__columns\s*{[^}]*grid-template-columns:\s*1fr;/,
    );
    expect(launcherCss).toMatch(
      /\.human-review__build-browser\s*{[^}]*grid-template-columns:\s*minmax\(240px, 0\.7fr\) minmax\(0, 1\.3fr\);/s,
    );
    expect(launcherCss).toMatch(
      /@media\s*\(max-width:\s*900px\)[\s\S]*?\.human-review__build-browser\s*{[^}]*grid-template-columns:\s*1fr;/,
    );
    expect(launcherCss).toMatch(
      /@media\s*\(max-width:\s*560px\)[\s\S]*?\.commit-history__summary > div\s*{[^}]*grid-template-columns:\s*1fr;/,
    );
  });

  it('prepares and opens a named instance through semantic controls without infrastructure details', async () => {
    const client = new FakeClient();
    render(<HumanReviewLauncherView client={client} />);

    await screen.findByRole('button', { name: /codex\/feature/ });
    fireEvent.click(screen.getByRole('button', { name: 'Prepare new build' }));
    fireEvent.change(screen.getByLabelText('Build name'), {
      target: { value: 'Checkout review' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Prepare build' }));

    const details = await screen.findByRole('region', {
      name: 'Selected retained build details',
    });
    const review = within(details)
      .getByRole('heading', { name: 'Checkout review' })
      .closest('article');
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
      preparedRevision: '333333333333',
      currentRevision: '444444444444',
      sourceState: 'outdated' as const,
      outdatedByCommits: 3,
      currentUse: 'Source changed since this build',
      actionRequired: true,
      actionSummary: 'Prepare a fresh instance for the selected worktree.',
    };
    client.listInstances = async () => [alpha, beta, historical];
    client.detail = async (instanceRef) => detail(instanceRef === 'beta' ? beta : alpha);
    render(<HumanReviewLauncherView client={client} />);

    expect(await screen.findByRole('heading', { name: 'Alpha build' })).toBeVisible();
    expect(screen.getByRole('button', { name: /^Beta build/ })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: /^Historical build/ }));
    const historicalCard = screen
      .getByRole('heading', { name: 'Historical build' })
      .closest('article')!;
    expect(within(historicalCard).getByText('Source changed since this build')).toBeVisible();
    expect(within(historicalCard).getByText('Outdated by 3 commits')).toBeVisible();
    expect(within(historicalCard).getByRole('button', { name: 'Rebuild' })).toBeEnabled();
    expect(within(historicalCard).getByRole('button', { name: 'Open' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: /^Alpha build/ }));
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
    expect(await screen.findByRole('heading', { name: 'Retained builds' })).toBeVisible();
  });

  it('shows only the retained builds belonging to the selected worktree', async () => {
    const client = new FakeClient();
    const alpha = {
      ...instance('Alpha retained', 'prepared', 'passed'),
      sourceLabel: 'codex/alpha',
    };
    const beta = {
      ...instance('Beta retained', 'prepared', 'passed'),
      instanceRef: 'beta-instance',
      sourceRef: 'beta-source',
      sourceLabel: 'codex/beta',
    };
    client.listSources = async () => [
      source({ branch: 'codex/alpha', label: 'codex/alpha' }),
      source({ sourceRef: 'beta-source', branch: 'codex/beta', label: 'codex/beta' }),
    ];
    client.listInstances = async () => [alpha, beta];
    render(<HumanReviewLauncherView client={client} />);

    expect(await screen.findByRole('heading', { name: 'Alpha retained' })).toBeVisible();
    expect(screen.queryByRole('button', { name: /^Beta retained/ })).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: /codex\/beta/ }));
    expect(await screen.findByRole('heading', { name: 'Beta retained' })).toBeVisible();
    expect(screen.queryByRole('button', { name: /^Alpha retained/ })).toBeNull();
  });

  it('rebuilds an outdated retained build as a new current replacement', async () => {
    const client = new FakeClient();
    const outdated = {
      ...instance('Outdated build', 'stopped', 'superseded'),
      instanceRef: 'outdated-instance',
      preparedRevision: '222222222222',
      sourceState: 'outdated' as const,
      outdatedByCommits: 2,
    };
    const replacement = {
      ...instance('Outdated build', 'prepared', 'not-built'),
      instanceRef: 'replacement-instance',
    };
    client.listInstances = async () => [outdated];
    client.prepare = vi.fn(async () => replacement);
    client.build = vi.fn(async () => ({ ...replacement, build: 'passed' as const }));
    render(<HumanReviewLauncherView client={client} />);

    const outdatedBuild = await screen.findByRole('button', {
      name: /^Outdated build Outdated by 2 commits/,
    });
    fireEvent.click(outdatedBuild);
    await screen.findByRole('button', { name: 'Rebuild' });
    fireEvent.click(screen.getByRole('button', { name: 'Rebuild' }));

    await waitFor(() => expect(client.prepare).toHaveBeenCalledTimes(1));
    expect(client.prepare).toHaveBeenCalledWith(
      expect.stringMatching(/^prepare-/),
      outdated.sourceRef,
      outdated.name,
    );
    await waitFor(() => expect(client.build).toHaveBeenCalledTimes(1));
    expect(client.build).toHaveBeenCalledWith(
      expect.stringMatching(/^build-/),
      replacement.instanceRef,
    );
    const details = screen.getByRole('region', { name: 'Selected retained build details' });
    expect(within(details).getByText('Built from the current branch head')).toBeVisible();
    expect(
      screen.getByRole('button', { name: /^Outdated build Outdated by 2 commits/ }),
    ).toBeVisible();
  });

  it('uses typed proof presentation for legacy selection and retained full-output drill-down', async () => {
    const client = new FakeClient();
    const legacy = {
      ...instance('Legacy build', 'prepared', 'not-built'),
      sourceRef: 'legacy-source',
      compatibility: 'incompatible' as const,
      actionRequired: true,
      actionSummary: 'Update to a compatible worktree lineage before Build or Open.',
    };
    client.listSources = async () => [
      source({
        sourceRef: 'compatible-source',
        branch: 'codex/compatible',
        label: 'codex/compatible',
      }),
      source({
        sourceRef: 'legacy-source',
        branch: 'legacy branch',
        label: 'legacy branch',
        revision: '123456789abc',
        compatibility: 'incompatible' as const,
        compatibilityMessage:
          'This branch predates the Worktree Review child contract. Update it before Build or Open.',
      }),
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
    fireEvent.click(screen.getByRole('button', { name: /^Legacy build/ }));
    const card = screen.getByRole('heading', { name: 'Legacy build' }).closest('article')!;
    expect(within(card).getByRole('button', { name: 'Build' })).toBeDisabled();
    expect(within(card).getByRole('button', { name: 'Open' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Prepare new build' })).toBeDisabled();

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

  it('shows one row per branch and a selectable newest-first commit history', async () => {
    const client = new FakeClient();
    client.listSources = async () => [
      source({
        sourceRef: 'main-source',
        branch: 'main',
        label: 'main - launcher source',
        isMain: true,
        isCurrent: true,
        ahead: 0,
        forkRevision: '111111111111',
        revision: '111111111111',
      }),
      source({
        sourceRef: 'parent-source',
        branch: 'codex/parent',
        label: 'codex/parent - parent',
        parentSourceRef: 'main-source',
        revision: '222222222222',
      }),
      source({
        sourceRef: 'child-source',
        branch: 'codex/child',
        label: 'codex/child - child',
        parentSourceRef: 'parent-source',
        ahead: 2,
        revision: '444444444444',
      }),
      source({
        sourceRef: 'detached-source',
        branch: undefined,
        label: 'Detached 55555555',
        detached: true,
        revision: '555555555555',
      }),
    ];
    render(<HumanReviewLauncherView client={client} />);

    await screen.findByRole('heading', { name: 'Repository history' });
    const sourcePicker = screen.getByRole('region', { name: 'Review source' });
    const childRow = within(sourcePicker).getByRole('button', { name: /codex\/child/ });
    expect(childRow).toHaveTextContent('Review worktree ready');
    expect(screen.queryByRole('button', { name: /Detached 55555555/ })).toBeNull();
    fireEvent.click(childRow);
    const historyTrigger = screen.getByRole('button', { name: 'Compare with main' });
    fireEvent.click(historyTrigger);

    const dialog = await screen.findByRole('dialog', { name: 'codex/child' });
    const close = within(dialog).getByRole('button', { name: 'Close history' });
    await waitFor(() => expect(close).toHaveFocus());
    expect(document.querySelector('.human-review')).toHaveAttribute('aria-hidden', 'true');
    expect((document.querySelector('.human-review') as HTMLElement).inert).toBe(true);
    expect(dialog).toHaveTextContent('2 commits since main · fork 111111111111');
    expect(within(dialog).getByText('Branch lineage').closest('div')).toHaveTextContent(
      'codex/parent',
    );
    const details = within(dialog).getByRole('region', { name: 'Commit details' });
    expect(details).toHaveTextContent('Newest description');
    const parentCommit = within(dialog).getByRole('button', { name: /Parent foundation/ });
    fireEvent.keyDown(document, { key: 'Tab', shiftKey: true });
    expect(parentCommit).toHaveFocus();
    fireEvent.keyDown(document, { key: 'Tab' });
    expect(close).toHaveFocus();
    fireEvent.click(parentCommit);
    expect(details).toHaveTextContent('Parent branch description');
    expect(details).toHaveTextContent('3 files changed');

    fireEvent.keyDown(document, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    await waitFor(() => expect(historyTrigger).toHaveFocus());
    expect(document.querySelector('.human-review')).not.toHaveAttribute('aria-hidden');
    expect((document.querySelector('.human-review') as HTMLElement).inert).toBe(false);
    expect(screen.getByRole('button', { name: /codex\/child/ })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    fireEvent.change(screen.getByRole('searchbox', { name: 'Search repository history' }), {
      target: { value: 'parent' },
    });
    expect(screen.queryByRole('button', { name: /codex\/child/ })).toBeNull();
    expect(screen.getByRole('button', { name: /codex\/parent/ })).toBeVisible();
  });

  it('moves the same attached worktree under its refreshed branch head', async () => {
    const client = new FakeClient();
    let branch = 'codex/origin';
    client.listSources = async () => [
      source({
        sourceRef: 'moving-worktree',
        branch,
        label: `${branch} - active`,
      }),
    ];
    render(<HumanReviewLauncherView client={client} />);

    const original = await screen.findByRole('button', { name: /codex\/origin/ });
    expect(original).toHaveAttribute('aria-pressed', 'true');

    branch = 'codex/new-head';
    fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));

    const moved = await screen.findByRole('button', { name: /codex\/new-head/ });
    expect(moved).toHaveAttribute('aria-pressed', 'true');
    expect(screen.queryByText('codex/origin')).toBeNull();
  });

  it('makes the worktree prerequisite explicit and updates the same branch row after creation', async () => {
    const client = new FakeClient();
    let attached = false;
    const archived = () =>
      source({
        sourceRef: 'archive-source',
        branch: 'codex/explore-harness-inspector',
        label: 'codex/explore-harness-inspector',
        refKind: 'archive',
        attached,
        compatibility: attached ? 'compatible' : 'incompatible',
        compatibilityMessage: attached
          ? 'Compatible.'
          : 'Attach a review worktree before preparing a build.',
      });
    client.listSources = async () => [archived()];
    client.attachWorktree = vi.fn(async () => {
      attached = true;
      return archived();
    });
    render(<HumanReviewLauncherView client={client} />);

    const row = await screen.findByRole('button', { name: /codex\/explore-harness-inspector/ });
    expect(row).toHaveTextContent('Review worktree needed');
    expect(screen.getByText(/must be created before this branch can be built/)).toBeVisible();
    expect(screen.getByRole('button', { name: 'Prepare new build' })).toBeDisabled();

    fireEvent.click(screen.getByRole('button', { name: 'Create review worktree' }));
    await waitFor(() => expect(client.attachWorktree).toHaveBeenCalledWith('archive-source'));
    expect(await screen.findByText('Review worktree is ready to build.')).toBeVisible();
    expect(screen.getByRole('button', { name: /codex\/explore-harness-inspector/ })).toHaveTextContent(
      'Review worktree ready',
    );
    expect(screen.getByRole('button', { name: 'Prepare new build' })).toBeEnabled();
  });

  it('reports history failures and renders an empty named-branch history truthfully', async () => {
    const client = new FakeClient();
    client.sourceHistory = vi
      .fn()
      .mockRejectedValueOnce(new Error('Git could not inspect the selected branch.'))
      .mockResolvedValueOnce({
        ...history(),
        commitCount: 0,
        commits: [],
        lineageMarkers: [],
      });
    render(<HumanReviewLauncherView client={client} />);

    const trigger = await screen.findByRole('button', { name: 'Compare with main' });
    fireEvent.click(trigger);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Git could not inspect the selected branch.',
    );
    expect(trigger).toBeEnabled();
    expect(screen.queryByRole('dialog')).toBeNull();

    fireEvent.click(trigger);
    const dialog = await screen.findByRole('dialog', { name: 'codex/child' });
    expect(dialog).toHaveTextContent('0 commits since main');
    expect(dialog).toHaveTextContent('This branch has no commits beyond main.');
    expect(dialog).toHaveTextContent('Select a commit to inspect its details.');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Close history' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });

  it('reports raw unrelated-history facts without inferring a product meaning', async () => {
    const client = new FakeClient();
    client.listSources = async () => [
      source({
        sourceRef: 'unrelated-source',
        branch: 'codex/orphan',
        label: 'codex/orphan',
        parentSourceRef: undefined,
        relationship: 'unrelated',
        forkRevision: 'No common ancestor',
      }),
    ];
    render(<HumanReviewLauncherView client={client} />);

    await waitFor(() => expect(screen.getByText(/No common ancestor with main/)).toBeVisible());
    expect(screen.getByRole('button', { name: 'Compare with main' })).toBeDisabled();
  });

  it('returns to an available source when refresh removes the selected worktree', async () => {
    const client = new FakeClient();
    const main = source({
      sourceRef: 'main-source',
      branch: 'main',
      label: 'main',
      isMain: true,
    });
    let sources = [
      main,
      source({
        sourceRef: 'temporary-source',
        branch: 'codex/temporary',
        label: 'codex/temporary',
        parentSourceRef: 'main-source',
      }),
    ];
    client.listSources = async () => sources;
    render(<HumanReviewLauncherView client={client} />);

    fireEvent.click(await screen.findByRole('button', { name: /codex\/temporary/ }));
    sources = [main];
    fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: /^main Branch/ }),
      ).toHaveAttribute('aria-pressed', 'true'),
    );
    expect(screen.queryByRole('button', { name: /codex\/temporary/ })).toBeNull();
  });
});

class LongBuildClient implements HumanReviewLauncherClient {
  private progressCalls = 0;
  private resolveBuild!: (value: HumanReviewInstance) => void;
  private instance = instance('Long build', 'prepared', 'not-built');
  listSources = async () => [source({ branch: 'codex/long', label: 'codex/long' })];
  sourceHistory: HumanReviewLauncherClient['sourceHistory'] = async () => history();
  attachWorktree: HumanReviewLauncherClient['attachWorktree'] = async () => source();
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
  listSources: HumanReviewLauncherClient['listSources'] = async () => [source()];
  sourceHistory: HumanReviewLauncherClient['sourceHistory'] = async () => history();
  attachWorktree: HumanReviewLauncherClient['attachWorktree'] = async (sourceRef) =>
    source({ sourceRef, attached: true });
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
    sourceRef: 'opaque',
    sourceLabel: 'codex/feature - review',
    preparedRevision: '444444444444',
    currentRevision: '444444444444',
    sourceState: 'current',
    outdatedByCommits: 0,
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

function source(overrides: Partial<HumanReviewSource> = {}): HumanReviewSource {
  return {
    sourceRef: 'opaque',
    label: 'codex/feature - review',
    branch: 'codex/feature',
    detached: false,
    isMain: false,
    isCurrent: false,
    parentSourceRef: undefined,
    lineageAmbiguous: false,
    relationship: 'related',
    ahead: 2,
    behind: 0,
    forkRevision: '111111111111',
    revision: '444444444444',
    compatibility: 'compatible',
    compatibilityMessage: 'Compatible.',
    attached: true,
    refKind: 'branch',
    mergedDirectly: false,
    equivalentPatches: 0,
    comparisonBranch: 'main',
    ...overrides,
  };
}

function history() {
  return {
    branch: 'codex/child',
    sourceLabel: 'codex/child - child',
    revision: '444444444444',
    forkRevision: '111111111111',
    commitCount: 2,
    truncated: false,
    commits: [
      {
        id: '4444444444444444444444444444444444444444',
        abbreviatedId: '4444444',
        subject: 'Newest feature commit',
        description: 'Newest description',
        author: 'Codex',
        committedAt: '2026-08-09T12:00:00Z',
        filesChanged: 5,
        insertions: 30,
        deletions: 4,
      },
      {
        id: '2222222222222222222222222222222222222222',
        abbreviatedId: '2222222',
        subject: 'Parent foundation',
        description: 'Parent branch description',
        author: 'Codex',
        committedAt: '2026-08-08T12:00:00Z',
        filesChanged: 3,
        insertions: 12,
        deletions: 2,
      },
    ],
    lineageMarkers: [
      {
        branch: 'codex/parent',
        commitId: '2222222222222222222222222222222222222222',
        abbreviatedId: '2222222',
      },
    ],
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
