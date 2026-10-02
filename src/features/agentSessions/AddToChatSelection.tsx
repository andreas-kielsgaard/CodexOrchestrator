import { useEffect, useId, useState, type RefObject } from 'react';
import { createPortal } from 'react-dom';

/** Transcript selection stays local until the user adds the quote to their draft. */
export function AddToChatSelection({
  region,
  onAdd,
}: {
  region: RefObject<HTMLDivElement | null>;
  onAdd(text: string): void;
}) {
  const [excerpt, setExcerpt] = useState<{ text: string; x: number; y: number } | null>(null);
  const [commenting, setCommenting] = useState(false);
  const [comment, setComment] = useState('');
  const id = useId();
  useEffect(() => {
    const element = region.current;
    if (!element) return;
    const capture = () => {
      const selection = window.getSelection();
      if (
        !selection ||
        selection.isCollapsed ||
        !selection.rangeCount ||
        !element.contains(selection.anchorNode) ||
        !element.contains(selection.focusNode)
      ) {
        setExcerpt(null);
        return;
      }
      const text = selection.toString().trim();
      if (!text) return;
      const rect = selection.getRangeAt(0).getBoundingClientRect();
      setExcerpt({
        text,
        x: Math.max(8, Math.min(rect.left, window.innerWidth - 340)),
        y: Math.max(8, Math.min(rect.top - 58, window.innerHeight - 100)),
      });
      setComment('');
      setCommenting(false);
    };
    element.addEventListener('mouseup', capture);
    element.addEventListener('keyup', capture);
    const close = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setExcerpt(null);
    };
    const outside = (e: PointerEvent) => {
      if (!(e.target instanceof Element) || !e.target.closest(`[data-quote-menu="${id}"]`))
        setExcerpt(null);
    };
    document.addEventListener('keydown', close);
    document.addEventListener('pointerdown', outside);
    const scroll = () => setExcerpt(null);
    element.addEventListener('scroll', scroll, { passive: true });
    return () => {
      element.removeEventListener('mouseup', capture);
      element.removeEventListener('keyup', capture);
      element.removeEventListener('scroll', scroll);
      document.removeEventListener('keydown', close);
      document.removeEventListener('pointerdown', outside);
    };
  }, [region, id]);
  if (!excerpt) return null;
  const add = () => {
    onAdd(
      `${excerpt.text
        .split('\n')
        .map((line) => `> ${line}`)
        .join('\n')}${comment.trim() ? `\n\n${comment.trim()}` : ''}`,
    );
    setExcerpt(null);
    window.getSelection()?.removeAllRanges();
  };
  return createPortal(
    <div
      className="session-quote-menu"
      data-quote-menu={id}
      style={{ left: excerpt.x, top: excerpt.y }}
      role="group"
      aria-label="Selected text actions"
      onMouseDown={(e) => {
        if (e.target instanceof HTMLButtonElement) e.preventDefault();
      }}
    >
      {commenting ? (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            add();
          }}
        >
          <input
            autoFocus
            aria-label="Optional quote comment"
            placeholder="Add an optional comment…"
            value={comment}
            onChange={(e) => setComment(e.target.value)}
          />
          <button type="submit">Add excerpt</button>
        </form>
      ) : (
        <button onClick={() => setCommenting(true)}>Add to chat</button>
      )}
    </div>,
    document.body,
  );
}
