import { useEffect, useState } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import type {
  ExecutionTargetClient,
  ExecutionTargetProfileDto,
  SessionExecutionSelectionDto,
} from '../../application/executionTargets/contracts';
export interface WorktreeCreationRequest {
  repositoryId: string;
  repositoryName: string;
  branchRef: string;
  profile: ExecutionTargetProfileDto;
}
export function WorktreeCreationConfirmation({
  client,
  request,
  onCancel,
  onConfirm,
}: {
  client: ExecutionTargetClient;
  request: WorktreeCreationRequest;
  onCancel(): void;
  onConfirm(selection: SessionExecutionSelectionDto): void;
}) {
  const [commit, setCommit] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    setCommit(null);
    setError(null);
    if (!client.resolvePublishedTip) {
      setError('Published commit discovery is unavailable.');
      return;
    }
    void client
      .resolvePublishedTip({
        repositoryId: request.repositoryId,
        branchRef: request.branchRef,
        execution: request.profile.execution,
      })
      .then(
        (value) => {
          if (current) setCommit(value.commit);
        },
        (cause) => {
          if (current) setError(String(cause));
        },
      );
    return () => {
      current = false;
    };
  }, [client, request]);
  return (
    <ModalDialog
      labelledBy="create-session-worktree-title"
      className="session-worktree-confirmation"
      onClose={onCancel}
    >
      <h2 id="create-session-worktree-title">Create a worktree on Send?</h2>
      <p>
        This branch has no worktree on {request.profile.execution.deviceName}. Send will create one
        at the published commit below.
      </p>
      <dl>
        <dt>Repository</dt>
        <dd>{request.repositoryName}</dd>
        <dt>Branch</dt>
        <dd>{request.branchRef.replace(/^refs\/heads\//, '')}</dd>
        <dt>Device</dt>
        <dd>{request.profile.execution.deviceName}</dd>
        <dt>Published commit</dt>
        <dd>
          {commit ? (
            <code title={commit}>{commit.slice(0, 12)}</code>
          ) : error ? (
            'Unavailable'
          ) : (
            'Resolving published commit…'
          )}
        </dd>
      </dl>
      {error && <p role="alert">{error}</p>}
      <footer>
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
        <button
          type="button"
          disabled={!commit}
          onClick={() =>
            commit &&
            onConfirm({
              capabilityProfileId: request.profile.capabilityProfileId,
              capabilityProfileRevision: request.profile.capabilityProfileRevision,
              execution: request.profile.execution,
              workspace: {
                kind: 'create',
                repositoryId: request.repositoryId,
                branchRef: request.branchRef,
                commit,
                attachment: 'branch',
              },
            })
          }
        >
          Use this commit
        </button>
      </footer>
    </ModalDialog>
  );
}
