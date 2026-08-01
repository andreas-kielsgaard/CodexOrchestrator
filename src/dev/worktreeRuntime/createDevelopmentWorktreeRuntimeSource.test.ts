import { createDevelopmentWorktreeRuntimeSource } from './createDevelopmentWorktreeRuntimeSource';

describe('development worktree runtime source', () => {
  it('labels matching launch metadata and status ownership as observed', async () => {
    const source = createDevelopmentWorktreeRuntimeSource(
      {
        VITE_RUNTIME_STATUS_URL: 'http://127.0.0.1:41635/status',
        VITE_RUNTIME_INSTANCE_ID: 'proof-a',
        VITE_RUNTIME_SESSION_ID: 'session-a',
        VITE_RUNTIME_WORKTREE_PATH: 'C:\\worktree-a',
        VITE_RUNTIME_GIT_COMMIT: 'commit-a',
        VITE_RUNTIME_SOURCE_FINGERPRINT: 'source-a',
        VITE_RUNTIME_TAURI_IDENTIFIER: 'dev.worktree.a',
        VITE_RUNTIME_ROOT: 'C:\\runtime-a',
        VITE_RUNTIME_BUILD_OBSERVED: 'true',
        VITE_RUNTIME_TESTS_OBSERVED: 'true',
        VITE_RUNTIME_RUST_CACHE_MODE: 'isolated-target-only',
      },
      async () =>
        new Response(
          JSON.stringify({
            stale: false,
            owner: {
              instanceId: 'proof-a',
              sessionId: 'session-a',
              worktreePath: 'C:\\worktree-a',
              gitCommit: 'commit-a',
            },
          }),
          { status: 200 },
        ),
    );

    const snapshot = await source.load();

    expect(snapshot.label).toBe('Live instance metadata');
    expect(snapshot.lifecycle).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          stage: 'Running',
          state: 'Healthy owner match',
          evidence: 'observed',
        }),
      ]),
    );
    expect(snapshot.materials).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          material: 'npm download cache',
          disposition: 'shared-keyed',
        }),
        expect.objectContaining({
          material: 'Rust compilation',
          disposition: 'isolated',
        }),
      ]),
    );
  });

  it('keeps an unmanaged development launch explicitly recorded and unsupported', async () => {
    const snapshot = await createDevelopmentWorktreeRuntimeSource(
      {},
      async () => new Response(null, { status: 503 }),
    ).load();

    expect(snapshot.label).toBe('Recorded fallback');
    expect(snapshot.lifecycle.find(({ stage }) => stage === 'Running')?.evidence).toBe(
      'unsupported',
    );
  });
});
