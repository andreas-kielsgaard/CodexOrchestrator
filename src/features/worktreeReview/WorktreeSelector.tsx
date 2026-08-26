import { useEffect, useState } from 'react';
import {
  baselineLabel,
  changesLabel,
  hasWorkSinceBaseline,
  type AssociateWorktreeRequest,
  type AssociatedWorktree,
  type GitCommit,
  type GitObjectId,
  type WorktreeAssociationCandidate,
  type WorktreeAssociationId,
} from '../../application/worktreeReview';

export function WorktreeSelector({
  worktrees,
  candidates,
  selectedAssociationId,
  activeWorktreeId,
  historyCommits,
  historyLoading,
  hasMoreHistory,
  createAvailable,
  busy,
  onSelect,
  onCreate,
  onAssociate,
  onRequestHistory,
  onLoadMoreHistory,
}: {
  readonly worktrees: readonly AssociatedWorktree[];
  readonly candidates: readonly WorktreeAssociationCandidate[];
  readonly selectedAssociationId: WorktreeAssociationId | '';
  readonly activeWorktreeId?: string;
  readonly historyCommits: readonly GitCommit[];
  readonly historyLoading: boolean;
  readonly hasMoreHistory: boolean;
  readonly createAvailable: boolean;
  readonly busy: boolean;
  readonly onSelect: (associationId: WorktreeAssociationId) => void;
  readonly onCreate: () => void;
  readonly onAssociate: (
    candidate: WorktreeAssociationCandidate,
    baseline: AssociateWorktreeRequest['baseline'],
  ) => void;
  readonly onRequestHistory: () => void;
  readonly onLoadMoreHistory: () => void;
}) {
  const availableWorktrees = worktrees.filter(
    (worktree) => worktree.availability.state === 'available',
  );

  return (
    <section className="worktree-review__section" aria-labelledby="worktree-review-worktrees">
      <div className="worktree-review__section-heading">
        <div>
          <p className="worktree-review__step">2 · Worktree checkout</p>
          <h2 id="worktree-review-worktrees">Choose the exact worktree</h2>
        </div>
        <button
          type="button"
          className="worktree-review__secondary"
          disabled={!createAvailable || busy}
          onClick={onCreate}
        >
          Create worktree
        </button>
      </div>
      <p className="worktree-review__supporting">
        Worktrees are physical checkouts. More than one can represent the same branch or commit.
      </p>

      {worktrees.length === 0 ? (
        <p className="worktree-review__empty">
          No associated worktree exists. You can create one now, or Create Build can create and
          retain one after disclosing the change.
        </p>
      ) : (
        <div
          className="worktree-review__worktree-list"
          role="radiogroup"
          aria-label="Worktree checkout"
        >
          {worktrees.map((worktree) => {
            const available = worktree.availability.state === 'available';
            const selected = worktree.associationId === selectedAssociationId;
            const active = worktree.worktreeId === activeWorktreeId;
            return (
              <label
                className={`worktree-review__worktree${selected ? ' worktree-review__worktree--selected' : ''}${active ? ' worktree-review__worktree--active' : ''}${!available ? ' worktree-review__worktree--unavailable' : ''}`}
                key={worktree.associationId}
              >
                <input
                  type="radio"
                  name="worktree"
                  value={worktree.associationId}
                  checked={selected}
                  disabled={!available || busy}
                  onChange={() => onSelect(worktree.associationId)}
                />
                <span className="worktree-review__worktree-content">
                  <span className="worktree-review__worktree-title">
                    <strong>{worktree.name}</strong>
                    <code>{worktree.currentHead.abbreviatedObjectId}</code>
                    {worktree.detachedHead && (
                      <span className="worktree-review__tag">Detached HEAD</span>
                    )}
                    {active && <span className="worktree-review__tag">Active build checkout</span>}
                  </span>
                  <span className="worktree-review__path">{worktree.locationLabel}</span>
                  <span>{baselineLabel(worktree.baseline)}</span>
                  <span
                    className={
                      hasWorkSinceBaseline(worktree.changes)
                        ? 'worktree-review__changed'
                        : undefined
                    }
                  >
                    {changesLabel(worktree.changes)}
                  </span>
                  <span>Ownership: {ownershipLabel(worktree.ownership)}</span>
                  {!available && (
                    <span className="worktree-review__error-text">
                      {worktree.availability.detail}
                    </span>
                  )}
                </span>
              </label>
            );
          })}
        </div>
      )}

      {availableWorktrees.length > 1 && (
        <p className="worktree-review__supporting" role="status">
          {availableWorktrees.length} worktrees are available. Your exact selection will be recorded
          with the build.
        </p>
      )}

      {candidates.length > 0 && (
        <div className="worktree-review__candidates">
          <h3>Checkouts requiring association</h3>
          <p className="worktree-review__supporting">
            These checkouts are not build sources until you explicitly associate one with this
            branch.
          </p>
          {candidates.map((candidate) => (
            <AssociationCandidate
              key={candidate.worktreeId}
              candidate={candidate}
              commits={historyCommits}
              historyLoading={historyLoading}
              hasMoreHistory={hasMoreHistory}
              busy={busy}
              onAssociate={onAssociate}
              onRequestHistory={onRequestHistory}
              onLoadMoreHistory={onLoadMoreHistory}
            />
          ))}
        </div>
      )}
    </section>
  );
}

function AssociationCandidate({
  candidate,
  commits,
  historyLoading,
  hasMoreHistory,
  busy,
  onAssociate,
  onRequestHistory,
  onLoadMoreHistory,
}: {
  readonly candidate: WorktreeAssociationCandidate;
  readonly commits: readonly GitCommit[];
  readonly historyLoading: boolean;
  readonly hasMoreHistory: boolean;
  readonly busy: boolean;
  readonly onAssociate: (
    candidate: WorktreeAssociationCandidate,
    baseline: AssociateWorktreeRequest['baseline'],
  ) => void;
  readonly onRequestHistory: () => void;
  readonly onLoadMoreHistory: () => void;
}) {
  const [baseline, setBaseline] = useState<'observed' | GitObjectId>('observed');
  const [historyOpen, setHistoryOpen] = useState(false);
  useEffect(() => {
    setBaseline('observed');
    setHistoryOpen(false);
  }, [candidate.worktreeId]);

  return (
    <div className="worktree-review__candidate">
      <div>
        <strong>{candidate.name}</strong>
        <span>{candidate.locationLabel}</span>
        <span>
          {candidate.detachedHead ? 'Detached HEAD' : 'Checkout'} at{' '}
          {candidate.currentHead.abbreviatedObjectId}
        </span>
        <small>{candidate.associationReason}</small>
      </div>
      {!historyOpen ? (
        <button
          type="button"
          className="worktree-review__secondary"
          disabled={busy}
          onClick={() => {
            setHistoryOpen(true);
            onRequestHistory();
          }}
        >
          Choose a specific baseline
        </button>
      ) : (
        <div className="worktree-review__history-picker">
          <label className="worktree-review__field worktree-review__field--compact">
            <span>Association baseline</span>
            <select
              aria-label={`Association baseline for ${candidate.name}`}
              value={baseline}
              disabled={busy || historyLoading}
              onChange={(event) => setBaseline(event.target.value as 'observed' | GitObjectId)}
            >
              <option value="observed">Current HEAD when associated</option>
              {commits.map((commit) => (
                <option key={commit.objectId} value={commit.objectId}>
                  {commit.abbreviatedObjectId} · {commit.subject}
                </option>
              ))}
            </select>
          </label>
          {historyLoading && <span role="status">Loading branch history…</span>}
          {hasMoreHistory && !historyLoading && (
            <button
              type="button"
              className="worktree-review__secondary"
              disabled={busy}
              onClick={onLoadMoreHistory}
            >
              Load more commits
            </button>
          )}
        </div>
      )}
      <button
        type="button"
        className="worktree-review__secondary"
        disabled={busy}
        onClick={() =>
          onAssociate(
            candidate,
            baseline === 'observed'
              ? { kind: 'observed_current_head' }
              : { kind: 'selected_commit', objectId: baseline },
          )
        }
      >
        Associate with branch
      </button>
    </div>
  );
}

function ownershipLabel(ownership: AssociatedWorktree['ownership']): string {
  switch (ownership) {
    case 'borrowed_external':
      return 'Borrowed; never removed by build cleanup';
    case 'managed_branch_worktree':
      return 'Managed branch worktree; retained independently';
    case 'owned_build_worktree':
      return 'Build-owned; retained with its build';
  }
}
