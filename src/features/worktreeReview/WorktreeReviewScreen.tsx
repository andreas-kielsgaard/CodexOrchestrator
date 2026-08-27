import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  AssociateWorktreeRequest,
  BranchRef,
  BranchReviewDetail,
  BuildId,
  CreateBuildRequest,
  GitCommit,
  RepositoryId,
  WorktreeAssociationCandidate,
  WorktreeAssociationId,
  WorktreeReviewClient,
  WorktreeReviewOverview,
} from '../../application/worktreeReview';
import { ProductViewHeader } from '../shared/ProductViewHeader';
import { BranchNavigator } from './BranchNavigator';
import { BuildComposer } from './BuildComposer';
import { BuildHistory } from './BuildHistory';
import { ReadinessNotice } from './ReadinessNotice';
import { WorktreeSelector } from './WorktreeSelector';
import './worktreeReview.css';

export function WorktreeReviewScreen({ client }: { readonly client: WorktreeReviewClient }) {
  const [overview, setOverview] = useState<WorktreeReviewOverview>({
    repositories: [],
    branches: [],
  });
  const [repositoryId, setRepositoryId] = useState<RepositoryId | ''>('');
  const [branchRef, setBranchRef] = useState<BranchRef | ''>('');
  const [detail, setDetail] = useState<BranchReviewDetail | null>(null);
  const [associationId, setAssociationId] = useState<WorktreeAssociationId | ''>('');
  const [repositoryRoot, setRepositoryRoot] = useState('');
  const [busy, setBusy] = useState<string | null>('loading');
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [history, setHistory] = useState<{
    readonly branchRef: BranchRef;
    readonly commits: readonly GitCommit[];
    readonly nextCursor?: string;
  } | null>(null);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [openingBuildId, setOpeningBuildId] = useState<BuildId | undefined>();
  const branchRequest = useRef(0);
  const historyRequest = useRef(0);

  const loadBranch = useCallback(
    async (nextRepositoryId: RepositoryId, nextBranchRef: BranchRef) => {
      const request = ++branchRequest.current;
      setBusy('branch');
      setError(null);
      setNotice(null);
      setDetail(null);
      setHistory(null);
      historyRequest.current += 1;
      try {
        const value = await client.branchDetail(nextRepositoryId, nextBranchRef);
        if (request !== branchRequest.current) return;
        setDetail(value);
        setAssociationId((current) => preferredAssociation(value, current));
      } catch (cause) {
        if (request === branchRequest.current) setError(message(cause));
      } finally {
        if (request === branchRequest.current) setBusy(null);
      }
    },
    [client],
  );

  useEffect(() => {
    let active = true;
    async function initialize() {
      setBusy('loading');
      try {
        const initial = await client.overview();
        if (!active) return;
        const selected =
          initial.repositories.find(
            (repository) => repository.repositoryId === initial.selectedRepositoryId,
          )?.repositoryId ??
          initial.repositories[0]?.repositoryId ??
          '';
        const resolved =
          selected && initial.selectedRepositoryId !== selected
            ? await client.selectRepository(selected)
            : initial;
        if (!active) return;
        setOverview(resolved);
        setRepositoryId(selected);
        const firstBranch = resolved.branches[0]?.branchRef ?? '';
        setBranchRef(firstBranch);
        if (selected && firstBranch) await loadBranch(selected, firstBranch);
        else setBusy(null);
      } catch (cause) {
        if (active) {
          setError(message(cause));
          setBusy(null);
        }
      }
    }
    void initialize();
    return () => {
      active = false;
      branchRequest.current += 1;
    };
  }, [client, loadBranch]);

  const selectedRepository = overview.repositories.find(
    (repository) => repository.repositoryId === repositoryId,
  );
  const selectedWorktree = detail?.worktrees.find(
    (worktree) =>
      worktree.associationId === associationId && worktree.availability.state === 'available',
  );
  const canCreateWorktree = selectedRepository?.readiness.createWorktree.state === 'available';
  const canBuild = selectedRepository?.readiness.build.state === 'available';
  const activeMutation = busy !== null && busy !== 'branch' && busy !== 'loading';

  async function changeRepository(nextRepositoryId: RepositoryId) {
    branchRequest.current += 1;
    setBusy('repository');
    setError(null);
    setNotice(null);
    setDetail(null);
    setAssociationId('');
    try {
      const value = await client.selectRepository(nextRepositoryId);
      setOverview(value);
      setRepositoryId(nextRepositoryId);
      const nextBranch = value.branches[0]?.branchRef ?? '';
      setBranchRef(nextBranch);
      if (nextBranch) await loadBranch(nextRepositoryId, nextBranch);
      else setBusy(null);
    } catch (cause) {
      setError(message(cause));
      setBusy(null);
    }
  }

  async function connectRepository() {
    if (!client.connectRepository || !repositoryRoot.trim()) return;
    setBusy('connect-repository');
    setError(null);
    setNotice(null);
    try {
      const value = await client.connectRepository(repositoryRoot.trim());
      const selected = value.selectedRepositoryId ?? value.repositories[0]?.repositoryId ?? '';
      setOverview(value);
      setRepositoryId(selected);
      const nextBranch = value.branches[0]?.branchRef ?? '';
      setBranchRef(nextBranch);
      setRepositoryRoot('');
      if (selected && nextBranch) await loadBranch(selected, nextBranch);
      else setBusy(null);
    } catch (cause) {
      setError(message(cause));
      setBusy(null);
    }
  }

  function changeBranch(nextBranchRef: BranchRef) {
    if (!repositoryId || nextBranchRef === branchRef) return;
    setBranchRef(nextBranchRef);
    setAssociationId('');
    void loadBranch(repositoryId, nextBranchRef);
  }

  async function createWorktree() {
    if (!repositoryId || !branchRef || !detail) return;
    setBusy('create-worktree');
    setError(null);
    try {
      const worktree = await client.createWorktree({ repositoryId, branchRef });
      setDetail({
        ...detail,
        branch: {
          ...detail.branch,
          associatedWorktreeCount: detail.branch.associatedWorktreeCount + 1,
        },
        worktrees: [...detail.worktrees, worktree],
      });
      setAssociationId(worktree.associationId);
      setNotice(`Created and selected ${worktree.name}.`);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }

  async function associateWorktree(
    candidate: WorktreeAssociationCandidate,
    baseline: AssociateWorktreeRequest['baseline'],
  ) {
    if (!repositoryId || !branchRef || !detail) return;
    setBusy(`associate:${candidate.worktreeId}`);
    setError(null);
    try {
      const worktree = await client.associateWorktree({
        repositoryId,
        branchRef,
        worktreeId: candidate.worktreeId,
        baseline,
      });
      setDetail({
        ...detail,
        branch: {
          ...detail.branch,
          associatedWorktreeCount: detail.branch.associatedWorktreeCount + 1,
        },
        worktrees: [...detail.worktrees, worktree],
        associationCandidates: detail.associationCandidates.filter(
          (item) => item.worktreeId !== candidate.worktreeId,
        ),
      });
      setAssociationId(worktree.associationId);
      setNotice(`Associated and selected ${worktree.name}.`);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }

  async function createBuild(request: CreateBuildRequest) {
    if (!detail) return;
    setBusy('create-build');
    setError(null);
    try {
      const build = await client.createBuild(request);
      setDetail({ ...detail, builds: [build, ...detail.builds] });
      setNotice(`Created ${build.name}. Its attempt and retained output are tracked independently.`);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }

  async function loadHistory(cursor?: string) {
    if (!repositoryId || !branchRef || historyLoading) return;
    if (!cursor && history?.branchRef === branchRef && history.commits.length > 0) return;
    const request = ++historyRequest.current;
    setHistoryLoading(true);
    setError(null);
    try {
      const page = await client.branchHistory(repositoryId, branchRef, cursor);
      if (request !== historyRequest.current) return;
      setHistory((current) => ({
        branchRef,
        commits: uniqueCommits([
          ...(cursor && current?.branchRef === branchRef ? current.commits : []),
          ...page.commits,
        ]),
        nextCursor: page.nextCursor,
      }));
    } catch (cause) {
      if (request === historyRequest.current) setError(message(cause));
    } finally {
      if (request === historyRequest.current) setHistoryLoading(false);
    }
  }

  async function openBuild(buildId: BuildId) {
    setOpeningBuildId(buildId);
    setError(null);
    try {
      await client.openBuild({ buildId });
      setNotice(`Opened build ${buildId}.`);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setOpeningBuildId(undefined);
    }
  }

  async function refresh() {
    if (!repositoryId || !branchRef) return;
    await loadBranch(repositoryId, branchRef);
  }

  return (
    <main className="worktree-review" aria-label="Worktree Review">
      <ProductViewHeader
        context="Repository builds"
        title="Worktree Review"
        actions={
          <button
            type="button"
            className="worktree-review__secondary"
            disabled={!repositoryId || !branchRef || busy !== null}
            onClick={() => void refresh()}
          >
            Refresh facts
          </button>
        }
      />
      <div className="worktree-review__layout">
        <BranchNavigator
          repositories={overview.repositories}
          selectedRepositoryId={repositoryId}
          branches={overview.branches}
          selectedBranchRef={branchRef}
          disabled={busy !== null}
          onRepositoryChange={(value) => void changeRepository(value)}
          onBranchChange={changeBranch}
        />

        <div
          className="worktree-review__content"
          aria-busy={busy === 'branch' || busy === 'loading'}
        >
          {overview.repositories.length === 0 && busy === null && (
            <section
              className="worktree-review__connect"
              aria-labelledby="connect-repository-title"
            >
              <div>
                <h2 id="connect-repository-title">Connect a Git repository</h2>
                <p>
                  Worktree Review verifies the repository and stores its stable identity before
                  showing branches. The path is not used as durable operation authority.
                </p>
              </div>
              <form
                onSubmit={(event) => {
                  event.preventDefault();
                  void connectRepository();
                }}
              >
                <label className="worktree-review__field">
                  <span>Repository root</span>
                  <input
                    aria-label="Repository root"
                    value={repositoryRoot}
                    placeholder="C:\\Projects\\Repository"
                    disabled={!client.connectRepository || busy !== null}
                    onChange={(event) => setRepositoryRoot(event.target.value)}
                  />
                </label>
                <button
                  type="submit"
                  className="worktree-review__primary"
                  disabled={!client.connectRepository || !repositoryRoot.trim() || busy !== null}
                >
                  Verify repository
                </button>
              </form>
            </section>
          )}
          {selectedRepository && <ReadinessNotice repository={selectedRepository} />}
          {overview.activeBuildContext && (
            <section className="worktree-review__active-context" aria-label="Active build context">
              <strong>Active build context</strong>
              <p>
                The application is running from this build checkout. Its exact worktree is marked
                below and cannot be used as a direct or snapshot source for another build.
              </p>
              <dl>
                <div>
                  <dt>Build ID</dt>
                  <dd>{overview.activeBuildContext.buildId}</dd>
                </div>
                <div>
                  <dt>Worktree ID</dt>
                  <dd>{overview.activeBuildContext.worktreeId}</dd>
                </div>
              </dl>
            </section>
          )}
          {error && (
            <div className="worktree-review__alert" role="alert">
              <strong>Worktree Review could not complete the action.</strong>
              <span>{error}</span>
            </div>
          )}
          {notice && (
            <p className="worktree-review__success" role="status">
              {notice}
            </p>
          )}

          {(busy === 'loading' || busy === 'branch') && !detail && (
            <p className="worktree-review__loading" role="status">
              Loading branch facts…
            </p>
          )}
          {!branchRef && busy === null && (
            <p className="worktree-review__empty">Select a repository with at least one branch.</p>
          )}
          {detail && (
            <>
              <BranchSummary detail={detail} />
              <WorktreeSelector
                worktrees={detail.worktrees}
                candidates={detail.associationCandidates}
                selectedAssociationId={associationId}
                activeWorktreeId={overview.activeBuildContext?.worktreeId}
                historyCommits={history?.branchRef === branchRef ? history.commits : []}
                historyLoading={historyLoading}
                hasMoreHistory={Boolean(history?.branchRef === branchRef && history.nextCursor)}
                createAvailable={canCreateWorktree}
                busy={activeMutation}
                onSelect={setAssociationId}
                onCreate={() => void createWorktree()}
                onAssociate={(candidate, baseline) => void associateWorktree(candidate, baseline)}
                onRequestHistory={() => void loadHistory()}
                onLoadMoreHistory={() => void loadHistory(history?.nextCursor)}
              />
              <BuildComposer
                key={detail.branch.branchRef}
                detail={detail}
                selectedWorktree={selectedWorktree}
                activeWorktreeId={overview.activeBuildContext?.worktreeId}
                commits={history?.branchRef === branchRef ? history.commits : []}
                historyLoading={historyLoading}
                hasMoreHistory={Boolean(history?.branchRef === branchRef && history.nextCursor)}
                buildAvailable={canBuild}
                disabled={activeMutation}
                creating={busy === 'create-build'}
                onCreate={(request) => void createBuild(request)}
                onRequestHistory={() => void loadHistory()}
                onLoadMoreHistory={() => void loadHistory(history?.nextCursor)}
              />
              <BuildHistory
                builds={detail.builds}
                openingBuildId={openingBuildId}
                onOpen={(buildId) => void openBuild(buildId)}
              />
            </>
          )}
        </div>
      </div>
    </main>
  );
}

function BranchSummary({ detail }: { readonly detail: BranchReviewDetail }) {
  return (
    <section
      className="worktree-review__branch-summary"
      aria-labelledby="worktree-review-selected-branch"
    >
      <p className="worktree-review__step">1 · Selected branch</p>
      <div className="worktree-review__branch-title">
        <div>
          <h2 id="worktree-review-selected-branch">{detail.branch.displayName}</h2>
          <p>
            <code>{detail.branch.tip.abbreviatedObjectId}</code> · {detail.branch.tip.subject}
          </p>
        </div>
        <span>
          {detail.branch.aheadOfDefault} ahead · {detail.branch.behindDefault} behind default
        </span>
      </div>
    </section>
  );
}

function preferredAssociation(
  detail: BranchReviewDetail,
  current: WorktreeAssociationId | '',
): WorktreeAssociationId | '' {
  if (
    current &&
    detail.worktrees.some(
      (worktree) =>
        worktree.associationId === current && worktree.availability.state === 'available',
    )
  ) {
    return current;
  }
  return (
    detail.worktrees.find((worktree) => worktree.availability.state === 'available')
      ?.associationId ?? ''
  );
}

function message(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function uniqueCommits(commits: readonly GitCommit[]): readonly GitCommit[] {
  return commits.filter(
    (commit, index) =>
      commits.findIndex((candidate) => candidate.objectId === commit.objectId) === index,
  );
}
