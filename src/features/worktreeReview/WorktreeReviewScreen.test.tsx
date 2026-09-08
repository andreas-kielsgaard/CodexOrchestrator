import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type {
  RegisteredRepository,
  RepositoryCatalogClient,
  RepositoryCatalogOverview,
} from '../../application/repositoryCatalog';
import type {
  AssociateWorktreeRequest,
  AssociatedWorktree,
  BranchRef,
  BranchHistoryPage,
  BranchReviewDetail,
  BuildId,
  CreateBuildRequest,
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

describe('WorktreeReviewScreen', () => {
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
    expect(second).toBeChecked();
    expect(screen.getByText('Detached HEAD')).toBeVisible();

    const branchButton = screen.getByRole('button', { name: /main/ });
    await user.click(branchButton);
    expect(branchButton).toHaveFocus();
    expect(await screen.findByRole('heading', { name: 'main' })).toBeVisible();
  });

  it('records an exact worktree snapshot and discloses its retained owned checkout', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    await screen.findByRole('heading', { name: 'codex/durable-review' });

    await user.click(screen.getByRole('radio', { name: /Agent checkout B/ }));
    await user.click(screen.getByRole('radio', { name: /Snapshot current work/ }));
    expect(screen.getByRole('note', { name: 'Worktree change' })).toHaveTextContent(
      'create and retain an isolated Worktree checkout',
    );
    expect(screen.getByRole('note', { name: 'Worktree change' })).toHaveTextContent(
      'must create a virtual commit before the build can continue',
    );

    const name = screen.getByRole('textbox', { name: 'Build name' });
    await user.clear(name);
    await user.type(name, 'Snapshot B');
    await user.click(screen.getByRole('button', { name: 'Create build' }));

    await waitFor(() => expect(client.createBuildCalls).toHaveLength(1));
    expect(client.createBuildCalls[0]).toMatchObject({
      name: 'Snapshot B',
      source: {
        kind: 'worktree_snapshot',
        associationId: worktreeTwo.associationId,
      },
      workspacePlan: {
        kind: 'create_owned_build_worktree',
        originatingAssociationId: worktreeTwo.associationId,
      },
    });
    expect(await screen.findByText(/Created Snapshot B/)).toHaveAttribute('role', 'status');
  });

  it('allows Create Build without a pre-existing worktree and makes automatic creation explicit', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient(
      branchDetailFixture({ worktrees: [], associationCandidates: [], builds: [] }),
    );
    renderScreen(client);

    expect(await screen.findByText(/No associated worktree exists/)).toBeVisible();
    expect(screen.getByRole('radio', { name: /Specific branch commit/ })).toBeChecked();
    expect(screen.getByRole('note', { name: 'Worktree change' })).toHaveTextContent(
      'first create and retain a Worktree checkout for this branch',
    );
    await user.click(screen.getByRole('button', { name: 'Create build' }));
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

    expect(screen.getByText('Checkouts requiring association')).toBeVisible();
    expect(
      screen.getByText('Detached investigation').closest('.worktree-review__candidate'),
    ).toHaveTextContent('Detached HEAD');
    expect(client.branchHistoryCalls).toHaveLength(0);
    await user.click(screen.getByRole('button', { name: 'Choose a specific baseline' }));
    await waitFor(() => expect(client.branchHistoryCalls).toHaveLength(1));
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

  it('presents compilation outcome, retained output, and cleanup as independent durable facts', async () => {
    const client = new FixtureClient();
    renderScreen(client);
    const builds = await screen.findByRole('heading', { name: 'Builds' });
    const region = builds.closest('section')!;
    const failed = within(region)
      .getByRole('heading', { name: 'Failed rebuild' })
      .closest('article')!;
    expect(failed).toHaveTextContent('Compilation failed');
    expect(failed).toHaveTextContent('No retained build output');
    expect(failed).toHaveTextContent('Eligible for cleanup');
    expect(failed).toHaveTextContent('Compiler exited with code 1');

    const completed = within(region)
      .getByRole('heading', { name: 'Completed review build' })
      .closest('article')!;
    expect(completed).toHaveTextContent('Compilation completed');
    expect(completed).toHaveTextContent('Available · AppData / build-output / build-completed');
    expect(completed).toHaveTextContent('Newest successful build for this source');
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
    expect(screen.getByRole('radio', { name: /Live Worktree checkout/ })).toBeDisabled();
    expect(screen.getByRole('radio', { name: /Snapshot current work/ })).toBeDisabled();
    expect(screen.getByRole('radio', { name: /Specific branch commit/ })).toBeChecked();

    await user.click(screen.getByRole('radio', { name: /Agent checkout B/ }));
    expect(screen.getByRole('radio', { name: /Live Worktree checkout/ })).toBeEnabled();
    expect(screen.getByRole('radio', { name: /Snapshot current work/ })).toBeEnabled();
  });

  it('loads paginated branch history only when the specific commit picker is opened', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    await screen.findByRole('heading', { name: 'codex/durable-review' });

    expect(client.branchHistoryCalls).toHaveLength(0);
    await user.click(screen.getByRole('radio', { name: /Specific branch commit/ }));
    expect(client.branchHistoryCalls).toHaveLength(0);
    await user.click(screen.getByRole('button', { name: 'Choose another commit' }));
    await waitFor(() => expect(client.branchHistoryCalls).toEqual([{ cursor: undefined }]));
    expect(screen.getByRole('combobox', { name: 'Branch commit' })).toHaveTextContent(
      earlierCommit.subject,
    );
  });

  it('opens a completed build through the unconditional Worktree Review client', async () => {
    const user = userEvent.setup();
    const client = new FixtureClient();
    renderScreen(client);
    const heading = await screen.findByRole('heading', { name: 'Completed review build' });
    await user.click(within(heading.closest('article')!).getByRole('button', { name: 'Open' }));
    await waitFor(() => expect(client.openBuildCalls).toEqual([completedBuild.buildId]));
  });
});

class FixtureClient implements WorktreeReviewClient {
  readonly createBuildCalls: CreateBuildRequest[] = [];
  readonly associateCalls: AssociateWorktreeRequest[] = [];
  readonly selectRepositoryCalls: RepositoryId[] = [];
  readonly branchHistoryCalls: { cursor?: string }[] = [];
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
  branchDetail = async (
    repositoryId: RepositoryId,
    branchRef: BranchRef,
  ): Promise<BranchReviewDetail> => {
    if (repositoryId !== 'repository-one') throw new Error('Unknown fixture repository.');
    return branchRef === 'refs/heads/main'
      ? branchDetailFixture({
          branch: overviewFixture.branches[1],
          worktrees: [{ ...worktreeOne, branchRef, associationId: 'association-main' }],
          associationCandidates: [],
          builds: [],
        })
      : this.detail;
  };
  branchHistory = async (
    _repositoryId: RepositoryId,
    _branchRef: BranchRef,
    cursor?: string,
  ): Promise<BranchHistoryPage> => {
    this.branchHistoryCalls.push({ cursor });
    return { commits: [tipCommit, earlierCommit, baseCommit] };
  };
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
  openBuild = async ({ buildId }: { readonly buildId: BuildId }): Promise<void> => {
    this.openBuildCalls.push(buildId);
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
) {
  return render(<WorktreeReviewScreen client={client} repositoryCatalog={repositoryCatalog} />);
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
    case 'branch_commit':
      return input.source;
  }
}
