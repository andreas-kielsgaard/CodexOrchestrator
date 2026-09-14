import { useState } from 'react';
import { workspacePlanDisclosure, type CreateBuildRequest } from '../../application/worktreeReview';
import { ReviewDialog } from './ReviewDialog';
import { requiresCheckout } from './buildDraft';

export function BuildConfirmationDialog({
  request,
  onClose,
  onConfirm,
}: {
  readonly request: CreateBuildRequest;
  readonly onClose: () => void;
  readonly onConfirm: (request: CreateBuildRequest) => void;
}) {
  const [debugging, setDebugging] = useState(false);
  const objectId =
    request.source.kind === 'branch_commit' || request.source.kind === 'exact_commit'
      ? request.source.objectId
      : request.source.kind === 'physical_worktree'
        ? request.source.headObjectId
        : null;
  const newCheckout = requiresCheckout(request);
  return (
    <ReviewDialog
      labelledBy="build-confirmation-title"
      onClose={onClose}
      className="worktree-review__checkout-dialog"
    >
      <header>
        <p className="worktree-review__step">Build · {request.name}</p>
        <h2 id="build-confirmation-title">Build this application?</h2>
      </header>
      <p>
        {objectId ? (
          <>
            Selected commit: <code>{objectId.slice(0, 8)}</code>.
          </>
        ) : (
          'The build will use the selected worktree.'
        )}
      </p>
      <p>
        {request.source.kind === 'worktree_snapshot' ||
        (request.source.kind === 'physical_worktree' && request.source.snapshot)
          ? 'Current changes will be captured in a new checkout.'
          : request.source.kind === 'existing_worktree' ||
              request.source.kind === 'physical_worktree'
            ? 'The live checkout will be compiled, including its current changes.'
            : 'The selected committed source will be compiled.'}
      </p>
      {newCheckout && <p>{workspacePlanDisclosure(request.workspacePlan)}</p>}
      <label className="worktree-review__debug-option">
        <input
          type="checkbox"
          checked={debugging}
          onChange={(event) => setDebugging(event.target.checked)}
        />
        Enable debugging
      </label>
      <p>
        Debugging builds include symbols for investigating problems. Normal builds are optimized for
        use.
      </p>
      <p>The completed application will be available to launch separately.</p>
      <footer className="worktree-review__actions">
        <button type="button" className="worktree-review__secondary" onClick={onClose}>
          Cancel
        </button>
        <button
          type="button"
          className="worktree-review__primary"
          onClick={() => onConfirm({ ...request, profile: debugging ? 'debug' : 'release' })}
        >
          Build
        </button>
      </footer>
    </ReviewDialog>
  );
}
