import type {
  GitBranchSummary,
  GitRemoteSummary,
  GitStatusCode,
  GitStatusEntryKind,
  GitStatusSnapshot,
  GitWorktreeState,
  GitWorktreeSummary,
} from './types';

const NUL = '\0';
const UNIT_SEPARATOR = '\x1f';

export const gitBranchSummaryFormat = [
  '%(HEAD)',
  '%(refname:short)',
  '%(objectname)',
  '%(upstream:short)',
  '%(upstream:track)',
  '%(worktreepath)',
].join('%x1f');

export const gitBranchSummaryArgs = ['branch', `--format=${gitBranchSummaryFormat}%x00`];

export function parseGitStatusPorcelainV1Z(output: string): GitStatusSnapshot {
  const tokens = splitNul(output);
  const entries = [];

  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token.length < 3) {
      continue;
    }

    const indexStatus = parseStatusCode(token[0]);
    const worktreeStatus = parseStatusCode(token[1]);
    const path = normalizeGitPath(token.slice(3));
    const entry = {
      path,
      indexStatus,
      worktreeStatus,
      kind: statusKind(indexStatus, worktreeStatus),
    };

    if (indexStatus === 'R' || indexStatus === 'C') {
      const originalPath = tokens[index + 1];
      if (originalPath !== undefined) {
        entries.push({
          ...entry,
          originalPath: normalizeGitPath(originalPath),
        });
        index += 1;
        continue;
      }
    }

    entries.push(entry);
  }

  return {
    entries,
    isDirty: entries.length > 0,
  };
}

export function parseGitBranchSummary(output: string): GitBranchSummary[] {
  return splitNul(output).map((record) => {
    const [headMarker, name, headSha, upstreamName, upstreamTrack, worktreePath] =
      record.split(UNIT_SEPARATOR);

    return {
      name,
      headSha,
      isCurrent: headMarker === '*',
      ...(upstreamName ? { upstreamName } : {}),
      ...(upstreamTrack ? { upstreamTrack } : {}),
      ...(worktreePath ? { worktreePath: normalizeGitPath(worktreePath) } : {}),
    };
  });
}

export function parseGitRemoteVerbose(output: string): GitRemoteSummary[] {
  const remotesByName = new Map<string, GitRemoteSummary>();

  for (const line of output.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }

    const match = /^(?<name>\S+)\s+(?<url>.+?)\s+\((?<direction>fetch|push)\)$/.exec(trimmed);
    if (!match?.groups) {
      continue;
    }

    const { name, url, direction } = match.groups;
    const remote = remotesByName.get(name) ?? { name };

    if (direction === 'fetch') {
      remote.fetchUrl = url;
    } else {
      remote.pushUrl = url;
    }

    remotesByName.set(name, remote);
  }

  return [...remotesByName.values()];
}

export function parseGitWorktreeListPorcelainZ(output: string): GitWorktreeSummary[] {
  const worktrees: GitWorktreeSummary[] = [];
  let current: Partial<GitWorktreeSummary> | undefined;

  for (const token of splitNul(output)) {
    if (token.startsWith('worktree ')) {
      if (current?.path) {
        worktrees.push(finalizeWorktree(current));
      }
      current = {
        path: normalizeGitPath(token.slice('worktree '.length)),
        state: 'detached',
        isBare: false,
        isDetached: true,
        isLocked: false,
        isPrunable: false,
      };
      continue;
    }

    if (!current) {
      continue;
    }

    if (token.startsWith('HEAD ')) {
      current.headSha = token.slice('HEAD '.length);
    } else if (token.startsWith('branch ')) {
      current.branchName = shortBranchName(token.slice('branch '.length));
      current.state = 'branch';
      current.isDetached = false;
    } else if (token === 'bare') {
      current.state = 'bare';
      current.isBare = true;
      current.isDetached = false;
    } else if (token === 'detached') {
      current.state = 'detached';
      current.isDetached = true;
    } else if (token === 'locked' || token.startsWith('locked ')) {
      current.isLocked = true;
      current.lockReason = valueAfterKeyword(token, 'locked');
    } else if (token === 'prunable' || token.startsWith('prunable ')) {
      current.isPrunable = true;
      current.pruneReason = valueAfterKeyword(token, 'prunable');
    }
  }

  if (current?.path) {
    worktrees.push(finalizeWorktree(current));
  }

  return worktrees;
}

export function normalizeGitPath(path: string): string {
  return path.replace(/\\/g, '/');
}

function splitNul(output: string): string[] {
  return output.split(NUL).filter((token) => token.length > 0);
}

function parseStatusCode(code: string): GitStatusCode {
  const knownCodes = new Set([' ', '!', '?', 'A', 'C', 'D', 'M', 'R', 'T', 'U']);
  if (!knownCodes.has(code)) {
    return ' ';
  }
  return code as GitStatusCode;
}

function statusKind(indexStatus: GitStatusCode, worktreeStatus: GitStatusCode): GitStatusEntryKind {
  const statuses = new Set([indexStatus, worktreeStatus]);

  if (indexStatus === '?' && worktreeStatus === '?') {
    return 'untracked';
  }
  if (indexStatus === '!' && worktreeStatus === '!') {
    return 'ignored';
  }
  if (isUnmergedStatusPair(indexStatus, worktreeStatus)) {
    return 'unmerged';
  }
  if (statuses.has('R')) {
    return 'renamed';
  }
  if (statuses.has('C')) {
    return 'copied';
  }
  if (statuses.has('A')) {
    return 'added';
  }
  if (statuses.has('D')) {
    return 'deleted';
  }
  if (statuses.has('T')) {
    return 'type_changed';
  }
  if (statuses.has('M')) {
    return 'modified';
  }

  return 'unknown';
}

function isUnmergedStatusPair(indexStatus: GitStatusCode, worktreeStatus: GitStatusCode): boolean {
  return ['DD', 'AU', 'UD', 'UA', 'DU', 'AA', 'UU'].includes(`${indexStatus}${worktreeStatus}`);
}

function shortBranchName(refName: string): string {
  return refName.startsWith('refs/heads/') ? refName.slice('refs/heads/'.length) : refName;
}

function valueAfterKeyword(token: string, keyword: string): string | undefined {
  return token.length > keyword.length ? token.slice(keyword.length + 1) : undefined;
}

function finalizeWorktree(worktree: Partial<GitWorktreeSummary>): GitWorktreeSummary {
  const state: GitWorktreeState = worktree.state ?? 'detached';

  return {
    path: worktree.path ?? '',
    headSha: worktree.headSha,
    branchName: worktree.branchName,
    state,
    isBare: worktree.isBare ?? state === 'bare',
    isDetached: worktree.isDetached ?? state === 'detached',
    isLocked: worktree.isLocked ?? false,
    lockReason: worktree.lockReason,
    isPrunable: worktree.isPrunable ?? false,
    pruneReason: worktree.pruneReason,
  };
}
