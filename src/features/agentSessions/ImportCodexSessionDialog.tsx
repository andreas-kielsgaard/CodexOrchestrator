import { useEffect, useId, useRef, useState, type FormEvent } from 'react';
import type {
  AgentSessionImportClient,
  CodexImportPreview,
} from '../../application/agentSessions/importContracts';
import './importCodexSession.css';

export function ImportCodexSessionDialog({
  client,
  onImported,
  onClose,
}: {
  client: AgentSessionImportClient;
  onImported(id: string): void;
  onClose(): void;
}) {
  const titleId = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const [link, setLink] = useState('');
  const [preview, setPreview] = useState<CodexImportPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [attempted, setAttempted] = useState(false);
  const requestId = useRef(crypto.randomUUID());
  const busy = useRef(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.current?.showModal();
    return () => {
      mounted.current = false;
      opener?.focus();
    };
  }, []);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (busy.current) return;
    busy.current = true;
    setPending(true);
    setError(null);
    try {
      if (!preview) {
        const result = await client.preview(link.trim());
        if (mounted.current) setPreview(result);
      } else {
        setAttempted(true);
        const id = await client.importConversation({
          requestId: requestId.current,
          link: link.trim(),
          profileId: preview.profileId,
          lastTurnId: preview.lastTurnId,
        });
        if (mounted.current) onImported(id);
      }
    } catch (caught) {
      if (mounted.current) setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      busy.current = false;
      if (mounted.current) setPending(false);
    }
  };
  return (
    <dialog
      ref={dialog}
      className="codex-import-dialog"
      aria-labelledby={titleId}
      onCancel={(event) => {
        event.preventDefault();
        if (!pending) onClose();
      }}
    >
      <form onSubmit={(event) => void submit(event)}>
        <h2 id={titleId}>Import from Codex</h2>
        <p>Create a separate conversation in Orchid using this Codex conversation’s history.</p>
        <label htmlFor={titleId + '-link'}>Codex link</label>
        <input
          id={titleId + '-link'}
          autoFocus
          required
          value={link}
          placeholder="codex://threads/…"
          readOnly={pending || attempted}
          onChange={(event) => {
            setLink(event.target.value);
            setPreview(null);
            setError(null);
          }}
        />
        {preview && (
          <section className="codex-import-preview" aria-label="Conversation preview">
            <h3>{preview.title}</h3>
            <p>
              {preview.turnCount} settled {preview.turnCount === 1 ? 'turn' : 'turns'}
            </p>
            <dl>
              <dt>Working folder</dt>
              <dd>
                {preview.allocateWorkspace
                  ? 'The original folder is unavailable. Orchid will create a new empty working folder.'
                  : preview.sourceDirectory}
              </dd>
              <dt>Capability Profile</dt>
              <dd>{preview.capabilityProfile}</dd>
              <dt>Codex home</dt>
              <dd>{preview.nativeHome}</dd>
            </dl>
            {!preview.allocateWorkspace && <p>Project files stay in the shared working folder.</p>}
            {preview.excerpt && <p className="codex-import-excerpt">{preview.excerpt}</p>}
            <p>Importing sends no message.</p>
          </section>
        )}
        {error && <p role="alert">{error}</p>}
        {pending && (
          <p role="status">{preview ? 'Importing conversation…' : 'Reading conversation…'}</p>
        )}
        <footer>
          <button type="button" onClick={onClose} disabled={pending}>
            Cancel
          </button>
          <button type="submit" disabled={pending || !link.trim()}>
            {preview ? 'Import conversation' : 'Preview conversation'}
          </button>
        </footer>
      </form>
    </dialog>
  );
}
