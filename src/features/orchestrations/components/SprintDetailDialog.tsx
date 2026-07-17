import { X } from 'lucide-react';
import { useEffect, useRef, type KeyboardEvent } from 'react';
import type { SprintPlanItemPresentation } from '../orchestrationModel';
import { sprintStatusLabel } from './presentationLabels';
import '../styles/sprintDetailDialog.css';

export interface SprintDetailDialogProps {
  readonly sprint: SprintPlanItemPresentation;
  readonly restoreFocusTo: HTMLButtonElement;
  readonly onClose: () => void;
}

export function SprintDetailDialog({ sprint, restoreFocusTo, onClose }: SprintDetailDialogProps) {
  const dialogRef = useRef<HTMLElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    closeButtonRef.current?.focus();
    return () => {
      if (restoreFocusTo.isConnected) restoreFocusTo.focus();
    };
  }, [restoreFocusTo]);

  const handleKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onClose();
      return;
    }
    if (event.key !== 'Tab') return;

    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(
        'button:not([disabled]), a[href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ) ?? [],
    );
    if (focusable.length === 0) {
      event.preventDefault();
      dialogRef.current?.focus();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (focusable.length === 1 || (event.shiftKey && document.activeElement === first)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <div className="sprint-dialog-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        ref={dialogRef}
        className="sprint-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="sprint-dialog-heading"
        tabIndex={-1}
        onMouseDown={(event) => event.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <header>
          <div>
            <p className="eyebrow">Sprint · {sprintStatusLabel(sprint.status)}</p>
            <h2 id="sprint-dialog-heading">{sprint.name}</h2>
          </div>
          <button
            ref={closeButtonRef}
            type="button"
            onClick={onClose}
            aria-label="Close Sprint detail"
          >
            <X size={18} aria-hidden="true" />
          </button>
        </header>
        <p>{sprint.purpose}</p>
        {sprint.detail && (
          <dl>
            <div>
              <dt>Summary</dt>
              <dd>{sprint.detail.summary}</dd>
            </div>
            <div>
              <dt>Recorded outcome</dt>
              <dd>{sprint.detail.outcome}</dd>
            </div>
          </dl>
        )}
        {sprint.agentSession && (
          <p className="sprint-dialog__session">
            Agent Session <strong>{sprint.agentSession.title}</strong>
            <code>{sprint.agentSession.sessionId}</code>
          </p>
        )}
      </section>
    </div>
  );
}
