import { useLayoutEffect, type RefObject } from 'react';

export function useComposerAutosize(
  textarea: RefObject<HTMLTextAreaElement | null>,
  draft: string,
) {
  useLayoutEffect(() => {
    const input = textarea.current;
    const viewport = input?.closest<HTMLElement>('.agent-session-workspace');
    if (!input) return;
    const nativeSizing = typeof CSS !== 'undefined' && CSS.supports?.('field-sizing', 'content');
    const resize = () => {
      const limit = ((viewport?.clientHeight || window.innerHeight) * 2) / 3;
      input.style.maxHeight = `${limit}px`;
      if (nativeSizing) {
        input.style.setProperty('field-sizing', 'content');
        input.style.height = 'auto';
        return;
      }
      input.style.height = '0px';
      input.style.height = `${Math.min(Math.max(70, input.scrollHeight), limit)}px`;
      input.style.overflowY = input.scrollHeight > limit ? 'auto' : 'hidden';
    };
    resize();
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(resize);
    if (viewport) observer?.observe(viewport);
    return () => observer?.disconnect();
  }, [textarea]);
  // Older WebViews have no native field sizing; only that fallback measures per edit.
  useLayoutEffect(() => {
    if (typeof CSS !== 'undefined' && CSS.supports?.('field-sizing', 'content')) return;
    const input = textarea.current;
    if (!input) return;
    input.style.height = '0px';
    const limit = parseFloat(input.style.maxHeight) || (window.innerHeight * 2) / 3;
    input.style.height = `${Math.min(Math.max(70, input.scrollHeight), limit)}px`;
    input.style.overflowY = input.scrollHeight > limit ? 'auto' : 'hidden';
  }, [textarea, draft]);
}
