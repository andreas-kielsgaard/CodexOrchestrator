import { buildGitRepoScanResult, mapGitRepoScanToDomainFacts } from './gitAdapter';

describe('buildGitRepoScanResult', () => {
  it('assembles a repo scan from raw Git command outputs', () => {
    const scannedAt = '2026-07-01T10:00:00.000Z';
    const scan = buildGitRepoScanResult({
      rootPath: 'C:\\Users\\user\\Documents\\Code Projects\\Codex Orchestrator',
      defaultBranch: 'main',
      scannedAt,
      outputs: {
        remoteVerbose: [
          'origin\tgit@github.com:andreas-kielsgaard/CodexOrchestrator.git (fetch)',
          'origin\tgit@github.com:andreas-kielsgaard/CodexOrchestrator.git (push)',
          'upstream\thttps://github.com/openai/example.git (fetch)',
          'upstream\tgit@github.com:openai/example.git (push)',
          '',
        ].join('\n'),
        statusPorcelainV1Z: [' M src/app/App.tsx', '?? docs/task-logs/worker-004.md', ''].join(
          '\0',
        ),
        branchSummary: [
          [
            '*',
            'main',
            'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            'origin/main',
            '',
            'C:\\Users\\user\\Documents\\Code Projects\\Codex Orchestrator',
          ].join('\x1f'),
          [
            ' ',
            'worker/004-git-scan-mapping',
            'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
            '',
            '',
            'C:\\Users\\user\\.codex\\worktrees\\fe6a\\Codex Orchestrator',
          ].join('\x1f'),
          [
            ' ',
            'review/detached-context',
            'cccccccccccccccccccccccccccccccccccccccc',
            '',
            '',
            '',
          ].join('\x1f'),
          '',
        ].join('\0'),
        worktreeListPorcelainZ: [
          'worktree C:\\Users\\user\\Documents\\Code Projects\\Codex Orchestrator',
          'HEAD aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          'branch refs/heads/main',
          'worktree C:\\Users\\user\\.codex\\worktrees\\fe6a\\Codex Orchestrator',
          'HEAD bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
          'branch refs/heads/worker/004-git-scan-mapping',
          'locked active worker',
          'worktree C:\\Users\\user\\.codex\\worktrees\\detached',
          'HEAD cccccccccccccccccccccccccccccccccccccccc',
          'detached',
          'prunable gitdir file points to missing location',
          '',
        ].join('\0'),
      },
    });

    expect(scan).toEqual({
      rootPath: 'C:/Users/user/Documents/Code Projects/Codex Orchestrator',
      currentBranch: 'main',
      defaultBranch: 'main',
      scannedAt,
      remotes: [
        {
          name: 'origin',
          fetchUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
          pushUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
        },
        {
          name: 'upstream',
          fetchUrl: 'https://github.com/openai/example.git',
          pushUrl: 'git@github.com:openai/example.git',
        },
      ],
      status: {
        isDirty: true,
        entries: [
          {
            path: 'src/app/App.tsx',
            indexStatus: ' ',
            worktreeStatus: 'M',
            kind: 'modified',
          },
          {
            path: 'docs/task-logs/worker-004.md',
            indexStatus: '?',
            worktreeStatus: '?',
            kind: 'untracked',
          },
        ],
      },
      branches: [
        {
          name: 'main',
          headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          isCurrent: true,
          upstreamName: 'origin/main',
          worktreePath: 'C:/Users/user/Documents/Code Projects/Codex Orchestrator',
        },
        {
          name: 'worker/004-git-scan-mapping',
          headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
          isCurrent: false,
          worktreePath: 'C:/Users/user/.codex/worktrees/fe6a/Codex Orchestrator',
        },
        {
          name: 'review/detached-context',
          headSha: 'cccccccccccccccccccccccccccccccccccccccc',
          isCurrent: false,
        },
      ],
      worktrees: [
        {
          path: 'C:/Users/user/Documents/Code Projects/Codex Orchestrator',
          headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          branchName: 'main',
          state: 'branch',
          isBare: false,
          isDetached: false,
          isLocked: false,
          isPrunable: false,
        },
        {
          path: 'C:/Users/user/.codex/worktrees/fe6a/Codex Orchestrator',
          headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
          branchName: 'worker/004-git-scan-mapping',
          state: 'branch',
          isBare: false,
          isDetached: false,
          isLocked: true,
          lockReason: 'active worker',
          isPrunable: false,
        },
        {
          path: 'C:/Users/user/.codex/worktrees/detached',
          headSha: 'cccccccccccccccccccccccccccccccccccccccc',
          state: 'detached',
          isBare: false,
          isDetached: true,
          isLocked: false,
          isPrunable: true,
          pruneReason: 'gitdir file points to missing location',
        },
      ],
    });
  });
});

describe('mapGitRepoScanToDomainFacts', () => {
  it('maps scan facts into domain-facing repo, branch, and worktree facts', () => {
    const scan = buildGitRepoScanResult({
      rootPath: 'C:\\Repos\\Codex Orchestrator',
      scannedAt: '2026-07-01T10:00:00.000Z',
      outputs: {
        remoteVerbose: [
          'origin\tgit@github.com:andreas-kielsgaard/CodexOrchestrator.git (fetch)',
          'origin\tgit@github.com:andreas-kielsgaard/CodexOrchestrator.git (push)',
          '',
        ].join('\n'),
        statusPorcelainV1Z: '',
        branchSummary: [
          [
            ' ',
            'worker/004-git-scan-mapping',
            'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
            '',
            '',
            'C:\\Repos\\Codex Orchestrator Worktrees\\004',
          ].join('\x1f'),
          '',
        ].join('\0'),
        worktreeListPorcelainZ: [
          'worktree C:\\Repos\\Codex Orchestrator',
          'HEAD aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          'branch refs/heads/main',
          'worktree C:\\Repos\\Codex Orchestrator Worktrees\\004',
          'HEAD bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
          'branch refs/heads/worker/004-git-scan-mapping',
          '',
        ].join('\0'),
      },
    });

    expect(mapGitRepoScanToDomainFacts(scan)).toEqual({
      repo: {
        name: 'Codex Orchestrator',
        rootPath: 'C:/Repos/Codex Orchestrator',
        remoteUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
      },
      branches: [
        {
          name: 'worker/004-git-scan-mapping',
          headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
          isCurrent: false,
          worktreePath: 'C:/Repos/Codex Orchestrator Worktrees/004',
        },
      ],
      worktrees: [
        {
          path: 'C:/Repos/Codex Orchestrator',
          branchName: 'main',
          headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          isMain: true,
          dirtyState: 'clean',
          isDirty: false,
          isBare: false,
          isDetached: false,
          isLocked: false,
          isPrunable: false,
          lastScannedAt: '2026-07-01T10:00:00.000Z',
        },
        {
          path: 'C:/Repos/Codex Orchestrator Worktrees/004',
          branchName: 'worker/004-git-scan-mapping',
          headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
          isMain: false,
          dirtyState: 'unknown',
          isDirty: false,
          isBare: false,
          isDetached: false,
          isLocked: false,
          isPrunable: false,
          lastScannedAt: '2026-07-01T10:00:00.000Z',
        },
      ],
    });
  });

  it('does not invent a default branch when scan facts cannot identify one', () => {
    const scan = buildGitRepoScanResult({
      rootPath: 'C:\\Repos\\Detached Only',
      scannedAt: '2026-07-01T10:00:00.000Z',
      outputs: {
        remoteVerbose: '',
        statusPorcelainV1Z: '',
        branchSummary: '',
        worktreeListPorcelainZ: [
          'worktree C:\\Repos\\Detached Only',
          'HEAD cccccccccccccccccccccccccccccccccccccccc',
          'detached',
          '',
        ].join('\0'),
      },
    });

    expect(mapGitRepoScanToDomainFacts(scan).repo).toEqual({
      name: 'Detached Only',
      rootPath: 'C:/Repos/Detached Only',
    });
  });
});
