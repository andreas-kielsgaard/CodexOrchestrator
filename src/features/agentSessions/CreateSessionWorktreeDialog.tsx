import { useState } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import { BranchGraphBrowser } from '../branches/BranchGraphBrowser';
import type { RepositoryBranchSource, ReviewTarget } from '../../application/branches';
import type { SessionNavigationClient } from '../../application/agentSessions/organization';

export function CreateSessionWorktreeDialog({
  repositoryId,
  source,
  client,
  onClose,
  onCreated,
}: {
  repositoryId: string;
  source: RepositoryBranchSource;
  client: SessionNavigationClient;
  onClose(): void;
  onCreated(): void;
}) {
  const [branch, setBranch] = useState<ReviewTarget | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <ModalDialog labelledBy="create-session-worktree-title" onClose={onClose} dismissible={!busy}>
      <h2 id="create-session-worktree-title">Create worktree</h2>
      <p>Select a branch. The worktree will be created in Orchid’s managed location.</p>
      <BranchGraphBrowser
        source={source}
        repositoryId={repositoryId}
        branchesOnly
        selectedTarget={branch}
        onSelect={setBranch}
      />
      {error && <p role="alert">{error}</p>}
      <footer className="session-target-dialog__footer">
        <button onClick={onClose} disabled={busy}>
          Cancel
        </button>
        <button
          disabled={busy || branch?.kind !== 'branch' || !client.createWorktree}
          onClick={async () => {
            if (branch?.kind !== 'branch') return;
            setBusy(true);
            setError(null);
            try {
              await client.createWorktree!(repositoryId, branch.branchRef);
              onCreated();
              onClose();
            } catch (e) {
              setError(String(e));
              setBusy(false);
            }
          }}
        >
          {busy ? 'Creating…' : 'Create worktree'}
        </button>
      </footer>
    </ModalDialog>
  );
}
