import type {
  WorktreeRuntimeEvidenceKind,
  WorktreeRuntimeExplorationSnapshot,
  WorktreeRuntimeExplorationSource,
  WorktreeRuntimeMaterialBoundary,
} from '../../application/worktreeRuntime';

interface RuntimeEnvironment {
  readonly VITE_RUNTIME_STATUS_URL?: string;
  readonly VITE_RUNTIME_INSTANCE_ID?: string;
  readonly VITE_RUNTIME_SESSION_ID?: string;
  readonly VITE_RUNTIME_WORKTREE_PATH?: string;
  readonly VITE_RUNTIME_GIT_COMMIT?: string;
  readonly VITE_RUNTIME_SOURCE_FINGERPRINT?: string;
  readonly VITE_RUNTIME_TAURI_IDENTIFIER?: string;
  readonly VITE_RUNTIME_ROOT?: string;
  readonly VITE_RUNTIME_DIST?: string;
  readonly VITE_RUNTIME_CARGO_TARGET?: string;
  readonly VITE_RUNTIME_APP_DATA?: string;
  readonly VITE_RUNTIME_CREDENTIALS?: string;
  readonly VITE_RUNTIME_LOGS?: string;
  readonly VITE_RUNTIME_NODE_CACHE_KEY?: string;
  readonly VITE_RUNTIME_NODE_CACHE_PATH?: string;
  readonly VITE_RUNTIME_RUST_CACHE_KEY?: string;
  readonly VITE_RUNTIME_RUST_CACHE_PATH?: string;
  readonly VITE_RUNTIME_RUST_CACHE_MODE?: string;
  readonly VITE_RUNTIME_VITE_PORT?: string;
  readonly VITE_RUNTIME_STATUS_PORT?: string;
  readonly VITE_RUNTIME_BUILD_OBSERVED?: string;
  readonly VITE_RUNTIME_TESTS_OBSERVED?: string;
}

interface RuntimeStatus {
  readonly stale?: unknown;
  readonly owner?: {
    readonly instanceId?: unknown;
    readonly sessionId?: unknown;
    readonly worktreePath?: unknown;
    readonly gitCommit?: unknown;
  };
}

export function createDevelopmentWorktreeRuntimeSource(
  environment: RuntimeEnvironment = viteEnvironment(),
  fetchStatus: typeof fetch = fetch,
): WorktreeRuntimeExplorationSource {
  return {
    async load(): Promise<WorktreeRuntimeExplorationSnapshot> {
      const instanceId = environment.VITE_RUNTIME_INSTANCE_ID ?? 'unmanaged-development';
      const managed = Boolean(environment.VITE_RUNTIME_INSTANCE_ID);
      const status = await loadStatus(environment.VITE_RUNTIME_STATUS_URL, fetchStatus);
      const ownerMatches =
        status.available &&
        status.body?.owner?.instanceId === instanceId &&
        status.body?.owner?.sessionId === environment.VITE_RUNTIME_SESSION_ID &&
        status.body?.owner?.worktreePath === environment.VITE_RUNTIME_WORKTREE_PATH &&
        status.body?.owner?.gitCommit === environment.VITE_RUNTIME_GIT_COMMIT;
      const stale = status.body?.stale === true;
      const buildObserved = environment.VITE_RUNTIME_BUILD_OBSERVED === 'true';
      const testsObserved = environment.VITE_RUNTIME_TESTS_OBSERVED === 'true';
      const runtimeRoot = environment.VITE_RUNTIME_ROOT ?? 'No managed runtime root';

      return {
        label: managed ? 'Live instance metadata' : 'Recorded fallback',
        notice: managed
          ? 'Launch-time manifest metadata is paired with the current status owner. This development view does not register or control the instance.'
          : 'This recorded fallback shows the proposed read contract. Launch through the worktree runtime harness for live instance metadata.',
        checkedAt: new Date().toISOString(),
        identity: {
          instanceId,
          sessionId: environment.VITE_RUNTIME_SESSION_ID ?? 'not assigned',
          worktreePath: environment.VITE_RUNTIME_WORKTREE_PATH ?? 'not observed',
          gitCommit: environment.VITE_RUNTIME_GIT_COMMIT ?? 'not observed',
          sourceFingerprint: environment.VITE_RUNTIME_SOURCE_FINGERPRINT ?? 'not observed',
          tauriIdentifier: environment.VITE_RUNTIME_TAURI_IDENTIFIER ?? 'not observed',
        },
        materials: materials(environment, runtimeRoot, buildObserved),
        lifecycle: [
          {
            stage: 'Prepared',
            state: managed ? 'Manifest identity loaded' : 'Recorded shape only',
            detail: managed
              ? `Instance ${instanceId} carries one worktree, build, and session identity.`
              : 'No worktree runtime manifest was provided to this development launch.',
            evidence: managed ? 'observed' : 'recorded',
          },
          {
            stage: 'Built',
            state: buildObserved ? 'Build completion recorded' : 'Not observed',
            detail: buildObserved
              ? 'The manifest records a successful debug Tauri build for this source fingerprint.'
              : 'A projected output path is not build evidence.',
            evidence: buildObserved ? 'observed' : 'unsupported',
          },
          {
            stage: 'Tested',
            state: testsObserved ? 'Focused test completion recorded' : 'Not observed',
            detail: testsObserved
              ? 'Harness, application status, and Rust runtime/process scopes completed.'
              : 'No test completion was supplied by the manifest.',
            evidence: testsObserved ? 'observed' : 'unsupported',
          },
          {
            stage: 'Running',
            state: ownerMatches && !stale ? 'Healthy owner match' : 'Health not established',
            detail: ownerMatches
              ? stale
                ? 'The status owner matches, but the instance reports stale state.'
                : 'Status identity matches the instance, session, worktree, and commit.'
              : status.available
                ? 'A status response was received, but its owner did not match this instance.'
                : 'The development status endpoint did not return readable evidence.',
            evidence: ownerMatches && !stale ? 'observed' : 'unsupported',
          },
          {
            stage: 'Teardown and recovery',
            state: 'Recorded exploration proof',
            detail:
              'The harness proved ownership-checked tree teardown and stale recovery. This view cannot observe its own completed teardown.',
            evidence: 'recorded',
          },
        ],
        unsupported: [
          'No durable instance registry, port lease broker, or attention router.',
          'Windows process ownership is verified before taskkill /T, but is not atomic without a Job Object.',
          'No provider credential provisioning or cross-instance approval policy.',
          'Screenshot and recording roots exist; capture is not implemented.',
          'Pause means stop at a safe boundary and restart; process suspension and resumable actions are absent.',
          'No parallel scheduler, automatic approval, or projected-to-actual product event store.',
        ],
        reviewPoints: [
          'Decide whether parallel test instances may receive provider credentials.',
          'Choose which failures or approval gates deserve human attention.',
          'Confirm whether stop and explicit restart is an acceptable first pause model.',
          'Define which observed gates may continue automatically.',
        ],
      };
    },
  };
}

function materials(
  environment: RuntimeEnvironment,
  runtimeRoot: string,
  buildObserved: boolean,
): readonly WorktreeRuntimeMaterialBoundary[] {
  const rustShared = environment.VITE_RUNTIME_RUST_CACHE_MODE === 'sccache';
  const builtEvidence: WorktreeRuntimeEvidenceKind = buildObserved ? 'observed' : 'projected';
  return [
    {
      material: 'Source and modules',
      disposition: 'isolated',
      detail: `${environment.VITE_RUNTIME_WORKTREE_PATH ?? 'worktree'} Â· local node_modules`,
      evidence: 'projected',
    },
    {
      material: 'npm download cache',
      disposition: 'shared-keyed',
      detail: `${environment.VITE_RUNTIME_NODE_CACHE_KEY ?? 'key unavailable'} Â· ${environment.VITE_RUNTIME_NODE_CACHE_PATH ?? 'path unavailable'}`,
      evidence: 'projected',
    },
    {
      material: 'Rust compilation',
      disposition: rustShared ? 'shared-keyed' : 'isolated',
      detail: rustShared
        ? `${environment.VITE_RUNTIME_RUST_CACHE_KEY} Â· ${environment.VITE_RUNTIME_RUST_CACHE_PATH}`
        : `${environment.VITE_RUNTIME_CARGO_TARGET ?? `${runtimeRoot}/cargo-target`} Â· sccache unavailable`,
      evidence: builtEvidence,
    },
    {
      material: 'Frontend and Tauri output',
      disposition: 'isolated',
      detail: `${environment.VITE_RUNTIME_DIST ?? `${runtimeRoot}/dist`} Â· ${environment.VITE_RUNTIME_CARGO_TARGET ?? `${runtimeRoot}/cargo-target`}`,
      evidence: builtEvidence,
    },
    {
      material: 'Database and application state',
      disposition: 'isolated',
      detail: environment.VITE_RUNTIME_APP_DATA ?? `${runtimeRoot}/app-data`,
      evidence: 'projected',
    },
    {
      material: 'Ports',
      disposition: 'isolated',
      detail: `Vite ${environment.VITE_RUNTIME_VITE_PORT ?? 'unassigned'} Â· status ${environment.VITE_RUNTIME_STATUS_PORT ?? 'unassigned'} Â· strict`,
      evidence: 'projected',
    },
    {
      material: 'Credentials',
      disposition: 'isolated',
      detail: `${environment.VITE_RUNTIME_CREDENTIALS ?? `${runtimeRoot}/credentials/codex-home`} Â· ambient provider variables scrubbed`,
      evidence: 'projected',
    },
    {
      material: 'Logs and review evidence',
      disposition: 'isolated',
      detail: `${environment.VITE_RUNTIME_LOGS ?? `${runtimeRoot}/logs`} Â· screenshots Â· recordings`,
      evidence: 'projected',
    },
  ];
}

async function loadStatus(
  statusUrl: string | undefined,
  fetchStatus: typeof fetch,
): Promise<{ readonly available: boolean; readonly body?: RuntimeStatus }> {
  if (!statusUrl) return { available: false };
  try {
    const response = await fetchStatus(statusUrl, { cache: 'no-store' });
    if (!response.ok) return { available: false };
    return { available: true, body: (await response.json()) as RuntimeStatus };
  } catch {
    return { available: false };
  }
}

function viteEnvironment(): RuntimeEnvironment {
  return (import.meta as unknown as { env?: RuntimeEnvironment }).env ?? {};
}
