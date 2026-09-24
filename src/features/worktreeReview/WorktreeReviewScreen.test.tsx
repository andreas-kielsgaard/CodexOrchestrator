import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type {
  RegisteredRepository,
  RepositoryCatalogClient,
  RepositoryCatalogOverview,
} from '../../application/repositoryCatalog';
import type {
  AssociateWorktreeRequest,
  AssociatedWorktree,
  ReviewTarget,
  CommitHistoryQuery,
  CommitHistoryPage,
  BranchGraphData,
  BranchReviewDetail,
  BuildId,
  CreateBuildRequest,
  OpenBuildOutcome,
  RepositoryId,
  ReviewBuild,
  ReviewBuildSource,
  WorktreeReviewClient,
  WorktreeReviewOverview,
} from '../../application/worktreeReview';
import { WorktreeReviewScreen } from './WorktreeReviewScreen';
import {
  baseCommit,
  branchDetailFixture,
  earlierCommit,
  overviewFixture,
  completedBuild,
  tipCommit,
  worktreeOne,
  worktreeTwo,
} from './WorktreeReviewScreen.fixtures';

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
  };
});

describe('WorktreeReviewScreen', () => {
  it('shows completion on the exact branch and source worktree, then clears after loading it', async () => {
    const markedRead = vi.fn();
    const unread = { ...completedBuild, sourceWorktreeId: worktreeOne.worktreeId };
    renderScreen(new FixtureClient(), new FixtureCatalog(), [unread], markedRead);

    await screen.findByRole('heading', { name: 'codex/durable-review' });
    expect(
      within(screen.getByRole('button', { name: /codex\/durable-review/ })).getByLabelText(
        'Build finished',
      ),
    ).toBeVisible();
    expect(
      within(screen.getByText('Agent checkout A').closest('label')!).getByLabelText(
        'Build finished',
      ),
    ).toBeVisible();
    await waitFor(() => expect(markedRead).toHaveBeenCalledWith([completedBuild.buildId]));
  });

  it('creates a live build from the modal, defaults to normal, and never launches it', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    await screen.findByRole('heading', { name: 'codex/durable-review' });
    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    const dialog = screen.getByRole('dialog', { name: 'Create a build' });
    expect(within(dialog).getByRole('combobox', { name: 'Build mode' })).toHaveValue('release');
    expect(within(dialog).getByText('Build directly in the worktree.')).toBeVisible();
    expect(client.createBuildCalls).toHaveLength(0);
    await user.click(within(dialog).getByRole('button', { name: 'Create build' }));
    await waitFor(() => expect(client.createBuildCalls).toHaveLength(1));
    expect(client.createBuildCalls[0].profile).toBe('release');
    expect(client.openBuildCalls).toEqual([]);
  });

  it('keeps navigation and a second build available after a background build starts', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    client.createBuild = async (input: CreateBuildRequest): Promise<ReviewBuild> => {
      client.createBuildCalls.push(input);
      return {
        ...completedBuild,
        buildId: `running-${client.createBuildCalls.length}`,
        name: input.name,
        source: sourceReceipt(input),
        latestAttempt: {
          attemptId: `attempt-running-${client.createBuildCalls.length}`,
          executionState: 'running',
          outcome: 'unknown',
          stage: 'compilation',
          startedAt: '2026-08-24T11:00:00Z',
        },
        output: { state: 'not_produced' },
      };
    };
    renderScreen(client);
    await screen.findByRole('heading', { name: 'codex/durable-review' });

    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    await user.click(screen.getByRole('button', { name: 'Create build' }));
    expect(await screen.findByText(/continue navigating while it runs/)).toBeVisible();
    const main = screen.getByRole('button', { name: /main/ });
    expect(main).toBeEnabled();
    await user.click(main);
    expect(await screen.findByRole('heading', { name: 'main' })).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    await user.click(screen.getByRole('button', { name: 'Create build' }));
    await waitFor(() => expect(client.createBuildCalls).toHaveLength(2));
  });

  it('resets build mode after cancellation and sends debugging only on creation', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    await screen.findByRole('heading', { name: 'codex/durable-review' });
    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    await user.selectOptions(screen.getByRole('combobox', { name: 'Build mode' }), 'debug');
    await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Cancel' }));
    expect(client.createBuildCalls).toHaveLength(0);
    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    expect(screen.getByRole('combobox', { name: 'Build mode' })).toHaveValue('release');
    await user.selectOptions(screen.getByRole('combobox', { name: 'Build mode' }), 'debug');
    await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Create build' }));
    await waitFor(() => expect(client.createBuildCalls[0]?.profile).toBe('debug'));
    expect(client.openBuildCalls).toEqual([]);
  });

  it('registers a directory in the separate repository modal before presenting branches', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient(branchDetailFixture(), { repositories: [], branches: [] });
    const catalog = new FixtureCatalog();
    renderScreen(client, catalog);

    await user.click(await screen.findByRole('button', { name: 'Add repository…' }));
    const input = await screen.findByRole('textbox', {
      name: 'Directory inside a Git repository',
    });
    await user.type(input, 'C:\\Projects\\Codex Orchestrator');
    await user.click(screen.getByRole('button', { name: 'Add directory' }));

    expect(catalog.registerDirectoryCalls).toEqual(['C:\\Projects\\Codex Orchestrator']);
    expect(client.selectRepositoryCalls).toEqual(['repository-one']);
    expect(await screen.findByRole('heading', { name: 'codex/durable-review' })).toBeVisible();
  });

  it('registers a local instance exposed through Codex while leaving remote-only repos informational', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient(branchDetailFixture(), { repositories: [], branches: [] });
    const catalog = new FixtureCatalog();
    renderScreen(client, catalog);

    await user.click(await screen.findByRole('button', { name: 'Add repository…' }));
    expect(await screen.findByText('remote/only')).toBeVisible();
    expect(screen.getByText('Not available locally')).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Register repository' }));

    expect(catalog.registerCodexCalls).toEqual(['repository-one']);
    expect(client.selectRepositoryCalls).toEqual(['repository-one']);
    expect(await screen.findByRole('heading', { name: 'codex/durable-review' })).toBeVisible();
  });

  it('selects a branch before distinguishing multiple exact worktrees at the same commit', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);

    expect(await screen.findByRole('heading', { name: 'codex/durable-review' })).toBeVisible();
    const worktreeGroup = screen.getByRole('radiogroup', { name: 'Worktree checkout' });
    const first = within(worktreeGroup).getByRole('radio', { name: /Agent checkout A/ });
    const second = within(worktreeGroup).getByRole('radio', { name: /Agent checkout B/ });
    expect(first).toBeChecked();
    expect(second).not.toBeChecked();
    expect(within(worktreeGroup).getAllByText('22222222')).toHaveLength(2);
    expect(
      screen.getByText(
        '2 worktrees are available. Your exact selection will be recorded with the build.',
      ),
    ).toBeVisible();

    await user.click(second);
    expect(await screen.findByRole('heading', { name: 'Agent checkout B' })).toBeVisible();
    expect(screen.getByRole('radio', { name: /Agent checkout B/ })).toBeChecked();
    expect(screen.getByText('Detached HEAD')).toBeVisible();

    const branchButton = screen.getByRole('button', { name: /main/ });
    await user.click(branchButton);
    expect(branchButton).toHaveFocus();
    expect(await screen.findByRole('heading', { name: 'main' })).toBeVisible();
  });

  it('keeps the navigator count aligned with the selected branch detail', async () => {
    const initialOverview = {
      ...overviewFixture,
      branches: overviewFixture.branches.map((branch) => ({
        ...branch,
        associatedWorktreeCount: 0,
      })),
    };
    const client = new FixtureClient(branchDetailFixture(), initialOverview);
    renderScreen(client);

    await screen.findByRole('heading', { name: 'codex/durable-review' });
    expect(screen.getByRole('button', { name: /codex\/durable-review/ })).toHaveTextContent(
      '2 worktrees',
    );
  });

  it('records an exact worktree snapshot and discloses its retained owned checkout', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    await screen.findByRole('heading', { name: 'codex/durable-review' });

    await user.click(screen.getByRole('radio', { name: /Agent checkout B/ }));
    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    await user.click(screen.getByRole('radio', { name: /Snapshot current work/ }));
    expect(screen.getByText('Copy the selected worktree and build in the destination.')).toBeVisible();
    expect(screen.getByText(/create and retain an isolated Worktree checkout/)).toBeVisible();

    const name = screen.getByRole('textbox', { name: 'Build name' });
    await user.clear(name);
    await user.type(name, 'Snapshot B');
    expect(client.createBuildCalls).toHaveLength(0);
    await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Create build' }));

    await waitFor(() => expect(client.createBuildCalls).toHaveLength(1));
    expect(client.createBuildCalls[0]).toMatchObject({
      name: 'Snapshot B',
      source: {
        kind: 'physical_worktree',
        worktreeId: worktreeTwo.worktreeId,
        snapshot: true,
      },
      workspacePlan: {
        kind: 'create_owned_build_worktree',
      },
    });
    expect(await screen.findByText(/Build started/)).toHaveAttribute('role', 'status');
  });

  it('allows Create Build without a pre-existing worktree and makes automatic creation explicit', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient(
      branchDetailFixture({ worktrees: [], associationCandidates: [], builds: [] }),
    );
    renderScreen(client);

    expect(await screen.findByText(/No matching worktree exists/)).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    expect(screen.getByRole('radio', { name: /Specific commit/ })).toBeChecked();
    expect(screen.getByText(/first create and retain a Worktree checkout/)).toHaveTextContent(
      'first create and retain a Worktree checkout for this branch',
    );
    expect(client.createBuildCalls).toHaveLength(0);
    await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Create build' }));
    await waitFor(() => expect(client.createBuildCalls).toHaveLength(1));
    expect(client.createBuildCalls[0]).toMatchObject({
      source: {
        kind: 'branch_commit',
        branchRef: 'refs/heads/codex/durable-review',
        objectId: tipCommit.objectId,
      },
      workspacePlan: {
        kind: 'create_managed_branch_worktree',
        branchRef: 'refs/heads/codex/durable-review',
      },
    });
  });

  it('requires an explicit branch association for a detached checkout and records the selected baseline', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    await screen.findByRole('heading', { name: 'codex/durable-review' });

    expect(screen.getByText('Other existing checkouts')).toBeVisible();
    expect(
      screen.getByText('Detached investigation').closest('.worktree-review__candidate'),
    ).toHaveTextContent('Detached HEAD');
    expect(client.commitHistoryCalls).toHaveLength(0);
    await user.click(screen.getByRole('button', { name: 'Choose a specific baseline' }));
    await waitFor(() => expect(client.commitHistoryCalls).toHaveLength(1));
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Association baseline for Detached investigation' }),
      earlierCommit.objectId,
    );
    await user.click(screen.getByRole('button', { name: 'Associate with branch' }));

    await waitFor(() => expect(client.associateCalls).toHaveLength(1));
    expect(client.associateCalls[0]).toEqual({
      repositoryId: 'repository-one',
      branchRef: 'refs/heads/codex/durable-review',
      worktreeId: 'candidate-detached',
      baseline: { kind: 'selected_commit', objectId: earlierCommit.objectId },
    });
  });

  it('presents a simple available or unavailable build state', async () => {
    const client = new FixtureClient();
    renderScreen(client);
    const builds = await screen.findByRole('heading', { name: 'Builds' });
    const region = builds.closest('section')!;
    const failed = (
      await within(region).findByRole('heading', { name: 'Failed rebuild' })
    ).closest('article')!;
    expect(failed).toHaveTextContent('Build failed');
    expect(failed).not.toHaveTextContent('Eligible for cleanup');
    expect(failed).toHaveTextContent('Compiler exited with code 1');
    expect(within(failed).queryByRole('button', { name: 'Launch' })).not.toBeInTheDocument();

    const completed = within(region)
      .getByRole('heading', { name: 'Completed review build' })
      .closest('article')!;
    expect(completed).toHaveTextContent('Available');
    expect(completed).not.toHaveTextContent('Newest successful build for this source');
    expect(within(completed).getByRole('button', { name: 'Launch' })).toBeEnabled();
  });

  it('shows removed output as no longer available without a launch action', async () => {
    const removed = {
      ...completedBuild,
      buildId: 'build-removed',
      name: 'Removed review build',
      output: {
        state: 'removed' as const,
        buildOutputId: 'output-removed',
        removedAt: '2026-08-23T11:00:00Z',
      },
    };
    renderScreen(new FixtureClient(branchDetailFixture({ builds: [removed] })));

    const heading = await screen.findByRole('heading', { name: removed.name });
    const card = heading.closest('article')!;
    expect(card).toHaveTextContent('No longer available');
    expect(within(card).queryByRole('button', { name: 'Launch' })).not.toBeInTheDocument();
  });

  it('prefills rebuild for the same live worktree, name, and build mode', async () => {
    const user = userEvent.setup();
    const live = {
      ...completedBuild,
      profile: 'debug' as const,
      sourceWorktreeId: worktreeOne.worktreeId,
      source: {
        kind: 'existing_worktree' as const,
        associationId: worktreeOne.associationId!,
        triggerHeadObjectId: tipCommit.objectId,
      },
      workspace: {
        ...completedBuild.workspace,
        worktreeId: worktreeOne.worktreeId,
        ownership: 'borrowed_external' as const,
      },
    };
    renderScreen(new FixtureClient(branchDetailFixture({ builds: [live] })));

    const heading = await screen.findByRole('heading', { name: live.name });
    await user.click(within(heading.closest('article')!).getByRole('button', { name: 'Rebuild' }));
    const dialog = screen.getByRole('dialog', { name: 'Create a build' });
    expect(within(dialog).getByRole('radio', { name: /Live worktree checkout/ })).toBeChecked();
    expect(within(dialog).getByRole('textbox', { name: 'Build name' })).toHaveValue(live.name);
    expect(within(dialog).getByRole('combobox', { name: 'Build mode' })).toHaveValue('debug');
  });

  it('marks the exact active worktree and only blocks direct sources for that checkout', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient(branchDetailFixture(), {
      ...overviewFixture,
      activeBuildContext: {
        buildId: completedBuild.buildId,
        worktreeId: worktreeOne.worktreeId,
      },
    });
    renderScreen(client);

    const context = await screen.findByRole('region', { name: 'Active build context' });
    expect(context).toHaveTextContent(completedBuild.buildId);
    expect(context).toHaveTextContent(worktreeOne.worktreeId);
    const activeCheckout = screen
      .getByText('Agent checkout A')
      .closest('.worktree-review__worktree')!;
    expect(activeCheckout).toHaveTextContent('Active build checkout');
    await user.click(screen.getByRole('radio', { name: /Agent checkout A/ }));
    expect(await screen.findByRole('heading', { name: 'Agent checkout A' })).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    expect(screen.getByRole('radio', { name: /Live worktree checkout/ })).toBeDisabled();
    expect(screen.getByRole('radio', { name: /Snapshot current work/ })).toBeDisabled();
    expect(screen.getByRole('radio', { name: /Specific commit/ })).toBeChecked();
    await user.click(screen.getByRole('button', { name: 'Cancel' }));

    await user.click(screen.getByRole('button', { name: /codex\/durable-review/ }));
    await screen.findByRole('heading', { name: 'codex/durable-review' });
    await user.click(screen.getByRole('radio', { name: /Agent checkout B/ }));
    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    expect(screen.getByRole('radio', { name: /Live worktree checkout/ })).toBeEnabled();
    expect(screen.getByRole('radio', { name: /Snapshot current work/ })).toBeEnabled();
  });

  it('loads paginated branch history only when the specific commit picker is opened', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    await screen.findByRole('heading', { name: 'codex/durable-review' });

    expect(client.commitHistoryCalls).toHaveLength(0);
    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    await user.click(screen.getByRole('radio', { name: /Specific commit/ }));
    expect(client.commitHistoryCalls).toHaveLength(0);
    await user.click(screen.getByRole('button', { name: 'Choose from repository history…' }));
    const graph = await screen.findByRole('dialog', { name: 'Choose a commit' });
    await user.click(await within(graph).findByRole('button', { name: /^2 commits from/ }));
    await waitFor(() => expect(client.commitHistoryCalls).toEqual([{ cursor: undefined }]));
    expect(screen.getByRole('dialog', { name: 'Select a commit' })).toHaveTextContent(
      earlierCommit.subject,
    );
  });

  it('selects a graph commit in the build dialog and provisions only after Create build', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    await screen.findByRole('heading', { name: 'codex/durable-review' });
    await user.click(screen.getByRole('button', { name: 'Select branch…' }));
    const graph = await screen.findByRole('dialog', { name: 'Select branch' });
    await user.click(await within(graph).findByRole('button', { name: /^2 commits from/ }));
    const range = screen.getByRole('dialog', { name: 'Select a commit' });
    await user.click(
      await within(range).findByRole('radio', { name: new RegExp(earlierCommit.subject) }),
    );
    await user.click(within(range).getByRole('button', { name: 'Use this commit' }));
    expect(
      await screen.findByRole('heading', { name: `Commit ${earlierCommit.abbreviatedObjectId}` }),
    ).toBeVisible();
    expect(client.createBuildCalls).toHaveLength(0);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    expect(screen.getByRole('dialog', { name: 'Create a build' })).toHaveTextContent(
      earlierCommit.abbreviatedObjectId,
    );
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(client.createBuildCalls).toHaveLength(0);
    expect(
      screen.getByRole('heading', { name: `Commit ${earlierCommit.abbreviatedObjectId}` }),
    ).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Create a build' }));
    await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Create build' }));
    await waitFor(() => expect(client.createBuildCalls).toHaveLength(1));
    expect(client.createBuildCalls[0]).toMatchObject({
      branchRef: null,
      source: {
        kind: 'exact_commit',
        objectId: earlierCommit.objectId,
        context: { kind: 'branch', tipObjectId: tipCommit.objectId },
      },
    });
  });

  it('uses one graph Tab stop, previews with arrows, and restores nested modal focus', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    const trigger = await screen.findByRole('button', { name: 'Select branch…' });
    await user.click(trigger);
    const dialog = await screen.findByRole('dialog', { name: 'Select branch' });
    const graph = await within(dialog).findByRole('group', { name: 'Repository branch graph' });
    const cards = graph.querySelectorAll<HTMLButtonElement>('.branch-graph__target');
    expect([...graph.querySelectorAll('button')].filter((b) => b.tabIndex === 0)).toHaveLength(1);
    act(() => cards[0].focus());
    await user.keyboard('{ArrowDown}');
    expect(document.activeElement).toBe(cards[1]);
    expect(cards[0]).toHaveAttribute('aria-pressed', 'true');
    expect(client.commitHistoryCalls).toHaveLength(0);
    await user.tab();
    expect(document.activeElement).toBe(within(dialog).getByRole('button', { name: 'Use branch' }));
    await user.tab();
    expect(document.activeElement).toBe(within(dialog).getByRole('button', { name: 'Close' }));
    await user.tab({ shift: true });
    expect(document.activeElement).toBe(within(dialog).getByRole('button', { name: 'Use branch' }));
    const count = within(graph).getByRole('button', { name: /^2 commits from/ });
    await user.click(count);
    const nested = await screen.findByRole('dialog', { name: 'Select a commit' });
    const back = within(nested).getByRole('button', { name: 'Back to graph' });
    act(() => back.focus());
    await user.tab({ shift: true });
    expect(document.activeElement).toBe(within(nested).getByRole('button', { name: 'Cancel' }));
    fireEvent(nested, new Event('cancel', { bubbles: false, cancelable: true }));
    await waitFor(() => expect(nested).not.toBeInTheDocument());
    expect(count).toHaveFocus();
    expect(dialog).toBeInTheDocument();
    expect(client.createBuildCalls).toHaveLength(0);
    await user.click(within(dialog).getByRole('button', { name: 'Close' }));
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it('keeps labels compact and reveals worktree details on hover and keyboard focus without changing selection', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    const fetchGraph = vi.spyOn(client, 'branchGraph');
    renderScreen(client);
    await user.click(await screen.findByRole('button', { name: 'Select branch…' }));
    const dialog = await screen.findByRole('dialog', { name: 'Select branch' });
    const graph = await within(dialog).findByRole('group', { name: 'Repository branch graph' });
    const labels = [...graph.querySelectorAll<HTMLButtonElement>('.branch-graph__target')];
    const geometry = labels.map((label) => label.getAttribute('style'));
    expect(labels[0]).toHaveTextContent(/^codex\/durable-review$/);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    await user.hover(labels[1]);
    expect(screen.getByRole('tooltip')).toHaveTextContent(/worktree instance/);
    expect(labels[0]).toHaveAttribute('aria-pressed', 'true');
    await user.unhover(labels[1]);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    act(() => labels[0].focus());
    expect(labels[0]).toHaveFocus();
    expect(screen.getByRole('tooltip')).toHaveTextContent('codex/durable-review');
    expect(labels.map((label) => label.getAttribute('style'))).toEqual(geometry);
    expect(fetchGraph).toHaveBeenCalledTimes(1);
    expect(client.commitHistoryCalls).toHaveLength(0);
    expect(client.createBuildCalls).toHaveLength(0);
  });

  it('identifies incomplete range counts and loaded history in the commit picker', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    const original = client.branchGraph;
    client.branchGraph = async () => {
      const graph = await original();
      return {
        ...graph,
        hasMore: true,
        connections: graph.connections.map((edge) => ({ ...edge, incomplete: true })),
      };
    };
    renderScreen(client);
    await user.click(await screen.findByRole('button', { name: 'Select branch…' }));
    await user.click(await screen.findByRole('button', { name: /^2\+ commits from/ }));
    const picker = await screen.findByRole('dialog', { name: 'Select a commit' });
    expect(picker).toHaveTextContent('Showing loaded history');
    expect(picker).toHaveTextContent('load earlier relationships');
  });

  it('retains the source when a graph range has expired', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    client.commitHistory = async () => {
      throw new Error('This graph snapshot expired; reopen the branch selector.');
    };
    renderScreen(client);
    await user.click(await screen.findByRole('button', { name: 'Select branch…' }));
    const graph = await screen.findByRole('dialog', { name: 'Select branch' });
    await user.click(await within(graph).findByRole('button', { name: /^2 commits from/ }));
    const nested = await screen.findByRole('dialog', { name: 'Select a commit' });
    expect(await within(nested).findByRole('alert')).toHaveTextContent('expired');
    expect(within(nested).getByRole('button', { name: 'Use this commit' })).toBeDisabled();
    await user.click(within(nested).getByRole('button', { name: 'Cancel' }));
    expect(within(graph).getByRole('button', { name: 'Use branch' })).toBeEnabled();
    expect(client.createBuildCalls).toHaveLength(0);
  });

  it('opens a completed build through the unconditional Worktree Review client', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    const heading = await screen.findByRole('heading', { name: 'Completed review build' });
    await user.click(within(heading.closest('article')!).getByRole('button', { name: 'Launch' }));
    await waitFor(() => expect(client.openBuildCalls).toEqual([completedBuild.buildId]));
  });
});

class FixtureClient implements WorktreeReviewClient {
  readonly createBuildCalls: CreateBuildRequest[] = [];
  readonly associateCalls: AssociateWorktreeRequest[] = [];
  readonly selectRepositoryCalls: RepositoryId[] = [];
  readonly commitHistoryCalls: { cursor?: string }[] = [];
  readonly openBuildCalls: BuildId[] = [];

  constructor(
    private detail = branchDetailFixture(),
    private initialOverview: WorktreeReviewOverview = overviewFixture,
  ) {}

  overview = async (): Promise<WorktreeReviewOverview> => this.initialOverview;
  selectRepository = async (repositoryId: RepositoryId): Promise<WorktreeReviewOverview> => {
    this.selectRepositoryCalls.push(repositoryId);
    return overviewFixture;
  };
  targetDetail = async (target: ReviewTarget): Promise<BranchReviewDetail> => {
    if (target.repositoryId !== 'repository-one') throw new Error('Unknown fixture repository.');
    if (target.kind === 'worktree') {
      const worktree = [worktreeOne, worktreeTwo].find(
        (item) => item.worktreeId === target.worktreeId,
      )!;
      return branchDetailFixture({
        branch: {
          ...this.detail.branch,
          target,
          branchRef: null,
          displayName: worktree.name,
          tip: worktree.currentHead,
        },
        worktrees: [{ ...worktree, associationId: null }],
        associationCandidates: [],
        builds: [],
      });
    }
    if (target.kind === 'commit') {
      const commit = [tipCommit, earlierCommit, baseCommit].find(
        (item) => item.objectId === target.objectId,
      )!;
      return branchDetailFixture({
        branch: {
          ...this.detail.branch,
          target,
          branchRef: null,
          displayName: `Commit ${commit.abbreviatedObjectId}`,
          tip: commit,
        },
        worktrees: [],
        associationCandidates: [],
        builds: [],
      });
    }
    return target.branchRef === 'refs/heads/main'
      ? branchDetailFixture({
          branch: overviewFixture.branches[1],
          worktrees: [
            { ...worktreeOne, branchRef: target.branchRef, associationId: 'association-main' },
          ],
          associationCandidates: [],
          builds: [],
        })
      : this.detail;
  };
  commitHistory = async (
    query: CommitHistoryQuery,
    cursor?: string,
  ): Promise<CommitHistoryPage> => {
    this.commitHistoryCalls.push({ cursor });
    return {
      scope: query.scope,
      totalCount: 3,
      commits: [tipCommit, earlierCommit, baseCommit],
      nextCursor: null,
    };
  };
  worktreeActivity = async () => [];
  detachedWorktrees = async () => [];
  readBuildLog = async () => ({ text: '', nextOffset: 0, truncatedBefore: false });
  branchGraph = async (): Promise<BranchGraphData> => ({
    snapshotId: 'snapshot',
    referenceTarget: overviewFixture.branches[0].target,
    targets: overviewFixture.branches,
    anchors: [
      {
        objectId: tipCommit.objectId,
        parentIds: [baseCommit.objectId],
        boundary: false,
        merge: false,
      },
      { objectId: baseCommit.objectId, parentIds: [], boundary: false, merge: false },
    ],
    connections: [
      {
        id: 'base-to-tip',
        from: baseCommit.objectId,
        to: tipCommit.objectId,
        commitCount: 2,
        collapsed: false,
        incomplete: false,
        eligibleSources: [overviewFixture.branches[0].target],
        scope: {
          kind: 'graph_range',
          snapshotId: 'snapshot',
          rangeId: 'base-to-tip',
        },
      },
    ],
    hasMore: false,
    loadedCommitCount: 3,
  });
  associateWorktree = async (input: AssociateWorktreeRequest): Promise<AssociatedWorktree> => {
    this.associateCalls.push(input);
    return {
      ...worktreeTwo,
      associationId: 'association-candidate',
      worktreeId: input.worktreeId,
      name: 'Detached investigation',
    };
  };
  createWorktree = async (): Promise<AssociatedWorktree> => ({
    ...worktreeOne,
    associationId: 'association-created',
    worktreeId: 'worktree-created',
    name: 'Created checkout',
    ownership: 'managed_branch_worktree',
    provenance: 'worktree_review_created',
  });
  createBuild = async (input: CreateBuildRequest): Promise<ReviewBuild> => {
    this.createBuildCalls.push(input);
    return {
      ...completedBuild,
      buildId: `build-${this.createBuildCalls.length}`,
      name: input.name,
      source: sourceReceipt(input),
    };
  };
  openBuild = async ({
    buildId,
  }: {
    readonly buildId: BuildId;
  }): Promise<OpenBuildOutcome> => {
    this.openBuildCalls.push(buildId);
    return { outcome: 'launched' };
  };
}

class FixtureCatalog implements RepositoryCatalogClient {
  readonly registerDirectoryCalls: string[] = [];
  readonly registerCodexCalls: RepositoryId[] = [];

  overview = async (): Promise<RepositoryCatalogOverview> => ({
    codex: { state: 'ready', message: 'Directories from Codex tasks are available.' },
    github: {
      state: 'connected',
      login: 'fixture-user',
      message: 'GitHub CLI login is available.',
    },
    repositories: [
      {
        catalogId: 'github-one',
        name: 'fixture/codex-orchestrator',
        github: {
          repositoryId: 'github-one',
          nameWithOwner: 'fixture/codex-orchestrator',
          visibility: 'private',
          webUrl: 'https://github.com/fixture/codex-orchestrator',
        },
        localInstances: [
          {
            repositoryId: 'repository-one',
            name: 'Codex Orchestrator',
            locationLabel: 'C:\\Projects\\Codex Orchestrator',
            registered: false,
            disclosures: ['codex_task'],
          },
        ],
      },
      {
        catalogId: 'github-remote-only',
        name: 'remote/only',
        github: {
          repositoryId: 'github-remote-only',
          nameWithOwner: 'remote/only',
          visibility: 'public',
          webUrl: 'https://github.com/remote/only',
        },
        localInstances: [],
      },
    ],
  });
  registerDirectory = async (repositoryRoot: string): Promise<RegisteredRepository> => {
    this.registerDirectoryCalls.push(repositoryRoot);
    return registeredRepository;
  };
  registerCodexRepository = async (repositoryId: RepositoryId): Promise<RegisteredRepository> => {
    this.registerCodexCalls.push(repositoryId);
    return registeredRepository;
  };
  listWorktreeTargets = async () => [];
}

const registeredRepository: RegisteredRepository = {
  repositoryId: 'repository-one',
  name: 'Codex Orchestrator',
  locationLabel: 'C:\\Projects\\Codex Orchestrator',
};

function renderScreen(
  client: WorktreeReviewClient,
  repositoryCatalog: RepositoryCatalogClient = new FixtureCatalog(),
  unreadBuilds: readonly ReviewBuild[] = [],
  onMarkBuildsRead?: (buildIds: readonly BuildId[]) => void,
) {
  return render(
    <WorktreeReviewScreen
      client={client}
      repositoryCatalog={repositoryCatalog}
      unreadBuilds={unreadBuilds}
      onMarkBuildsRead={onMarkBuildsRead}
    />,
  );
}

function sourceReceipt(input: CreateBuildRequest): ReviewBuildSource {
  switch (input.source.kind) {
    case 'existing_worktree':
      return {
        ...input.source,
        triggerHeadObjectId: tipCommit.objectId,
        triggerVirtualCommitId: '3333333333333333333333333333333333333333',
      };
    case 'worktree_snapshot':
      return {
        ...input.source,
        headObjectId: tipCommit.objectId,
        capturedObjectId: '3333333333333333333333333333333333333333',
        virtualCommitId: '3333333333333333333333333333333333333333',
      };
    case 'exact_commit':
    case 'branch_commit':
      return input.source;
    case 'physical_worktree':
      return { ...input.source, capturedObjectId: input.source.headObjectId };
  }
}
