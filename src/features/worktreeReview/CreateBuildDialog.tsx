import { useState } from 'react';
import {
  workspacePlanDisclosure,
  type AssociatedWorktree,
  type BranchReviewDetail,
  type CreateBuildRequest,
  type GitCommit,
  type GraphConnection,
  type ReviewBranch,
  type WorktreeReviewClient,
} from '../../application/worktreeReview';
import { BranchGraphBrowser } from '../branches/BranchGraphBrowser';
import { buildRequest, type BuildDraft, type SourceMode } from './buildDraft';
import { CommitRangeDialog } from './branchSelection/CommitRangeDialog';
import { ReviewDialog } from './ReviewDialog';

export function CreateBuildDialog({
  client,
  detail,
  draft,
  selectedWorktree,
  activeWorktreeId,
  buildAvailable,
  submitting,
  onDraftChange,
  onClose,
  onCreate,
}: {
  readonly client: WorktreeReviewClient;
  readonly detail: BranchReviewDetail;
  readonly draft: BuildDraft;
  readonly selectedWorktree?: AssociatedWorktree;
  readonly activeWorktreeId?: string;
  readonly buildAvailable: boolean;
  readonly submitting: boolean;
  readonly onDraftChange: (draft: BuildDraft) => void;
  readonly onClose: () => void;
  readonly onCreate: (request: CreateBuildRequest) => void;
}) {
  const [commitPickerOpen, setCommitPickerOpen] = useState(false);
  const selectedWorktreeIsActive = selectedWorktree?.worktreeId === activeWorktreeId;
  const selectedWorktreeIsDirty = selectedWorktree ? hasUncommittedWork(selectedWorktree) : false;
  const setSourceMode = (sourceMode: SourceMode) => onDraftChange({ ...draft, sourceMode });
  const request = buildRequest(detail, selectedWorktree, draft);

  return (
    <ReviewDialog
      labelledBy="create-build-title"
      onClose={onClose}
      className="worktree-review__build-dialog"
    >
      <header className="worktree-review__modal-header">
        <div>
          <p className="worktree-review__step">Worktree Review</p>
          <h2 id="create-build-title">Create a build</h2>
        </div>
        <button type="button" className="worktree-review__secondary" onClick={onClose}>
          Close
        </button>
      </header>

      <fieldset className="worktree-review__source-options">
        <legend>Build from</legend>
        <SourceOption
          value="direct"
          checked={draft.sourceMode === 'direct'}
          disabled={!selectedWorktree || selectedWorktreeIsActive || submitting}
          title="Live worktree checkout"
          description="Build directly in the worktree."
          onChange={setSourceMode}
        />
        <SourceOption
          value="snapshot"
          checked={draft.sourceMode === 'snapshot'}
          disabled={!selectedWorktree || selectedWorktreeIsActive || submitting}
          title="Snapshot current work"
          description="Copy the selected worktree and build in the destination."
          onChange={setSourceMode}
        />
        <SourceOption
          value="commit"
          checked={draft.sourceMode === 'commit'}
          disabled={submitting}
          title="Specific commit"
          description="Create worktree from the selected commit and build there."
          onChange={setSourceMode}
        />
      </fieldset>

      {draft.sourceMode === 'commit' && (
        <div className="worktree-review__history-picker">
          <p className="worktree-review__supporting">
            Selected commit: <code>{draft.commit.abbreviatedObjectId}</code> ·{' '}
            {draft.commit.subject}
          </p>
          <button
            type="button"
            className="worktree-review__secondary"
            disabled={submitting}
            onClick={() => setCommitPickerOpen(true)}
          >
            Choose from repository history…
          </button>
        </div>
      )}

      <label className="worktree-review__field">
        <span>Build name</span>
        <input
          value={draft.name}
          disabled={submitting}
          onChange={(event) => onDraftChange({ ...draft, name: event.target.value })}
        />
      </label>
      <label className="worktree-review__field">
        <span>Build mode</span>
        <select
          value={draft.profile}
          disabled={submitting}
          onChange={(event) =>
            onDraftChange({
              ...draft,
              profile: event.target.value === 'debug' ? 'debug' : 'release',
            })
          }
        >
          <option value="release">Normal</option>
          <option value="debug">Debugging</option>
        </select>
      </label>

      {selectedWorktreeIsActive && draft.sourceMode !== 'commit' && (
        <p className="worktree-review__error-text">
          The worktree running this application cannot be used as a live or snapshot source.
        </p>
      )}
      {selectedWorktreeIsDirty && draft.sourceMode !== 'commit' && (
        <p className="worktree-review__attention">
          The selected checkout has uncommitted work. Worktree Review will preserve the source
          identity without changing its branch, index, or working tree.
        </p>
      )}
      {request && <p className="worktree-review__supporting">{workspacePlanDisclosure(request.workspacePlan)}</p>}
      {!buildAvailable && (
        <p className="worktree-review__error-text">
          Building is unavailable for this repository.
        </p>
      )}

      <footer className="worktree-review__actions">
        <button type="button" className="worktree-review__secondary" onClick={onClose}>
          Cancel
        </button>
        <button
          type="button"
          className="worktree-review__primary"
          disabled={
            !request || !buildAvailable || submitting || draft.name.trim().length === 0
          }
          onClick={() => request && onCreate(request)}
        >
          {submitting ? 'Starting…' : 'Create build'}
        </button>
      </footer>

      {commitPickerOpen && (
        <CommitGraphPickerDialog
          client={client}
          detail={detail}
          onClose={() => setCommitPickerOpen(false)}
          onSelect={(commit) => {
            onDraftChange({ ...draft, sourceMode: 'commit', commit });
            setCommitPickerOpen(false);
          }}
        />
      )}
    </ReviewDialog>
  );
}

function CommitGraphPickerDialog({
  client,
  detail,
  onClose,
  onSelect,
}: {
  readonly client: WorktreeReviewClient;
  readonly detail: BranchReviewDetail;
  readonly onClose: () => void;
  readonly onSelect: (commit: GitCommit) => void;
}) {
  const [range, setRange] = useState<{ segment: GraphConnection; source: ReviewBranch }>();
  return (
    <ReviewDialog labelledBy="build-commit-graph-title" onClose={onClose} className="branch-graph-dialog">
      <header className="branch-graph__header">
        <div>
          <p className="worktree-review__step">Repository history</p>
          <h2 id="build-commit-graph-title">Choose a commit</h2>
          <p>Open a commit range in the graph, then choose the exact commit to build.</p>
        </div>
        <button type="button" className="worktree-review__secondary" onClick={onClose}>
          Close
        </button>
      </header>
      <BranchGraphBrowser
        source={client}
        repositoryId={detail.branch.repositoryId}
        selectedTarget={detail.branch.target}
        contextualTarget={detail.branch.target}
        onRange={(segment, source) => setRange({ segment, source })}
      />
      {range && (
        <CommitRangeDialog
          client={client}
          query={{ target: range.source.target, scope: range.segment.scope }}
          source={range.source}
          connection={range.segment}
          onClose={() => setRange(undefined)}
          onSelect={() => undefined}
          onSelectCommit={(commit) => onSelect(commit)}
        />
      )}
    </ReviewDialog>
  );
}

function SourceOption({
  value,
  checked,
  disabled,
  title,
  description,
  onChange,
}: {
  readonly value: SourceMode;
  readonly checked: boolean;
  readonly disabled: boolean;
  readonly title: string;
  readonly description: string;
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
