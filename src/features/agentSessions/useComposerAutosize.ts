import { useLayoutEffect, type RefObject } from 'react';

export function useComposerAutosize(
  textarea: RefObject<HTMLTextAreaElement | null>,
  draft: string,
) {
  useLayoutEffect(() => {
    const input = textarea.current;
    const viewport = input?.closest<HTMLElement>('.agent-session-workspace');
    if (!input) return;
    const resize = () => {
      input.style.height = '0px';
      const limit = ((viewport?.clientHeight || window.innerHeight) * 2) / 3;
      input.style.height = `${Math.min(Math.max(70, input.scrollHeight), limit)}px`;
      input.style.overflowY = input.scrollHeight > limit ? 'auto' : 'hidden';
    };
    resize();
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(resize);
    if (viewport) observer?.observe(viewport);
    return () => observer?.disconnect();
  }, [textarea, draft]);
}
