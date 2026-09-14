import { formatSessionDeepLink, parseSessionDeepLink } from './deepLinks';
it('roundtrips a local Session ID', () =>
  expect(parseSessionDeepLink(formatSessionDeepLink('session-123'))).toBe('session-123'));
it.each([
  'https://sessions/a',
  'codex-orchestrator://workflows/a',
  'codex-orchestrator://sessions/',
  'codex-orchestrator://sessions/a/b',
  'codex-orchestrator://sessions/a?send=hi',
  'codex-orchestrator://sessions/%2Fbad',
  'codex-orchestrator://sessions/%5Cbad',
])('rejects a foreign or malformed link: %s', (value) =>
  expect(parseSessionDeepLink(value)).toBeNull(),
);
