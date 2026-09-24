import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  RegisteredRepository,
  RepositoryCatalogClient,
} from '../../application/repositoryCatalog';
import {
  commitSourceContext,
  type AssociateWorktreeRequest,
  type BranchReviewDetail,
  type DetachedWorktreeDetail,
  type BuildId,
  type ReviewBuild,
  type CreateBuildRequest,
  type ReviewTarget,
  type WorktreeAssociationCandidate,
  type WorktreeReviewClient,
} from '../../application/worktreeReview';
import { ProductViewHeader } from '../shared/ProductViewHeader';
import { BranchNavigator } from './BranchNavigator';
import { DetachedWorktreeList } from './DetachedWorktreeList';
import { BuildHistory } from './BuildHistory';
import { CreateBuildDialog } from './CreateBuildDialog';
import { ReadinessNotice } from './ReadinessNotice';
import { RepositoryRegistrationModal } from './RepositoryRegistrationModal';
import { WorktreeSelector } from './WorktreeSelector';
import { useReviewSelection } from './useReviewSelection';
import { useCommitHistory } from './useCommitHistory';
import { initialBuildDraft, type BuildDraft } from './buildDraft';
import { BranchGraphDialog } from './branchSelection/BranchGraphDialog';
import './worktreeReview.css';

export function WorktreeReviewScreen({
  client,
  repositoryCatalog,
  unreadBuilds = [],
  onMarkBuildsRead,
}: {
  readonly client: WorktreeReviewClient;
  readonly repositoryCatalog: RepositoryCatalogClient;
  readonly unreadBuilds?: readonly ReviewBuild[];
  readonly onMarkBuildsRead?: (buildIds: readonly BuildId[]) => void;
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
  const buildTrigger = useRef<HTMLButtonElement>(null);
  const [graphOpen, setGraphOpen] = useState(false);
  const [buildDialogOpen, setBuildDialogOpen] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [showDetached, setShowDetached] = useState(false);
  const [detached, setDetached] = useState<readonly DetachedWorktreeDetail[]>([]);
  const [detachedLoading, setDetachedLoading] = useState(false);
  const [detachedError, setDetachedError] = useState<string | null>(null);
  const [openingBuildId, setOpeningBuildId] = useState<BuildId>();
  const [liveBuilds, setLiveBuilds] = useState<readonly ReviewBuild[]>([]);
  const hasRunningBuild = liveBuilds.some(
    (build) => build.latestAttempt?.executionState === 'running',
  );
  useEffect(() => {
    setLiveBuilds(detail?.builds ?? []);
  }, [detail]);
  useEffect(() => {
    if (!onMarkBuildsRead || liveBuilds.length === 0 || unreadBuilds.length === 0) return;
    const visible = new Set(
      liveBuilds
        .filter(
          (build) =>
            build.latestAttempt?.executionState !== 'pending' &&
            build.latestAttempt?.executionState !== 'running',
        )
        .map((build) => build.buildId),
    );
    const read = unreadBuilds
      .filter((build) => visible.has(build.buildId))
      .map((build) => build.buildId);
    if (read.length > 0) onMarkBuildsRead(read);
  }, [liveBuilds, onMarkBuildsRead, unreadBuilds]);
  useEffect(() => {
    if (!target || (busy !== 'create-build' && !hasRunningBuild)) return;
    let active = true;
    const poll = () => {
      void client
        .targetDetail(target)
        .then((fresh) => {
          if (active) setLiveBuilds(fresh.builds);
        })
        .catch(() => {
          /* the primary action reports its own failure */
        });
    };
    poll();
    const timer = window.setInterval(poll, 1200);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [client, target, busy, hasRunningBuild]);
  const disabled = loading || (busy !== null && busy !== 'create-build');
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
    setShowDetached(false);
    setError(null);
    setNotice(null);
    await selection.selectTarget(target);
  }
  async function changeRepository(id: string) {
    setShowDetached(false);
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
      if (showDetached) await viewDetached();
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
  async function viewDetached() {
    if (!repositoryId) return;
    setShowDetached(true);
    setDetachedLoading(true);
    setDetachedError(null);
    try {
      setDetached(await client.detachedWorktrees(repositoryId));
    } catch (cause) {
      setDetachedError(message(cause));
    } finally {
      setDetachedLoading(false);
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
  async function createBuild(request: CreateBuildRequest) {
    setBusy('create-build');
    setError(null);
    setNotice(null);
    try {
      const started = await client.createBuild(request);
      setLiveBuilds((current) => [
        started,
        ...current.filter((build) => build.buildId !== started.buildId),
      ]);
      setBuildDialogOpen(false);
      requestAnimationFrame(() => buildTrigger.current?.focus());
      setNotice('Build started. You can continue navigating while it runs.');
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }
  async function openBuild(buildId: BuildId) {
    setOpeningBuildId(buildId);
    setError(null);
    try {
      const outcome = await client.openBuild({ buildId });
      setNotice(outcome.outcome === 'focused_existing' ? 'Focused existing build.' : 'Launched build.');
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
          onViewDetached={() => void viewDetached()}
          detachedSelected={showDetached}
          unreadBuilds={unreadBuilds}
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
          {showDetached && (
            <DetachedWorktreeList
              worktrees={detached}
              loading={detachedLoading}
              error={detachedError}
            />
          )}
          {!showDetached && detail && (
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
                unreadWorktreeIds={new Set(
                  unreadBuilds
                    .filter((build) => build.repositoryId === repositoryId)
                    .flatMap((build) =>
                      build.sourceWorktreeId ? [build.sourceWorktreeId] : [],
                    ),
                )}
              />
              <BuildHistory
                builds={liveBuilds}
                openingBuildId={openingBuildId}
                onOpen={(buildId) => void openBuild(buildId)}
                readLog={client.readBuildLog}
                createButtonRef={buildTrigger}
                onCreate={() => {
                  setDraft(
                    initialBuildDraft(detail, overview.activeBuildContext?.worktreeId),
                  );
                  setBuildDialogOpen(true);
                }}
                onRebuild={(build) => {
                  const worktree = detail.worktrees.find(
                    (candidate) =>
                      (build.source.kind === 'existing_worktree' &&
                        candidate.associationId === build.source.associationId) ||
                      (build.source.kind === 'physical_worktree' &&
                        candidate.worktreeId === build.source.worktreeId),
                  );
                  if (worktree) setSelectedWorktreeId(worktree.worktreeId);
                  setDraft({
                    sourceMode: 'direct',
                    commit: detail.branch.tip,
                    name: build.name,
                    profile: build.profile ?? 'release',
                  });
                  setBuildDialogOpen(true);
                }}
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
      {buildDialogOpen && detail && draft && (
        <CreateBuildDialog
          client={client}
          detail={detail}
          draft={draft}
          selectedWorktree={selectedWorktree}
          activeWorktreeId={overview.activeBuildContext?.worktreeId}
          buildAvailable={canBuild}
          submitting={busy === 'create-build'}
          onDraftChange={setDraft}
          onClose={() => {
            setBuildDialogOpen(false);
            requestAnimationFrame(() => buildTrigger.current?.focus());
          }}
          onCreate={(request) => void createBuild(request)}
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
