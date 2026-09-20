import type { SessionFolderTarget } from '../../application/agentSessions/organization';
import type { SessionExecutionSelectionDto } from '../../application/executionTargets/contracts';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';

const STORAGE_PREFIX = 'codex-orchestrator/composer-draft/v1/';

export interface CachedComposerDraft {
  readonly text?: string;
  readonly workingDirectory?: string;
  readonly executionSelection?: SessionExecutionSelectionDto | null;
  readonly runtimeSelection?: PerMessageRuntimeSelection;
}

/**
 * A new conversation belongs to its semantic owner, not to the UUID used to render one New action.
 * This makes each repository, workflow, and unattached composer independently recoverable.
 */
export function composerDraftCacheKey(
  sessionId: string | null,
  folderTarget?: SessionFolderTarget | null,
): string {
  if (sessionId) return `session:${sessionId}`;
  if (folderTarget?.kind === 'repository') return `new:repository:${folderTarget.repositoryId}`;
  if (folderTarget?.kind === 'workflow_instance') return `new:workflow:${folderTarget.instanceId}`;
  return 'new:unattached';
}

export function readCachedComposerDraft(key: string): CachedComposerDraft | null {
  const storage = browserStorage();
  if (!storage) return null;
  try {
    const value: unknown = JSON.parse(storage.getItem(STORAGE_PREFIX + key) ?? 'null');
    return isCachedComposerDraft(value) ? value : null;
  } catch {
    return null;
  }
}

export function writeCachedComposerDraft(key: string, patch: CachedComposerDraft): void {
  const storage = browserStorage();
  if (!storage) return;
  try {
    storage.setItem(
      STORAGE_PREFIX + key,
      JSON.stringify({ ...(readCachedComposerDraft(key) ?? {}), ...patch }),
    );
  } catch {
    // Draft recovery must never make the active composer unusable.
  }
}

export function clearCachedComposerDraft(key: string): void {
  const storage = browserStorage();
  if (!storage) return;
  try {
    storage.removeItem(STORAGE_PREFIX + key);
  } catch {
    // The send already succeeded; retaining a recoverable stale draft is safer than failing it.
  }
}

function browserStorage(): Storage | null {
  if (typeof window === 'undefined') return null;
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function isCachedComposerDraft(value: unknown): value is CachedComposerDraft {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const candidate = value as Record<string, unknown>;
  return (
    (candidate.text === undefined || typeof candidate.text === 'string') &&
    (candidate.workingDirectory === undefined || typeof candidate.workingDirectory === 'string') &&
    (candidate.executionSelection === undefined ||
      candidate.executionSelection === null ||
      typeof candidate.executionSelection === 'object') &&
    (candidate.runtimeSelection === undefined ||
      (candidate.runtimeSelection !== null && typeof candidate.runtimeSelection === 'object'))
  );
}
