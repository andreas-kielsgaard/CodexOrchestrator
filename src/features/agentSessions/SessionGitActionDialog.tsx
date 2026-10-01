import { useState } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import type { SessionNavigationFolder } from '../../application/agentSessions/navigation';
import type {
  SessionGitAction,
  SessionGitActionInput,
  SessionGitPreview,
  SessionNavigationClient,
} from '../../application/agentSessions/organization';

import { gitActionLabels } from '../../application/agentSessions/gitActions';
export function SessionGitActionDialog({
  node,
  action,
  client,
  onClose,
}: {
  node: SessionNavigationFolder;
  action: SessionGitAction;
  client: SessionNavigationClient;
  onClose(): void;
}) {
  const [value, setValue] = useState('');
  const [startPoint, setStartPoint] = useState(node.branch?.branchRef ?? 'HEAD');
  const [preview, setPreview] = useState<{
    input: SessionGitActionInput;
    result: SessionGitPreview;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [output, setOutput] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const repositoryId =
    node.branch?.repositoryId ??
    (node.placement.kind === 'repository' ? node.placement.repositoryId : '');
  const input: SessionGitActionInput = {
    repositoryId,
    branchRef: node.branch?.branchRef ?? null,
    action,
    value,
    startPoint,
  };
  const needsValue = action === 'create' || action === 'rename' || action === 'merge';
  const change = () => {
    setPreview(null);
    setError(null);
    setOutput(null);
  };
  const run = async (execute: boolean) => {
    setBusy(true);
    setError(null);
    try {
      if (execute && preview && client.executeGitAction) {
        setOutput(await client.executeGitAction(preview.input, preview.result));
        setPreview(null);
      } else if (client.previewGitAction)
        setPreview({ input, result: await client.previewGitAction(input) });
      else throw new Error('Git actions are unavailable.');
    } catch (cause) {
      setError(String(cause));
      setPreview(null);
    } finally {
      setBusy(false);
    }
  };
  return (
    <ModalDialog
      labelledBy="session-git-title"
      onClose={onClose}
      dismissible={!busy}
      className="session-git-dialog"
    >
      <h2 id="session-git-title">{gitActionLabels[action]}</h2>
      <p>{node.branch ? node.label : 'Repository'} · This device</p>
      <p className="session-git-help">
        {action === 'delete'
          ? 'Git refuses unmerged or checked-out branches. Worktrees and files are not deleted.'
          : action === 'merge'
            ? 'Updates this branch’s local worktree only if a clean fast-forward is possible.'
            : action === 'create'
              ? 'Creates a Git branch. It does not switch existing worktrees or create a worktree.'
              : action === 'rename'
                ? 'Renames the local Git branch without force-overwriting another branch.'
                : 'Fetches all configured remotes. Does not merge or change worktree files.'}
      </p>
      {needsValue && (
        <label>
          {action === 'merge' ? 'Merge from branch or commit' : 'New branch name'}
          <input
            disabled={busy}
            value={value}
            autoFocus
            onChange={(e) => {
              setValue(e.target.value);
              change();
            }}
          />
        </label>
      )}
      {action === 'create' && (
        <label>
          Start from branch or commit
          <input
            disabled={busy}
            value={startPoint}
            onChange={(e) => {
              setStartPoint(e.target.value);
              change();
            }}
          />
        </label>
      )}
      {preview && (
        <section aria-label="Git command preview">
          <p>Working directory</p>
          <code>{preview.result.path}</code>
          <pre>{['git', ...preview.result.arguments.map((a) => JSON.stringify(a))].join(' ')}</pre>
          <pre>{preview.result.status}</pre>
        </section>
      )}
      {error && <p role="alert">{error}</p>}
      {output && <pre role="status">{output}</pre>}
      <div className="session-git-buttons">
        <button disabled={busy} onClick={onClose}>
          Close
        </button>
        <button
          disabled={
            busy ||
            !client.previewGitAction ||
            (needsValue && !value.trim()) ||
            (action === 'create' && !startPoint.trim())
          }
          onClick={() => void run(Boolean(preview))}
        >
          {busy ? 'Working…' : preview ? 'Confirm and run' : 'Preview command'}
        </button>
      </div>
    </ModalDialog>
  );
}
