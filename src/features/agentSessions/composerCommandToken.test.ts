import { commandToken, preferredCommandIndex } from './composerCommandToken';
import type { ComposerQuickAction } from './composerQuickMenuTypes';

it('parses at the caret within a message, including after a newline', () => {
  expect(commandToken('Please use /plan and continue', 16)).toEqual({
    start: 11,
    end: 16,
    query: 'plan',
  });
  expect(commandToken('Text\n/review', 12)).toEqual({ start: 5, end: 12, query: 'review' });
});
it('prefers exact matches then prefixes without reordering the menu', () => {
  const actions: ComposerQuickAction[] = ['explain-plan', 'plan-implementation-shape', 'plan'].map(
    (label) => ({
      id: label,
      label,
      description: '',
      run: () => ({ replacement: '', notice: '' }),
    }),
  );
  expect(preferredCommandIndex(actions, 'plan')).toBe(2);
  expect(preferredCommandIndex(actions.slice(0, 2), 'plan')).toBe(1);
  expect(actions.map((item) => item.id)).toEqual([
    'explain-plan',
    'plan-implementation-shape',
    'plan',
  ]);
});
it('ends command selection on whitespace and allows another slash later', () => {
  expect(commandToken('/plan ', 6)).toBeNull();
  expect(commandToken('Use /plan more text', 19)).toBeNull();
  expect(commandToken('Use /plan then /review', 22)).toEqual({
    start: 15,
    end: 22,
    query: 'review',
  });
});
