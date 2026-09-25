import { displayCodexHomePath, localCodexRoutes } from './localRoutes';

describe('localCodexRoutes', () => {
  it('keeps extended Windows path syntax out of user-facing route labels', () => {
    const [route] = localCodexRoutes([
      {
        id: 'local',
        homePath: '\\\\?\\C:\\Users\\user\\.codex',
        lifecycle: 'active',
        selected: true,
      },
    ]);

    expect(displayCodexHomePath('\\\\?\\C:\\Users\\user\\.codex')).toBe(
      'C:\\Users\\user\\.codex',
    );
    expect(route.detail).toContain('C:\\Users\\user\\.codex');
    expect(route.detail).not.toContain('\\\\?\\');
  });
});
