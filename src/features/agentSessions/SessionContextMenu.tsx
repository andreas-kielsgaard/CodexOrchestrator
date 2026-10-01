import { useEffect, useRef, useState } from 'react';
import type {
  SessionNavigationModel,
  SessionNavigationRow,
} from '../../application/agentSessions/navigation';
import type { SessionPlacement } from '../../application/agentSessions/organization';
export function SessionContextMenu({
  session,
  model,
  position,
  onClose,
  onMove,
  onPin,
  onRename,
  onCopy,
  onCopySession,
  onSetIdentity,
  onConfigure,
  organizing,
}: {
  session: SessionNavigationRow;
  model: SessionNavigationModel;
  position: { x: number; y: number };
  organizing: boolean;
  onClose(): void;
  onMove(placement: SessionPlacement): void;
  onPin(): void;
  onRename(title: string): Promise<void>;
  onCopy(): Promise<void>;
  onCopySession?(): Promise<void>;
  onSetIdentity?(): void;
  onConfigure?(): void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [moving, setMoving] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [title, setTitle] = useState(session.summary.title);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    ref.current?.querySelector<HTMLButtonElement>('button')?.focus();
    const outside = (event: PointerEvent) => {
      if (!ref.current?.contains(event.target as Node)) onClose();
    };
    document.addEventListener('pointerdown', outside);
    return () => document.removeEventListener('pointerdown', outside);
  }, [onClose]);
  return (
    <div
      ref={ref}
      role="menu"
      aria-label={`Session actions for ${session.summary.title}`}
      className="session-context-menu"
      style={{
        left: Math.max(8, Math.min(position.x, window.innerWidth - 280)),
        top: Math.max(8, Math.min(position.y, window.innerHeight - 370)),
      }}
      onKeyDown={(event) => {
        if (event.key === 'Escape') {
          event.preventDefault();
          onClose();
        }
        if (event.target instanceof HTMLInputElement) return;
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
          const buttons = [...ref.current!.querySelectorAll<HTMLButtonElement>('button')];
          const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
          buttons[
            (index + (event.key === 'ArrowDown' ? 1 : buttons.length - 1)) % buttons.length
          ]?.focus();
          event.preventDefault();
        }
      }}
    >
      <button role="menuitem" disabled={!organizing} onClick={onPin}>
        {session.pinned ? 'Unpin' : 'Pin'}
      </button>
      <button role="menuitem" aria-expanded={renaming} onClick={() => setRenaming(!renaming)}>
        Rename
      </button>
      {renaming && (
        <form
          className="session-rename-form"
          onSubmit={(event) => {
            event.preventDefault();
            const next = title.trim();
            if (!next) {
              setError('Enter a session name.');
              return;
            }
            setError(null);
            void onRename(next).catch(() => setError('The session could not be renamed.'));
          }}
        >
          <label>
            <span>Session name</span>
            <input
              value={title}
              maxLength={160}
              autoFocus
              onChange={(event) => setTitle(event.target.value)}
            />
          </label>
          <button type="submit">Save</button>
        </form>
      )}
      <button
        role="menuitem"
        disabled={!organizing}
        aria-expanded={moving}
        onClick={() => setMoving(!moving)}
      >
        Move to…
      </button>
      {moving && (
        <div className="session-move-destinations" aria-label="Move destinations">
          {model.destinations.map((d) => (
            <button
              role="menuitem"
              key={d.label + JSON.stringify(d.placement)}
              onClick={() => onMove(d.placement)}
            >
              {d.label}
            </button>
          ))}
        </div>
      )}
      <button
        role="menuitem"
        onClick={() => void onCopy().then(onClose, () => setError('The link could not be copied.'))}
      >
        Copy deeplink
      </button>
      {onCopySession && (
        <button
          role="menuitem"
          onClick={() =>
            void onCopySession().then(onClose, () => setError('The session could not be copied.'))
          }
        >
          Copy entire session
        </button>
      )}
      {onSetIdentity && (
        <button
          role="menuitem"
          onClick={() => {
            onClose();
            onSetIdentity();
          }}
        >
          Set identity
        </button>
      )}
      {onConfigure && (
        <button
          role="menuitem"
          onClick={() => {
            onClose();
            onConfigure();
          }}
        >
          Session configuration
        </button>
      )}
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
