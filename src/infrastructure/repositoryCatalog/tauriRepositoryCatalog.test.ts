import { vi } from 'vitest';
import { createTauriRepositoryCatalogClient } from './tauriRepositoryCatalog';

describe('Tauri repository catalog client', () => {
  it('maps registration, discovery, and worktree target use cases to generic commands', async () => {
    const invoke = vi.fn().mockResolvedValue({});
    const client = createTauriRepositoryCatalogClient(invoke);

    await client.overview();
    await client.registerDirectory('C:\\Projects\\Repository');
    await client.registerCodexRepository('repository-codex');
    await client.listWorktreeTargets();

    expect(invoke.mock.calls).toEqual([
      ['repository_catalog_overview'],
      ['register_repository_directory', { input: { repositoryRoot: 'C:\\Projects\\Repository' } }],
      ['register_codex_repository', { input: { repositoryId: 'repository-codex' } }],
      ['list_registered_repository_worktree_targets'],
    ]);
  });
});
