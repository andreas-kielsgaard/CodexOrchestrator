import { useEffect, useRef, type ReactNode } from 'react';

/** Native modal stacking supplies inertness; Tab boundaries also contain reverse WebView traversal. */
export function ReviewDialog({
  labelledBy,
  onClose,
  dismissible = true,
  className = '',
  children,
}: {
  readonly labelledBy: string;
  readonly onClose: () => void;
  readonly dismissible?: boolean;
  readonly className?: string;
  readonly children: ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const prior = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const dialog = ref.current!;
    dialog.showModal();
    return () => {
      dialog.close();
      // Restore after nested dialogs finish unmounting, so their native close cannot steal focus.
      queueMicrotask(() => {
        if (prior?.isConnected) prior.focus();
      });
    };
  }, []);
  return (
    <dialog
      ref={ref}
      className={`worktree-review__dialog ${className}`}
      aria-labelledby={labelledBy}
      onKeyDown={(event) => {
        if (event.key !== 'Tab' || event.defaultPrevented) return;
        const dialog = event.currentTarget;
        if ([...document.querySelectorAll('dialog[open]')].at(-1) !== dialog) return;
        const controls = [
          ...dialog.querySelectorAll<HTMLElement>(
            'button, input, select, textarea, a[href], [tabindex]',
          ),
        ].filter(
          (element) =>
            element.tabIndex >= 0 &&
            !element.matches(':disabled, [type="hidden"]') &&
            !element.closest('[hidden], [inert]') &&
            getComputedStyle(element).display !== 'none',
        );
        const first = controls[0],
          last = controls.at(-1);
        if (!first) {
          event.preventDefault();
          dialog.focus();
        } else if (
          event.shiftKey &&
          (document.activeElement === first || document.activeElement === dialog)
        ) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }}
      onCancel={(event) => {
        event.preventDefault();
        event.stopPropagation();
        if (
          dismissible &&
          [...document.querySelectorAll('dialog[open]')].at(-1) === event.currentTarget
        )
          onClose();
      }}
      onClick={(event) => {
        if (event.target !== event.currentTarget || !dismissible) return;
        const bounds = event.currentTarget.getBoundingClientRect();
        if (
          event.clientX < bounds.left ||
          event.clientX > bounds.right ||
          event.clientY < bounds.top ||
          event.clientY > bounds.bottom
        )
          onClose();
      }}
    >
      {children}
    </dialog>
  );
}
