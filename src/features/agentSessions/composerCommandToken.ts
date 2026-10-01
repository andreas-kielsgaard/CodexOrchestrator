import type { ComposerQuickAction } from './composerQuickMenuTypes';

export function commandToken(draft: string, caret: number, nestedStart?: number) {
  const end = Math.min(caret, draft.length);
  const start = nestedStart ?? draft.lastIndexOf('/', end - 1);
  if (start < 0 || draft[start] !== '/') return null;
  const query = draft.slice(start + 1, end);
  if (/[\r\n]/.test(query)) return null;
  return { start, end, query };
}

export function preferredCommandIndex(items: readonly ComposerQuickAction[], query: string) {
  const search = query.trim().toLocaleLowerCase();
  if (!search) return 0;
  let best = 0;
  let score = -1;
  items.forEach((item, index) => {
    const names = [item.label, item.id.replace(/^skill:/, '')].map((s) => s.toLocaleLowerCase());
    const rank = names.some((name) => name === search)
      ? 3
      : names.some((name) => name.startsWith(search))
        ? 2
        : 1;
    if (rank > score) {
      best = index;
      score = rank;
    }
  });
  return best;
}
