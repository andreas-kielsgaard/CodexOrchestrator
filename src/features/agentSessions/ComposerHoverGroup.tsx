import { useId, useState, type ReactNode } from 'react';

/** Anchored overlays retain their footprint and allow pointer travel into the controls. */
export function ComposerHoverGroup({
  label,
  summary,
  children,
  className = '',
  trigger,
}: {
  label: string;
  summary: ReactNode;
  children: ReactNode;
  className?: string;
  trigger?: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const show = () => {
    setOpen(true);
  };
  const hide = () => {
    setOpen(false);
  };
  return (
    <div
      className={`composer-hover-group ${className}`}
      tabIndex={trigger ? 0 : undefined}
      role={trigger ? 'group' : undefined}
      aria-label={trigger ? label : undefined}
      onMouseEnter={show}
      onMouseLeave={hide}
      onFocus={show}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) hide();
      }}
      onKeyDown={(event) => {
        if (trigger && event.target === event.currentTarget && event.key === 'Enter') {
          event.preventDefault();
          show();
        }
        if (event.key === 'Escape') {
          if (
            event.currentTarget
              .querySelector('.composer-hover-group__overlay')
              ?.contains(document.activeElement)
          ) {
            const button = event.currentTarget.querySelector<HTMLButtonElement>(
              '.composer-hover-group__trigger',
            );
            (button ?? event.currentTarget).focus();
          }
          setOpen(false);
          event.stopPropagation();
        }
      }}
    >
      {trigger ?? (
        <button
          type="button"
          className="composer-hover-group__trigger"
          aria-label={label}
          aria-expanded={open}
          aria-controls={id}
          onClick={show}
        >
          {summary}
        </button>
      )}
      <div
        id={id}
        className="composer-hover-group__overlay"
        hidden={!open}
        role="group"
        aria-label={label}
      >
        {children}
      </div>
    </div>
  );
}
