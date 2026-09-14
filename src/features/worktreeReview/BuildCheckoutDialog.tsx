import { workspacePlanDisclosure, type CreateBuildRequest } from '../../application/worktreeReview';
import { ReviewDialog } from './ReviewDialog';

export function BuildCheckoutDialog({
  request,
  onClose,
  onConfirm,
}: {
  readonly request: CreateBuildRequest;
  readonly onClose: () => void;
  readonly onConfirm: () => void;
}) {
  const objectId =
    request.source.kind === 'branch_commit' || request.source.kind === 'exact_commit'
      ? request.source.objectId
      : null;
  return (
    <ReviewDialog
      labelledBy="build-checkout-title"
      onClose={onClose}
      className="worktree-review__checkout-dialog"
    >
      <header>
        <p className="worktree-review__step">Build · {request.name}</p>
        <h2 id="build-checkout-title">Create a worktree for this build?</h2>
      </header>
      <p>
        {objectId ? (
          <>
            The build will use commit <code>{objectId.slice(0, 8)}</code> in a new checkout.
          </>
        ) : (
          'The build will use a snapshot of the selected worktree in a new checkout.'
        )}
      </p>
      <p>{workspacePlanDisclosure(request.workspacePlan)}</p>
      <footer className="worktree-review__actions">
        <button type="button" className="worktree-review__secondary" onClick={onClose}>
          Cancel
        </button>
        <button type="button" className="worktree-review__primary" onClick={onConfirm}>
          Create worktree and build
        </button>
      </footer>
    </ReviewDialog>
  );
}
