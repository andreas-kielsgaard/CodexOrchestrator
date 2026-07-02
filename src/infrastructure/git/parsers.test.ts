import {
  gitBranchSummaryFormat,
  parseGitBranchSummary,
  parseGitRemoteVerbose,
  parseGitStatusPorcelainV1Z,
  parseGitWorktreeListPorcelainZ,
} from './parsers';

describe('parseGitStatusPorcelainV1Z', () => {
  it('normalizes status entries from porcelain v1 z output', () => {
    const output = [
      ' M src/app/App.tsx',
      'A  src/infrastructure/git/parsers.ts',
      '?? docs/task-logs/worker-003.md',
      '!! dist/bundle.js',
      '',
    ].join('\0');

    expect(parseGitStatusPorcelainV1Z(output)).toEqual({
      isDirty: true,
      entries: [
        {
          path: 'src/app/App.tsx',
          indexStatus: ' ',
          worktreeStatus: 'M',
          kind: 'modified',
        },
        {
          path: 'src/infrastructure/git/parsers.ts',
          indexStatus: 'A',
          worktreeStatus: ' ',
          kind: 'added',
        },
        {
          path: 'docs/task-logs/worker-003.md',
          indexStatus: '?',
          worktreeStatus: '?',
          kind: 'untracked',
        },
        {
          path: 'dist/bundle.js',
          indexStatus: '!',
          worktreeStatus: '!',
          kind: 'ignored',
        },
      ],
    });
  });

  it('handles z-format rename entries and Windows-style paths', () => {
    const output = ['R  src/new name.ts', 'src\\old name.ts', ''].join('\0');

    expect(parseGitStatusPorcelainV1Z(output).entries).toEqual([
      {
        path: 'src/new name.ts',
        originalPath: 'src/old name.ts',
        indexStatus: 'R',
        worktreeStatus: ' ',
        kind: 'renamed',
      },
    ]);
  });

  it('returns a clean snapshot for empty output', () => {
    expect(parseGitStatusPorcelainV1Z('')).toEqual({
      entries: [],
      isDirty: false,
    });
  });

  it('classifies all porcelain v1 unmerged status pairs as unmerged', () => {
    const output = [
      'DD conflict-both-deleted.txt',
      'AU conflict-added-by-us.txt',
      'UD conflict-deleted-by-them.txt',
      'UA conflict-added-by-them.txt',
      'DU conflict-deleted-by-us.txt',
      'AA conflict-both-added.txt',
      'UU conflict-both-modified.txt',
      '',
    ].join('\0');

    expect(parseGitStatusPorcelainV1Z(output).entries).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ path: 'conflict-both-deleted.txt', kind: 'unmerged' }),
        expect.objectContaining({ path: 'conflict-added-by-us.txt', kind: 'unmerged' }),
        expect.objectContaining({ path: 'conflict-deleted-by-them.txt', kind: 'unmerged' }),
        expect.objectContaining({ path: 'conflict-added-by-them.txt', kind: 'unmerged' }),
        expect.objectContaining({ path: 'conflict-deleted-by-us.txt', kind: 'unmerged' }),
        expect.objectContaining({ path: 'conflict-both-added.txt', kind: 'unmerged' }),
        expect.objectContaining({ path: 'conflict-both-modified.txt', kind: 'unmerged' }),
      ]),
    );
  });
});

describe('parseGitBranchSummary', () => {
  it('parses the documented branch summary format', () => {
    expect(gitBranchSummaryFormat).toContain('%(refname:short)');
    expect(gitBranchSummaryFormat).toContain('%1f');

    const output = [
      [
        '*',
        'main',
        'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        'origin/main',
        '[ahead 1]',
        'C:\\Repos\\App',
      ].join('\x1f'),
      [
        ' ',
        'worker/003-git-adapter-foundation',
        'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        '',
        '',
        'C:\\Repos\\App Worktrees\\003',
      ].join('\x1f'),
      '',
    ].join('\0');

    expect(parseGitBranchSummary(output)).toEqual([
      {
        name: 'main',
        headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        isCurrent: true,
        upstreamName: 'origin/main',
        upstreamTrack: '[ahead 1]',
        worktreePath: 'C:/Repos/App',
      },
      {
        name: 'worker/003-git-adapter-foundation',
        headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        isCurrent: false,
        worktreePath: 'C:/Repos/App Worktrees/003',
      },
    ]);
  });

  it('accepts git branch formatted records with newlines around NUL delimiters', () => {
    const output = [
      [
        ' ',
        'main',
        'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        'origin/main',
        '[gone]',
        'C:\\Repos\\App',
      ].join('\x1f'),
      '\n',
      [
        '*',
        'worker/035-local-git-runtime-adapters',
        'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        '',
        '',
        'C:\\Repos\\App Worktrees\\035',
      ].join('\x1f'),
      '\n',
    ].join('\0');

    expect(parseGitBranchSummary(output)).toEqual([
      {
        name: 'main',
        headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        isCurrent: false,
        upstreamName: 'origin/main',
        upstreamTrack: '[gone]',
        worktreePath: 'C:/Repos/App',
      },
      {
        name: 'worker/035-local-git-runtime-adapters',
        headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        isCurrent: true,
        worktreePath: 'C:/Repos/App Worktrees/035',
      },
    ]);
  });
});

describe('parseGitRemoteVerbose', () => {
  it('groups fetch and push URLs by remote name', () => {
    const output = [
      'origin\tgit@github.com:andreas-kielsgaard/CodexOrchestrator.git (fetch)',
      'origin\tgit@github.com:andreas-kielsgaard/CodexOrchestrator.git (push)',
      'upstream\thttps://github.com/openai/example.git (fetch)',
      'upstream\tgit@github.com:openai/example.git (push)',
      'backup\tssh://backup.example.com/repos/orchestrator.git (push)',
      '',
    ].join('\n');

    expect(parseGitRemoteVerbose(output)).toEqual([
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
      {
        name: 'backup',
        pushUrl: 'ssh://backup.example.com/repos/orchestrator.git',
      },
    ]);
  });

  it('ignores non-remote lines and supports empty output', () => {
    expect(parseGitRemoteVerbose('not remote output\n')).toEqual([]);
    expect(parseGitRemoteVerbose('')).toEqual([]);
  });
});

describe('parseGitWorktreeListPorcelainZ', () => {
  it('parses branch, detached, locked, and prunable worktrees', () => {
    const output = [
      'worktree C:\\Repos\\App',
      'HEAD aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      'branch refs/heads/main',
      'worktree C:\\Repos\\App Worktrees\\003',
      'HEAD bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
      'branch refs/heads/worker/003-git-adapter-foundation',
      'locked orchestrator task active',
      'worktree C:\\Repos\\App Worktrees\\detached',
      'HEAD cccccccccccccccccccccccccccccccccccccccc',
      'detached',
      'prunable gitdir file points to missing location',
      '',
    ].join('\0');

    expect(parseGitWorktreeListPorcelainZ(output)).toEqual([
      {
        path: 'C:/Repos/App',
        headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        branchName: 'main',
        state: 'branch',
        isBare: false,
        isDetached: false,
        isLocked: false,
        isPrunable: false,
      },
      {
        path: 'C:/Repos/App Worktrees/003',
        headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        branchName: 'worker/003-git-adapter-foundation',
        state: 'branch',
        isBare: false,
        isDetached: false,
        isLocked: true,
        lockReason: 'orchestrator task active',
        isPrunable: false,
      },
      {
        path: 'C:/Repos/App Worktrees/detached',
        headSha: 'cccccccccccccccccccccccccccccccccccccccc',
        state: 'detached',
        isBare: false,
        isDetached: true,
        isLocked: false,
        isPrunable: true,
        pruneReason: 'gitdir file points to missing location',
      },
    ]);
  });

  it('parses bare worktrees', () => {
    const output = ['worktree C:\\Repos\\Bare.git', 'bare', ''].join('\0');

    expect(parseGitWorktreeListPorcelainZ(output)).toEqual([
      {
        path: 'C:/Repos/Bare.git',
        state: 'bare',
        isBare: true,
        isDetached: false,
        isLocked: false,
        isPrunable: false,
      },
    ]);
  });
});
