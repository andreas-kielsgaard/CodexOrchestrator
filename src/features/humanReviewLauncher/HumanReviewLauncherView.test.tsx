import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
  HumanReviewOperationProgress,
  HumanReviewSource,
} from '../../application/humanReviewLauncher';
import type { WorktreeBuildDetail } from '../../application/worktreeBuild';
import { HumanReviewLauncherView } from './HumanReviewLauncherView';

const launcherCss = readFileSync(
  'src/features/humanReviewLauncher/humanReviewLauncher.css',
  'utf8',
);

describe('HumanReviewLauncherView', () => {
  it('renders worktree choices before retained build discovery finishes', async () => {
    const client = new FakeClient();
    let finishInstances!: (value: HumanReviewInstance[]) => void;
    client.listSources = vi.fn(async () => [source({ branch: 'codex/fast', label: 'codex/fast' })]);
    client.listInstances = () =>
      new Promise<HumanReviewInstance[]>((resolve) => {
        finishInstances = resolve;
      });

    render(<HumanReviewLauncherView client={client} />);

    expect(await screen.findByRole('button', { name: /codex\/fast/ })).toBeVisible();
    expect(client.listSources).toHaveBeenCalledWith({ includeDetached: false, refresh: false });
    expect(screen.getByText('Loading build...')).toBeVisible();
    finishInstances([]);
    await waitFor(() => expect(screen.getByText('No build for this worktree')).toBeVisible());
  });

  it('preserves branch selection while pending Git details become ready', async () => {
    const client = new FakeClient();
    let reads = 0;
    client.listSources = async (options) => {
      reads += 1;
      const pending = reads === 1;
      return [
        source({
          sourceRef: 'main-source',
          branch: 'main',
          label: 'main',
          isMain: true,
          detailsState: pending ? 'pending' : 'ready',
        }),
        source({
          sourceRef: 'child-source',
          branch: 'codex/child',
          label: 'codex/child',
          parentSourceRef: pending ? undefined : 'main-source',
          detailsState: pending ? 'pending' : 'ready',
          ahead: pending ? 0 : 3,
        }),
        ...(options?.includeDetached
          ? [source({ sourceRef: 'detached', branch: undefined, detached: true })]
          : []),
      ];
    };

    render(<HumanReviewLauncherView client={client} />);
    const child = await screen.findByRole('button', { name: /codex\/child/ });
    fireEvent.click(child);
    await waitFor(() => expect(reads).toBeGreaterThan(1));
    expect(screen.getByRole('button', { name: /codex\/child/ })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(screen.queryByRole('button', { name: /Detached/ })).toBeNull();
  });

  it('shows attachment on the branch line and requires a worktree before building', async () => {
    const client = new FakeClient();
    client.listSources = async () => [
      source({ sourceRef: 'main-source', branch: 'main', isMain: true }),
    ];
    client.listRepositoryHistory = async () => [
      source({ sourceRef: 'main-source', branch: 'main', isMain: true }),
      source({
        sourceRef: 'archive-source',
        branch: 'archive/codex/explore-harness-inspector',
        attached: false,
        refKind: 'archive',
        compatibility: 'unavailable',
        compatibilityMessage: 'Create a review worktree first.',
      }),
    ];
    client.attachWorktree = vi.fn(async () =>
      source({
        sourceRef: 'archive-source',
        branch: 'archive/codex/explore-harness-inspector',
        attached: true,
        refKind: 'archive',
      }),
    );

    render(<HumanReviewLauncherView client={client} />);

    await screen.findByRole('button', { name: /Main checkout/ });
    fireEvent.click(screen.getByRole('button', { name: 'Explore repository history' }));
    const explorer = await screen.findByRole('dialog', { name: 'Repository history' });
    expect(within(explorer).getByText('Review worktree needed')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Create build', hidden: true })).toBeEnabled();

    fireEvent.click(within(explorer).getByRole('button', { name: 'Create review worktree' }));
    await waitFor(() => expect(client.attachWorktree).toHaveBeenCalledWith('archive-source'));
    expect((await within(explorer).findAllByText('Review worktree ready')).length).toBeGreaterThan(
      0,
    );
    expect(screen.getByRole('button', { name: 'Create build', hidden: true })).toBeEnabled();
  });

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
    expect(launcherCss).toMatch(/\.human-review__build-detail\s*{[^}]*min-width:\s*0;/s);
    expect(launcherCss).toMatch(
      /@media\s*\(max-width:\s*560px\)[\s\S]*?\.commit-history__summary > div\s*{[^}]*grid-template-columns:\s*1fr;/,
    );
  });

  it('creates, builds, launches, focuses, and stops one worktree build', async () => {
    const client = new FakeClient();
    client.prepare = vi.fn(client.prepare);
    client.build = vi.fn(client.build);
    render(<HumanReviewLauncherView client={client} />);

    const create = await screen.findByRole('button', { name: 'Create build' });
    fireEvent.click(create);
    expect(await screen.findByRole('heading', { name: 'Build ready' })).toBeVisible();
    expect(client.prepare).toHaveBeenCalledWith(
      expect.stringMatching(/^prepare-/),
      'opaque',
      'Worktree review',
    );
    expect(client.build).toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Launch' }));
    expect(await screen.findByText('Running')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Focus window' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Stop' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }));
    await waitFor(() => expect(screen.getByText('Ready to launch')).toBeVisible());
  });

  it('renders typed long-build progress and quiet evidence without calling it stalled', async () => {
    const client = new LongBuildClient();
    render(<HumanReviewLauncherView client={client} />);
    const card = (await screen.findByRole('heading', { name: 'Build needed' })).closest('article')!;
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
    await waitFor(() => expect(within(card).getByText('Ready to launch')).toBeVisible());
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

  it('shows one preferred worktree build and keeps the detailed drill-down available', async () => {
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

    const alphaCard = (await screen.findByRole('heading', { name: 'Build ready' })).closest(
      'article',
    )!;
    expect(screen.queryByText('Beta build')).toBeNull();
    expect(screen.queryByText('Historical build')).toBeNull();
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
    expect(await screen.findByRole('heading', { name: 'Worktree build' })).toBeVisible();
  });

  it('shows the single build belonging to the selected worktree', async () => {
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

    const worktreeBuild = await screen.findByRole('region', { name: 'Worktree build' });
    expect(within(worktreeBuild).getByText('Build directly from codex/alpha.')).toBeVisible();
    expect(
      await within(worktreeBuild).findByRole('heading', { name: 'Build ready' }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: /codex\/beta/ }));
    await waitFor(() =>
      expect(within(worktreeBuild).getByText('Build directly from codex/beta.')).toBeVisible(),
    );
    expect(within(worktreeBuild).getAllByRole('heading', { name: 'Build ready' })).toHaveLength(1);
  });

  it('prefers an existing verified current build when a worktree is selected', async () => {
    const client = new FakeClient();
    const outdated = {
      ...instance('Older retained', 'stopped', 'superseded'),
      instanceRef: 'older-instance',
      sourceState: 'outdated' as const,
      outdatedByCommits: 2,
    };
    const current = {
      ...instance('Current retained', 'stopped', 'passed'),
      instanceRef: 'current-instance',
    };
    client.listInstances = async () => [outdated, current];
    render(<HumanReviewLauncherView client={client} />);

    const details = screen.getByRole('region', { name: 'Selected worktree build' });
    expect(await within(details).findByRole('heading', { name: 'Build ready' })).toBeVisible();
    expect(within(details).getByRole('button', { name: 'Launch' })).toBeEnabled();
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

    expect(await screen.findByText('Outdated by 2 commits')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Update build' }));

    await waitFor(() => expect(client.prepare).toHaveBeenCalledTimes(1));
    expect(client.prepare).toHaveBeenCalledWith(
      expect.stringMatching(/^prepare-/),
      outdated.sourceRef,
      'Worktree review',
    );
    await waitFor(() => expect(client.build).toHaveBeenCalledTimes(1));
    expect(client.build).toHaveBeenCalledWith(
      expect.stringMatching(/^build-/),
      replacement.instanceRef,
    );
    const details = screen.getByRole('region', { name: 'Selected worktree build' });
    expect(within(details).getByText("Built from the worktree's current HEAD")).toBeVisible();
    expect(within(details).getByRole('button', { name: 'Launch' })).toBeEnabled();
  });

  it('keeps legacy selection safe and exposes retained full output through product controls', async () => {
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
    render(<HumanReviewLauncherView client={client} />);

    fireEvent.click(
      (await screen.findByText('legacy branch', { selector: 'strong' })).closest('button')!,
    );
    expect(await screen.findByText(/predates the Worktree Review child contract/)).toBeVisible();
    const card = (await screen.findByRole('heading', { name: 'Build needed' })).closest('article')!;
    expect(within(card).getByRole('button', { name: 'Build' })).toBeDisabled();
    expect(within(card).queryByRole('button', { name: 'Launch' })).toBeNull();

    fireEvent.click(within(card).getByRole('button', { name: 'Build details' }));
    const detailView = await screen.findByRole('main', { name: 'Worktree build details' });
    fireEvent.click(within(detailView).getByText(/build .* Finished .* succeeded/));
    expect(within(detailView).getByText(/24 safe lines/)).toBeVisible();
    expect(detailView).toHaveTextContent('safe output 24');
  });

  it('shows a main-rooted branch map and a selectable newest-first commit history', async () => {
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
    client.listRepositoryHistory = client.listSources;
    render(<HumanReviewLauncherView client={client} />);

    await screen.findByRole('button', { name: /Main checkout/ });
    fireEvent.click(screen.getByRole('button', { name: 'Explore repository history' }));
    const explorer = await screen.findByRole('dialog', { name: 'Repository history' });
    fireEvent.change(
      within(explorer).getByRole('combobox', { name: 'Choose repository version' }),
      {
        target: { value: 'child-source' },
      },
    );
    expect(
      within(explorer).getByRole('generic', { name: 'Focused lineage for codex/child' }),
    ).toBeVisible();
    expect(within(explorer).queryByText('Detached 55555555')).toBeNull();
    const historyTrigger = within(explorer).getByRole('button', { name: 'Compare with main' });
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
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'codex/child' })).toBeNull());
    expect(screen.getByRole('dialog', { name: 'Repository history' })).toBeVisible();
    await waitFor(() => expect(historyTrigger).toHaveFocus());
    expect(document.querySelector('.human-review')).toHaveAttribute('aria-hidden', 'true');
    expect((document.querySelector('.human-review') as HTMLElement).inert).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Close repository history' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(document.querySelector('.human-review')).not.toHaveAttribute('aria-hidden');
    expect((document.querySelector('.human-review') as HTMLElement).inert).toBe(false);
    expect(screen.queryByRole('button', { name: /Detached 55555555/ })).toBeNull();
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

    const trigger = await screen.findByRole('button', { name: 'View commit history' });
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

  it('reports direct merge and unrelated history facts without inferring lineage', async () => {
    const client = new FakeClient();
    client.listSources = async () => [
      source({
        sourceRef: 'main-source',
        branch: 'main',
        label: 'main',
        isMain: true,
        parentSourceRef: undefined,
      }),
      source({
        sourceRef: 'ambiguous-source',
        branch: 'codex/ambiguous',
        label: 'codex/ambiguous',
        parentSourceRef: 'main-source',
        lineageAmbiguous: true,
      }),
      source({
        sourceRef: 'unrelated-source',
        branch: 'codex/orphan',
        label: 'codex/orphan',
        parentSourceRef: undefined,
        relationship: 'unrelated',
        forkRevision: 'No common ancestor',
      }),
    ];
    client.listRepositoryHistory = client.listSources;
    render(<HumanReviewLauncherView client={client} />);

    await screen.findByRole('button', { name: /Main checkout/ });
    fireEvent.click(screen.getByRole('button', { name: 'Explore repository history' }));
    const explorer = await screen.findByRole('dialog', { name: 'Repository history' });
    const versions = within(explorer).getByRole('combobox', { name: 'Choose repository version' });
    fireEvent.change(versions, { target: { value: 'ambiguous-source' } });
    expect(within(explorer).getByText('Not merged directly')).toBeVisible();
    expect(within(explorer).getByRole('button', { name: 'Compare with main' })).toBeEnabled();
    fireEvent.change(versions, { target: { value: 'unrelated-source' } });
    expect(within(explorer).getByRole('button', { name: 'Compare with main' })).toBeDisabled();
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
      expect(screen.getByRole('button', { name: /Main checkout/ })).toHaveAttribute(
        'aria-pressed',
        'true',
      ),
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
  listInstances = async () => [this.instance];
  settings = async () => ({ cleanupDetachedBuilds: false });
  updateSettings = async (settings: { cleanupDetachedBuilds: boolean }) => settings;
  listRepositoryHistory = async () => this.listSources();
  attachWorktree = async () => source();
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
  listSources: HumanReviewLauncherClient['listSources'] = async () => [source()];
  sourceHistory: HumanReviewLauncherClient['sourceHistory'] = async () => history();
  listInstances = async () => (this.instance ? [this.instance] : []);
  settings = async () => ({ cleanupDetachedBuilds: false });
  updateSettings = async (settings: { cleanupDetachedBuilds: boolean }) => settings;
  listRepositoryHistory = async () => this.listSources();
  attachWorktree = async (sourceRef: string) => source({ sourceRef, attached: true });
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
    detailsState: 'ready',
    attached: true,
    refKind: 'local_branch',
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
