import type { DomainRecords } from '../domain/model';
import type {
  GitRepoScanInput,
  GitRepoScanResult,
  GitRepoScanner,
} from './ports/gitRepoScanner';
import type { RepoSyncPlanIdProvider } from '../domain/repoSyncPlanApplier';
import { InMemoryRepoSyncStore } from '../domain/repoSyncStore';
import { registerAndScanRepo } from './repoRegistryScan';

const projectId = 'project-1';
const now = '2026-07-02T10:00:00.000Z';
const yesterday = '2026-07-01T10:00:00.000Z';

describe('registerAndScanRepo', () => {
  it('scans a repo, persists synced records, and returns a UI-friendly summary', async () => {
    const scanner = new FakeGitRepoScanner(
      repoScan({
        rootPath: 'C:/Repos/Codex Orchestrator',
        currentBranch: 'main',
        defaultBranch: 'main',
        scannedAt: now,
      }),
    );
    const store = new InMemoryRepoSyncStore(emptyRecords());

    const result = await registerAndScanRepo(
      {
        scanner,
        store,
        ids: deterministicIds(),
        clock: fixedClock(now),
      },
      {
        projectId,
        rootPath: 'C:\\Repos\\Codex Orchestrator',
        defaultBranch: 'main',
      },
    );

    expect(scanner.inputs).toEqual([
      {
        rootPath: 'C:\\Repos\\Codex Orchestrator',
        defaultBranch: 'main',
        scannedAt: now,
      },
    ]);
    expect(result.repo).toMatchObject({
      id: 'repo:C:/Repos/Codex Orchestrator',
      projectId,
      rootPath: 'C:/Repos/Codex Orchestrator',
      defaultBranch: 'main',
      remoteUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
    });
    expect(result.branches.map((branch) => branch.name)).toEqual([
      'main',
      'worker/030-repo-registry-scan-service',
    ]);
    expect(result.worktrees.map((worktree) => worktree.path)).toEqual([
      'C:/Repos/Codex Orchestrator',
      'C:/Repos/Codex Orchestrator Worktrees/030',
    ]);
    expect(result.scan).toEqual({
      rootPath: 'C:/Repos/Codex Orchestrator',
      scannedAt: now,
      currentBranch: 'main',
      defaultBranch: 'main',
      remoteUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
      isDirty: true,
      branchCount: 2,
      worktreeCount: 2,
    });
    expect(result.sync.changeCounts).toEqual({
      repo: { insert: 1, update: 0 },
      branch: { insert: 2, update: 0 },
      worktree: { insert: 2, update: 0 },
    });
    expect(result.sync.changes.map((change) => `${change.kind}:${change.action}`)).toEqual([
      'repo:insert',
      'branch:insert',
      'branch:insert',
      'worktree:insert',
      'worktree:insert',
    ]);
    expect(result.sync.staleWorktrees).toEqual([]);
    expect(store.snapshot().repos).toEqual([result.repo]);
  });

  it('updates an existing repo and returns only records touched by the scan', async () => {
    const scanner = new FakeGitRepoScanner(
      repoScan({
        rootPath: 'C:/Repos/Codex Orchestrator',
        currentBranch: 'main',
        defaultBranch: undefined,
        scannedAt: now,
      }),
    );
    const records = recordsWithExistingAndUnrelatedRepos();
    const store = new InMemoryRepoSyncStore(records);

    const result = await registerAndScanRepo(
      {
        scanner,
        store,
        ids: deterministicIds(),
        clock: fixedClock(now),
      },
      {
        projectId,
        rootPath: 'C:/Repos/Codex Orchestrator',
      },
    );

    expect(result.repo).toMatchObject({
      id: 'repo-1',
      defaultBranch: 'main',
      updatedAt: now,
    });
    expect(result.branches.map((branch) => branch.id)).toEqual(['branch-main', 'branch-worker']);
    expect(result.worktrees.map((worktree) => worktree.id)).toEqual([
      'worktree-main',
      'worktree-worker',
    ]);
    expect(result.sync.staleWorktrees).toEqual([
      {
        action: 'reported_missing_from_scan',
        worktreeId: 'worktree-old',
        repoId: 'repo-1',
        path: 'C:/Repos/Codex Orchestrator Worktrees/old',
        reason: 'absent_from_current_git_scan',
        lastObservedAt: yesterday,
        plannedAt: now,
      },
    ]);
    expect(result.sync.changeCounts).toEqual({
      repo: { insert: 0, update: 1 },
      branch: { insert: 0, update: 2 },
      worktree: { insert: 0, update: 2 },
    });
    expect(result.scan.defaultBranch).toBe('main');
  });

  it('uses an explicit scannedAt override instead of reading the clock', async () => {
    const scannedAt = '2026-07-02T11:00:00.000Z';
    const scanner = new FakeGitRepoScanner(
      repoScan({
        rootPath: 'C:/Repos/Codex Orchestrator',
        scannedAt,
      }),
    );

    const result = await registerAndScanRepo(
      {
        scanner,
        store: new InMemoryRepoSyncStore(emptyRecords()),
        ids: deterministicIds(),
        clock: {
          now: () => {
            throw new Error('clock should not be read');
          },
        },
      },
      {
        projectId,
        rootPath: 'C:/Repos/Codex Orchestrator',
        scannedAt,
      },
    );

    expect(scanner.inputs[0].scannedAt).toBe(scannedAt);
    expect(result.sync.plannedAt).toBe(scannedAt);
    expect(result.repo.createdAt).toBe(scannedAt);
  });
});

class FakeGitRepoScanner implements GitRepoScanner {
  readonly inputs: GitRepoScanInput[] = [];

  constructor(private readonly scan: GitRepoScanResult) {}

  async scanRepo(input: GitRepoScanInput): Promise<GitRepoScanResult> {
    this.inputs.push(input);
    return this.scan;
  }
}

function fixedClock(timestamp: string) {
  return {
    now: () => timestamp,
  };
}

function deterministicIds(): RepoSyncPlanIdProvider {
  return {
    repoId: (plan) => `repo:${plan.match.rootPath}`,
    branchId: (plan) => `branch:${plan.match.name}`,
    worktreeId: (plan) => `worktree:${plan.match.path}`,
  };
}

function repoScan(overrides: Partial<GitRepoScanResult> = {}): GitRepoScanResult {
  const rootPath = overrides.rootPath ?? 'C:/Repos/Codex Orchestrator';
  const scannedAt = overrides.scannedAt ?? now;

  return {
    rootPath,
    currentBranch: 'main',
    defaultBranch: 'main',
    remotes: [
      {
        name: 'origin',
        fetchUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
        pushUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
      },
    ],
    branches: [
      {
        name: 'main',
        headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        isCurrent: true,
        upstreamName: 'origin/main',
        worktreePath: rootPath,
      },
      {
        name: 'worker/030-repo-registry-scan-service',
        headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        isCurrent: false,
        worktreePath: 'C:/Repos/Codex Orchestrator Worktrees/030',
      },
    ],
    status: {
      isDirty: true,
      entries: [
        {
          path: 'src/application/repoRegistryScan.ts',
          indexStatus: ' ',
          worktreeStatus: 'M',
          kind: 'modified',
        },
      ],
    },
    worktrees: [
      {
        path: rootPath,
        headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        branchName: 'main',
        state: 'branch',
        isBare: false,
        isDetached: false,
        isLocked: false,
        isPrunable: false,
      },
      {
        path: 'C:/Repos/Codex Orchestrator Worktrees/030',
        headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        branchName: 'worker/030-repo-registry-scan-service',
        state: 'branch',
        isBare: false,
        isDetached: false,
        isLocked: true,
        lockReason: 'active worker',
        isPrunable: false,
      },
    ],
    scannedAt,
    ...overrides,
  };
}

function emptyRecords(): DomainRecords {
  return {
    projects: [],
    repos: [],
    branches: [],
    worktrees: [],
    conversations: [],
    tasks: [],
    taskRuns: [],
    artifacts: [],
    validationRuns: [],
    events: [],
  };
}

function recordsWithExistingAndUnrelatedRepos(): DomainRecords {
  const records = emptyRecords();
  records.repos = [
    {
      id: 'repo-1',
      projectId,
      name: 'Codex Orchestrator',
      rootPath: 'C:/Repos/Codex Orchestrator',
      defaultBranch: 'main',
      createdAt: yesterday,
      updatedAt: yesterday,
    },
    {
      id: 'repo-other',
      projectId,
      name: 'Other Repo',
      rootPath: 'C:/Repos/Other Repo',
      defaultBranch: 'main',
      createdAt: yesterday,
      updatedAt: yesterday,
    },
  ];
  records.branches = [
    {
      id: 'branch-main',
      repoId: 'repo-1',
      name: 'main',
      createdAt: yesterday,
      updatedAt: yesterday,
    },
    {
      id: 'branch-worker',
      repoId: 'repo-1',
      name: 'worker/030-repo-registry-scan-service',
      createdAt: yesterday,
      updatedAt: yesterday,
    },
    {
      id: 'branch-old',
      repoId: 'repo-1',
      name: 'worker/old-scan',
      createdAt: yesterday,
      updatedAt: yesterday,
    },
    {
      id: 'branch-other',
      repoId: 'repo-other',
      name: 'main',
      createdAt: yesterday,
      updatedAt: yesterday,
    },
  ];
  records.worktrees = [
    {
      id: 'worktree-main',
      repoId: 'repo-1',
      branchId: 'branch-main',
      path: 'C:/Repos/Codex Orchestrator',
      isMain: true,
      isDirty: false,
      lastScannedAt: yesterday,
      createdAt: yesterday,
      updatedAt: yesterday,
    },
    {
      id: 'worktree-worker',
      repoId: 'repo-1',
      branchId: 'branch-worker',
      path: 'C:/Repos/Codex Orchestrator Worktrees/030',
      isMain: false,
      isDirty: false,
      lastScannedAt: yesterday,
      createdAt: yesterday,
      updatedAt: yesterday,
    },
    {
      id: 'worktree-old',
      repoId: 'repo-1',
      branchId: 'branch-old',
      path: 'C:/Repos/Codex Orchestrator Worktrees/old',
      isMain: false,
      isDirty: false,
      lastScannedAt: yesterday,
      createdAt: yesterday,
      updatedAt: yesterday,
    },
    {
      id: 'worktree-other',
      repoId: 'repo-other',
      branchId: 'branch-other',
      path: 'C:/Repos/Other Repo',
      isMain: true,
      isDirty: false,
      lastScannedAt: yesterday,
      createdAt: yesterday,
      updatedAt: yesterday,
    },
  ];

  return records;
}
