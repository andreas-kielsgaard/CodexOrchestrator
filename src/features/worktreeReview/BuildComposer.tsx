import { useMemo, useState } from 'react';
import {
  workspacePlanDisclosure,
  uniqueCommits,
  type AssociatedWorktree,
  type BranchReviewDetail,
  type CreateBuildRequest,
  type GitCommit,
} from '../../application/worktreeReview';

import { buildRequest, type BuildDraft, type SourceMode } from './buildDraft';

export function BuildComposer({
  detail,
  draft,
  onDraftChange,
  onSelectCommit,
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
  readonly draft: BuildDraft;
  readonly onDraftChange: (draft: BuildDraft) => void;
  readonly onSelectCommit: (commit: GitCommit) => void;
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
  const { sourceMode, commit, name } = draft;
  const commitId = commit.objectId;
  const setSourceMode = (sourceMode: SourceMode) => onDraftChange({ ...draft, sourceMode });
  const setName = (name: string) => onDraftChange({ ...draft, name });
  const [historyOpen, setHistoryOpen] = useState(false);
  const selectedWorktreeIsActive = Boolean(
    selectedWorktree && selectedWorktree.worktreeId === activeWorktreeId,
  );
  const selectedWorktreeIsDirty = selectedWorktree ? hasUncommittedWork(selectedWorktree) : false;

  const commitOptions = useMemo(
    () => uniqueCommits([draft.commit, detail.branch.tip, ...commits]),
    [commits, detail.branch.tip, draft.commit],
  );

  const request = buildRequest(detail, selectedWorktree, draft);

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
          title="Live Worktree checkout"
          description={
            selectedWorktreeIsDirty
              ? 'Record the dirty checkout as a virtual commit, then compile the live checkout with its existing dependencies. Worktree Review will not install dependencies or invalidate the build after later edits.'
              : 'Record the branch and current commit, then compile the live checkout with its existing dependencies. Worktree Review will not install dependencies or invalidate the build after later edits.'
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
              ? 'Create a stable virtual commit first, then create a retained build checkout from that exact commit.'
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
          title="Specific commit"
          description="Build the selected exact commit in a retained checkout."
          onChange={setSourceMode}
        />
      </fieldset>

      {sourceMode === 'commit' && (
        <div className="worktree-review__history-picker">
          {!historyOpen ? (
            <>
              <p className="worktree-review__supporting">
                Current selection: {draft.commit.abbreviatedObjectId} · {draft.commit.subject}
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
                  onChange={(event) => {
                    const commit = commitOptions.find(
                      (item) => item.objectId === event.target.value,
                    );
                    if (commit) onSelectCommit(commit);
                  }}
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
              This checkout is dirty. Worktree Review must create a virtual commit before the build
              can continue. The branch, index, and working tree will not be changed.
            </p>
          )}
          <p>
            The produced application files will be retained in Worktree Review AppData, not copied
            into a source worktree. The source receipt records what triggered the build; it does not
            guarantee application quality or prevent later source changes.
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
          {creating ? 'Building…' : 'Build'}
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

function hasUncommittedWork(worktree: AssociatedWorktree): boolean {
  return (
    worktree.changes.stagedFiles > 0 ||
    worktree.changes.unstagedFiles > 0 ||
    worktree.changes.untrackedFiles > 0
  );
}
