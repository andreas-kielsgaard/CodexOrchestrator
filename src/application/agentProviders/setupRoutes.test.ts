import { displayFolderPath, providerSetupRoutes } from './setupRoutes';

describe('providerSetupRoutes', () => {
  it('offers each provider setup as a route with readable folder labels', () => {
    const [route] = providerSetupRoutes([
      {
        deviceId: 'local',
        provider: 'codex',
        configurationId: 'home-one',
        folder: '\\\\?\\C:\\Users\\user\\.codex',
        executable: 'codex',
        state: 'ready',
        detail: null,
        selected: true,
      },
    ]);

    expect(displayFolderPath('\\\\?\\C:\\Users\\user\\.codex')).toBe('C:\\Users\\user\\.codex');
    expect(route.detail).toContain('C:\\Users\\user\\.codex');
    expect(route.detail).not.toContain('\\\\?\\');
    expect(route.execution).toMatchObject({ provider: 'codex', configurationRef: 'home-one' });
    expect(route.label).toBe('This device · selected Codex CLI');
  });
});
