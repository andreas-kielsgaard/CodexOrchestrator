import { skillReferenceParts } from './SkillReferences';
it('highlights only complete references in the current catalogue', () => {
  const skill = { id: 'path', name: 'review', description: 'Review', invocationText: '$review' };
  const parts = skillReferenceParts('Use $review and $review-extra, not $other.', [skill]);
  expect(parts.filter((part) => part.skill)).toEqual([{ text: '$review', skill }]);
  expect(parts.map((part) => part.text).join('')).toBe(
    'Use $review and $review-extra, not $other.',
  );
});
