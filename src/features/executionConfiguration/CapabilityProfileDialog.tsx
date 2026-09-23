import type { ReactNode } from 'react';

export function CapabilityProfileDialog({
  title,
  children,
  onClose,
}: {
  readonly title: string;
  readonly children: ReactNode;
  onClose(): void;
}) {
  return (
    <div className="capability-dialog__backdrop" role="presentation">
      <section className="capability-dialog" role="dialog" aria-modal="true" aria-label={title}>
        <header>
          <h2>{title}</h2>
          <button type="button" aria-label={`Close ${title}`} onClick={onClose}>
            ×
          </button>
        </header>
        {children}
      </section>
    </div>
  );
}
