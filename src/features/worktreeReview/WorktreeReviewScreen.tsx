import { useCallback, useMemo, useRef, useState } from 'react';
import type {
  RegisteredRepository,
  RepositoryCatalogClient,
} from '../../application/repositoryCatalog';
import {
  commitTarget,
  commitSourceContext,
  type AssociateWorktreeRequest,
  type BranchReviewDetail,
  type BuildId,
  type CreateBuildRequest,
  type ReviewTarget,
  type WorktreeAssociationCandidate,
  type WorktreeReviewClient,
} from '../../application/worktreeReview';
import { ProductViewHeader } from '../shared/ProductViewHeader';
import { BranchNavigator } from './BranchNavigator';
import { BuildComposer } from './BuildComposer';
import { BuildHistory } from './BuildHistory';
import { ReadinessNotice } from './ReadinessNotice';
import { RepositoryRegistrationModal } from './RepositoryRegistrationModal';
import { WorktreeSelector } from './WorktreeSelector';
import { useReviewSelection } from './useReviewSelection';
import { useCommitHistory } from './useCommitHistory';
import { initialBuildDraft, requiresCheckout, type BuildDraft } from './buildDraft';
import { BuildCheckoutDialog } from './BuildCheckoutDialog';
import { BranchGraphDialog } from './branchSelection/BranchGraphDialog';
import './worktreeReview.css';

export function WorktreeReviewScreen({
  client,
  repositoryCatalog,
}: {
  readonly client: WorktreeReviewClient;
  readonly repositoryCatalog: RepositoryCatalogClient;
}) {
  const [draft, setDraft] = useState<BuildDraft | null>(null);
  const [selectedWorktreeId, setSelectedWorktreeId] = useState('');
  const onSelected = useCallback((detail: BranchReviewDetail, activeWorktreeId?: string) => {
    setDraft(initialBuildDraft(detail, activeWorktreeId));
    setSelectedWorktreeId(
      detail.worktrees.find(
        (worktree) =>
          worktree.availability.state === 'available' && worktree.worktreeId !== activeWorktreeId,
      )?.worktreeId ??
        detail.worktrees[0]?.worktreeId ??
        '',
    );
  }, []);
  const selection = useReviewSelection(client, onSelected);
  const { overview, branches, target, detail, loading } = selection;
  const repositoryId = overview.selectedRepositoryId ?? '';
  const selectedRepository = overview.repositories.find(
    (repository) => repository.repositoryId === repositoryId,
  );
  const [registrationOpen, setRegistrationOpen] = useState(false);
  const refreshTrigger = useRef<HTMLButtonElement>(null);
  const graphTrigger = useRef<HTMLButtonElement>(null);
  const [graphOpen, setGraphOpen] = useState(false);
  const [checkoutRequest, setCheckoutRequest] = useState<CreateBuildRequest | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [openingBuildId, setOpeningBuildId] = useState<BuildId>();
  const disabled = loading || busy !== null;
  const selectedWorktree = detail?.worktrees.find(
    (worktree) =>
      worktree.worktreeId === selectedWorktreeId && worktree.availability.state === 'available',
  );
  const canCreateWorktree =
    selectedRepository?.readiness.createWorktree.state === 'available' && target?.kind === 'branch';
  const canBuild =
    selectedRepository?.readiness.build.state === 'available' &&
    selectedRepository?.readiness.buildOutputStorage.state === 'available';
  const query = useMemo(
    () =>
      detail
        ? {
            target: detail.branch.target,
            scope: {
              kind: 'ancestry' as const,
              tipObjectId: commitSourceContext(detail.branch).tipObjectId,
            },
          }
        : null,
    [detail],
  );
  const history = useCommitHistory(client, query);

  async function selectTarget(target: ReviewTarget) {
    setError(null);
    setNotice(null);
    await selection.selectTarget(target);
  }
  async function changeRepository(id: string) {
    setError(null);
    setNotice(null);
    await selection.selectRepository(id);
  }
  async function registeredRepository(repository: RegisteredRepository) {
    setRegistrationOpen(false);
    await changeRepository(repository.repositoryId);
  }
  async function refresh() {
    setError(null);
    const button = refreshTrigger.current;
    let restore = document.activeElement === button;
    const moved = (event: Event) => {
      if (event.target !== button && event.target !== document.body) restore = false;
    };
    document.addEventListener('focusin', moved);
    document.addEventListener('pointerdown', moved);
    try {
      await selection.refresh();
    } finally {
      requestAnimationFrame(() => {
        document.removeEventListener('focusin', moved);
        document.removeEventListener('pointerdown', moved);
        if (
          restore &&
          button?.isConnected &&
          !button.disabled &&
          document.activeElement === document.body
        )
          button.focus();
      });
    }
  }
  async function mutate(label: string, action: () => Promise<unknown>, success: string) {
    setBusy(label);
    setError(null);
    setNotice(null);
    try {
      await action();
      await selection.refresh();
      setNotice(success);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }
  async function createWorktree() {
    if (target?.kind !== 'branch') return;
    await mutate(
      'create-worktree',
      () => client.createWorktree({ repositoryId, branchRef: target.branchRef }),
      'Worktree created.',
    );
  }
  async function associateWorktree(
    candidate: WorktreeAssociationCandidate,
    baseline: AssociateWorktreeRequest['baseline'],
  ) {
    if (target?.kind !== 'branch') return;
    await mutate(
      'associate-worktree',
      () =>
        client.associateWorktree({
          repositoryId,
          branchRef: target.branchRef,
          worktreeId: candidate.worktreeId,
          baseline,
        }),
      'Worktree associated.',
    );
  }
  function requestBuild(request: CreateBuildRequest) {
    if (requiresCheckout(request)) setCheckoutRequest(request);
    else void createBuild(request);
  }
  async function createBuild(request: CreateBuildRequest) {
    await mutate('create-build', () => client.createBuild(request), 'Build result recorded.');
  }
  async function openBuild(buildId: BuildId) {
    setOpeningBuildId(buildId);
    setError(null);
    try {
      await client.openBuild({ buildId });
      setNotice('Opened build.');
    } catch (cause) {
      setError(message(cause));
    } finally {
      setOpeningBuildId(undefined);
    }
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
            disabled={!repositoryId || !target || disabled}
            ref={refreshTrigger}
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
          branches={branches}
          selectedTarget={target}
          onSelectBranch={() => setGraphOpen(true)}
          graphTriggerRef={graphTrigger}
          disabled={disabled}
          onRepositoryChange={(value) => void changeRepository(value)}
          onRegisterRepository={() => setRegistrationOpen(true)}
          onBranchChange={(target) => void selectTarget(target)}
        />

        <div className="worktree-review__content" aria-busy={loading}>
          {overview.repositories.length === 0 && !disabled && (
            <section className="worktree-review__connect" aria-labelledby="no-repositories-title">
              <div>
                <h2 id="no-repositories-title">No repositories registered</h2>
                <p>
                  Use Add repository to choose a local directory or inspect repositories exposed by
                  Codex and the active GitHub CLI login.
                </p>
              </div>
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
          {(error || selection.error || history.error) && (
            <div className="worktree-review__alert" role="alert">
              <strong>Worktree Review could not complete the action.</strong>
              <span>{error || selection.error || history.error}</span>
            </div>
          )}
          {notice && (
            <p className="worktree-review__success" role="status">
              {notice}
            </p>
          )}

          {loading && !detail && (
            <p className="worktree-review__loading" role="status">
              Loading review facts…
            </p>
          )}
          {!target && !disabled && (
            <p className="worktree-review__empty">
              Select a repository with at least one branch or instantiated worktree.
            </p>
          )}
          {detail && (
            <>
              <BranchSummary detail={detail} />
              <WorktreeSelector
                worktrees={detail.worktrees}
                candidates={detail.associationCandidates}
                selectedWorktreeId={selectedWorktreeId}
                activeWorktreeId={overview.activeBuildContext?.worktreeId}
                historyCommits={history.commits}
                historyLoading={history.loading}
                hasMoreHistory={history.hasMore}
                createAvailable={canCreateWorktree}
                busy={disabled}
                onSelect={(worktreeId) =>
                  void selectTarget({ kind: 'worktree', repositoryId, worktreeId })
                }
                onCreate={() => void createWorktree()}
                onAssociate={(candidate, baseline) => void associateWorktree(candidate, baseline)}
                onRequestHistory={() => void history.load()}
                onLoadMoreHistory={() => void history.loadMore()}
              />
              {draft && (
                <BuildComposer
                  draft={draft!}
                  onDraftChange={setDraft}
                  onSelectCommit={(commit) =>
                    void selectTarget(commitTarget(detail.branch, commit.objectId))
                  }
                  detail={detail}
                  selectedWorktree={selectedWorktree}
                  activeWorktreeId={overview.activeBuildContext?.worktreeId}
                  commits={history.commits}
                  historyLoading={history.loading}
                  hasMoreHistory={history.hasMore}
                  buildAvailable={canBuild}
                  disabled={disabled}
                  creating={busy === 'create-build'}
                  onCreate={requestBuild}
                  onRequestHistory={() => void history.load()}
                  onLoadMoreHistory={() => void history.loadMore()}
                />
              )}
              <BuildHistory
                builds={detail.builds}
                openingBuildId={openingBuildId}
                onOpen={(buildId) => void openBuild(buildId)}
              />
            </>
          )}
        </div>
      </div>
      {graphOpen && (
        <BranchGraphDialog
          client={client}
          repositoryId={repositoryId}
          selectedTarget={target}
          onClose={() => setGraphOpen(false)}
          onSelect={(target) => {
            setGraphOpen(false);
            void selectTarget(target).then(() => {
              requestAnimationFrame(() => graphTrigger.current?.focus());
            });
          }}
        />
      )}
      {checkoutRequest && (
        <BuildCheckoutDialog
          request={checkoutRequest}
          onClose={() => setCheckoutRequest(null)}
          onConfirm={() => {
            const request = checkoutRequest;
            setCheckoutRequest(null);
            void createBuild(request);
          }}
        />
      )}
      {registrationOpen && (
        <RepositoryRegistrationModal
          catalog={repositoryCatalog}
          onClose={() => setRegistrationOpen(false)}
          onRegistered={(repository) => void registeredRepository(repository)}
        />
      )}
    </main>
  );
}

function BranchSummary({ detail }: { readonly detail: BranchReviewDetail }) {
  return (
    <section
      className="worktree-review__branch-summary"
      aria-labelledby="worktree-review-selected-branch"
    >
      <p className="worktree-review__step">1 · Selected source</p>
      <div className="worktree-review__branch-title">
        <div>
          <h2 id="worktree-review-selected-branch">{detail.branch.displayName}</h2>
          <p>
            <code>{detail.branch.tip.abbreviatedObjectId}</code> · {detail.branch.tip.subject}
          </p>
        </div>
        {detail.branch.target.kind === 'branch' && (
          <span>
            {detail.branch.aheadOfDefault} ahead · {detail.branch.behindDefault} behind default
          </span>
        )}
      </div>
    </section>
  );
}

function message(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
