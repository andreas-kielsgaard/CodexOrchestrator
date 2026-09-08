import {
  identityInitials,
  normalizedIdentityColor,
  readableIdentityForeground,
} from './identityPresentation';

describe('identity presentation', () => {
  it('derives at most two initials and preserves a visible empty-name fallback', () => {
    expect(identityInitials('Avery')).toBe('A');
    expect(identityInitials('  Ada Lovelace Byron  ')).toBe('AL');
    expect(identityInitials('')).toBe('?');
  });

  it('normalizes valid colors and safely replaces invalid values', () => {
    expect(normalizedIdentityColor('#AA11FF')).toBe('#aa11ff');
    expect(normalizedIdentityColor('unknown')).toBe('#5f6f65');
  });

  it('selects the foreground with stronger contrast', () => {
    expect(readableIdentityForeground('#ffffff')).toBe('#17211b');
    expect(readableIdentityForeground('#111111')).toBe('#ffffff');
  });
});
