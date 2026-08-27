import { useEffect, useMemo, useState } from 'react';
import {
  workspacePlanDisclosure,
  type AssociatedWorktree,
  type BranchReviewDetail,
  type CreateBuildRequest,
  type GitCommit,
  type GitObjectId,
} from '../../application/worktreeReview';

type SourceMode = 'direct' | 'snapshot' | 'commit';

export function BuildComposer({
  detail,
  selectedWorktree,
  activeWorktreeId,
  commits,
  historyLoading,
  hasMoreHistory,
  buildAvailable,
  disabled,
  creating,
  onCreate,
  onRequestHistory,
  onLoadMoreHistory,
}: {
  readonly detail: BranchReviewDetail;
  readonly selectedWorktree?: AssociatedWorktree;
  readonly activeWorktreeId?: string;
  readonly commits: readonly GitCommit[];
  readonly historyLoading: boolean;
  readonly hasMoreHistory: boolean;
  readonly buildAvailable: boolean;
  readonly disabled: boolean;
  readonly creating: boolean;
  readonly onCreate: (request: CreateBuildRequest) => void;
  readonly onRequestHistory: () => void;
  readonly onLoadMoreHistory: () => void;
}) {
  const [sourceMode, setSourceMode] = useState<SourceMode>(
    selectedWorktree && selectedWorktree.worktreeId !== activeWorktreeId ? 'direct' : 'commit',
  );
  const [commitId, setCommitId] = useState<GitObjectId>(detail.branch.tip.objectId);
  const [name, setName] = useState(`Review ${detail.branch.displayName}`);
  const [historyOpen, setHistoryOpen] = useState(false);
  const selectedWorktreeIsActive = selectedWorktree?.worktreeId === activeWorktreeId;
  const selectedWorktreeIsDirty = selectedWorktree
    ? hasUncommittedWork(selectedWorktree)
    : false;

  useEffect(() => {
    if ((!selectedWorktree || selectedWorktreeIsActive) && sourceMode !== 'commit') {
      setSourceMode('commit');
    }
  }, [selectedWorktree, selectedWorktreeIsActive, sourceMode]);

  const commitOptions = useMemo(
    () => uniqueCommits([detail.branch.tip, ...commits]),
    [commits, detail.branch.tip],
  );

  const request = useMemo(
    () => createRequest(detail, selectedWorktree, sourceMode, commitId, name.trim()),
    [commitId, detail, name, selectedWorktree, sourceMode],
  );

  return (
    <section className="worktree-review__section" aria-labelledby="worktree-review-build-source">
      <div className="worktree-review__section-heading">
        <div>
          <p className="worktree-review__step">3 · Compilation source</p>
          <h2 id="worktree-review-build-source">Create a build</h2>
        </div>
      </div>

      <fieldset className="worktree-review__source-options">
        <legend>Build from</legend>
        <SourceOption
          value="direct"
          checked={sourceMode === 'direct'}
          disabled={!selectedWorktree || selectedWorktreeIsActive || disabled}
          title="Selected Worktree checkout"
          description={
            selectedWorktreeIsDirty
              ? 'Record the dirty checkout as a virtual commit, then compile the selected live checkout. Later edits do not invalidate the build.'
              : 'Record the branch and current commit, then compile the selected live checkout. Later edits do not invalidate the build.'
          }
          unavailableReason={
            selectedWorktreeIsActive
              ? 'Unavailable for the worktree running this application. Choose another worktree or an exact commit.'
              : undefined
          }
          onChange={setSourceMode}
        />
        <SourceOption
          value="snapshot"
          checked={sourceMode === 'snapshot'}
          disabled={!selectedWorktree || selectedWorktreeIsActive || disabled}
          title="Snapshot current work"
          description={
            selectedWorktreeIsDirty
              ? 'Create a stable virtual commit first, then create a retained build checkout from that immutable commit.'
              : 'Create a retained build checkout from the selected worktree commit.'
          }
          unavailableReason={
            selectedWorktreeIsActive
              ? 'Unavailable for the worktree running this application. Choose another worktree or an exact commit.'
              : undefined
          }
          onChange={setSourceMode}
        />
        <SourceOption
          value="commit"
          checked={sourceMode === 'commit'}
          disabled={disabled}
          title="Specific branch commit"
          description="Build one exact commit belonging to the selected branch in a retained checkout."
          onChange={setSourceMode}
        />
      </fieldset>

      {sourceMode === 'commit' && (
        <div className="worktree-review__history-picker">
          {!historyOpen ? (
            <>
              <p className="worktree-review__supporting">
                Current selection: {detail.branch.tip.abbreviatedObjectId} ·{' '}
                {detail.branch.tip.subject}
              </p>
              <button
                type="button"
                className="worktree-review__secondary"
                disabled={disabled}
                onClick={() => {
                  setHistoryOpen(true);
                  onRequestHistory();
                }}
              >
                Choose another commit
              </button>
            </>
          ) : (
            <>
              <label className="worktree-review__field">
                <span>Branch commit</span>
                <select
                  aria-label="Branch commit"
                  value={commitId}
                  disabled={disabled || historyLoading}
                  onChange={(event) => setCommitId(event.target.value as GitObjectId)}
                >
                  {commitOptions.map((commit) => (
                    <option key={commit.objectId} value={commit.objectId}>
                      {commit.abbreviatedObjectId} · {commit.subject}
                    </option>
                  ))}
                </select>
              </label>
              {historyLoading && (
                <p className="worktree-review__supporting" role="status">
                  Loading branch history…
                </p>
              )}
              {hasMoreHistory && !historyLoading && (
                <button
                  type="button"
                  className="worktree-review__secondary"
                  disabled={disabled}
                  onClick={onLoadMoreHistory}
                >
                  Load more commits
                </button>
              )}
            </>
          )}
        </div>
      )}

      <label className="worktree-review__field">
        <span>Build name</span>
        <input value={name} disabled={disabled} onChange={(event) => setName(event.target.value)} />
      </label>

      {request && (
        <div className="worktree-review__disclosure" role="note" aria-label="Worktree change">
          <strong>Before you create this build</strong>
          <p>{workspacePlanDisclosure(request.workspacePlan)}</p>
          {selectedWorktreeIsDirty && sourceMode !== 'commit' && (
            <p>
              This checkout is dirty. Worktree Review must create a virtual commit before the
              build can continue. The branch, index, and working tree will not be changed.
            </p>
          )}
          <p>
            The produced application files will be retained in Worktree Review AppData, not
            copied into a source worktree. The source receipt records what triggered the build;
            it does not guarantee application quality or prevent later source changes.
          </p>
        </div>
      )}

      {!buildAvailable && (
        <p className="worktree-review__error-text" role="status">
          Building is unavailable for this repository. Branch and worktree facts remain inspectable.
        </p>
      )}

      <div className="worktree-review__actions">
        <button
          type="button"
          className="worktree-review__primary"
          disabled={!request || !buildAvailable || disabled || name.trim().length === 0}
          onClick={() => request && onCreate(request)}
        >
          {creating ? 'Creating build…' : 'Create build'}
        </button>
      </div>
    </section>
  );
}

function SourceOption({
  value,
  checked,
  disabled,
  title,
  description,
  unavailableReason,
  onChange,
}: {
  readonly value: SourceMode;
  readonly checked: boolean;
  readonly disabled: boolean;
  readonly title: string;
  readonly description: string;
  readonly unavailableReason?: string;
  readonly onChange: (mode: SourceMode) => void;
}) {
  return (
    <label
      className={`worktree-review__source-option${checked ? ' worktree-review__source-option--selected' : ''}`}
    >
      <input
        type="radio"
        name="build-source"
        value={value}
        checked={checked}
        disabled={disabled}
        onChange={() => onChange(value)}
      />
      <span>
        <strong>{title}</strong>
        <small>{description}</small>
        {unavailableReason && (
          <small className="worktree-review__warning">{unavailableReason}</small>
        )}
      </span>
    </label>
  );
}

function uniqueCommits(commits: readonly GitCommit[]): readonly GitCommit[] {
  return commits.filter(
    (commit, index) =>
      commits.findIndex((candidate) => candidate.objectId === commit.objectId) === index,
  );
}

function createRequest(
  detail: BranchReviewDetail,
  worktree: AssociatedWorktree | undefined,
  sourceMode: SourceMode,
  commitId: GitObjectId,
  name: string,
): CreateBuildRequest | null {
  const common = {
    repositoryId: detail.branch.repositoryId,
    branchRef: detail.branch.branchRef,
    name,
  } as const;

  if (sourceMode === 'direct') {
    if (!worktree) return null;
    return {
      ...common,
      source: {
        kind: 'existing_worktree',
        associationId: worktree.associationId,
      },
      workspacePlan: {
        kind: 'borrow_selected_worktree',
        associationId: worktree.associationId,
      },
    };
  }

  if (sourceMode === 'snapshot') {
    if (!worktree) return null;
    return {
      ...common,
      source: {
        kind: 'worktree_snapshot',
        associationId: worktree.associationId,
      },
      workspacePlan: {
        kind: 'create_owned_build_worktree',
        originatingAssociationId: worktree.associationId,
      },
    };
  }

  return {
    ...common,
    source: {
      kind: 'branch_commit',
      branchRef: detail.branch.branchRef,
      objectId: commitId,
    },
    workspacePlan: worktree
      ? {
          kind: 'create_owned_build_worktree',
        }
      : {
          kind: 'create_managed_branch_worktree',
          branchRef: detail.branch.branchRef,
        },
  };
}

function hasUncommittedWork(worktree: AssociatedWorktree): boolean {
  return (
    worktree.changes.stagedFiles > 0 ||
    worktree.changes.unstagedFiles > 0 ||
    worktree.changes.untrackedFiles > 0
  );
}
