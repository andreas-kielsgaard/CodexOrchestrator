import { useEffect, useId, useRef, useState, type ReactNode } from 'react';

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
  const timeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  const id = useId();
  const clear = () => {
    if (timeout.current) clearTimeout(timeout.current);
    timeout.current = null;
  };
  const show = () => {
    clear();
    setOpen(true);
  };
  const hide = () => {
    clear();
    timeout.current = setTimeout(() => setOpen(false), 200);
  };
  useEffect(
    () => () => {
      if (timeout.current) clearTimeout(timeout.current);
    },
    [],
  );
  return (
    <div
      className={`composer-hover-group ${className}`}
      tabIndex={trigger ? 0 : undefined}
      role={trigger ? 'group' : undefined}
      aria-label={trigger ? label : undefined}
      onMouseEnter={show}
      onMouseLeave={(event) => {
        const overlay = event.currentTarget.querySelector('.composer-hover-group__overlay');
        if (!overlay?.contains(document.activeElement)) hide();
      }}
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
          clear();
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
