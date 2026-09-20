import { afterEach, describe, expect, it } from 'vitest';
import {
  clearCachedComposerDraft,
  composerDraftCacheKey,
  readCachedComposerDraft,
  writeCachedComposerDraft,
} from './composerDraftCache';

afterEach(() => window.localStorage.clear());

describe('composer draft cache', () => {
  it('uses semantic conversation owners instead of transient new-conversation IDs', () => {
    expect(composerDraftCacheKey(null, { kind: 'repository', repositoryId: 'orchestrator' })).toBe(
      'new:repository:orchestrator',
    );
    expect(composerDraftCacheKey(null, { kind: 'workflow_instance', instanceId: 'review' })).toBe(
      'new:workflow:review',
    );
    expect(composerDraftCacheKey(null, null)).toBe('new:unattached');
    expect(composerDraftCacheKey('session-42')).toBe('session:session-42');
  });

  it('keeps each semantic draft and its execution choices separate', () => {
    const repository = composerDraftCacheKey(null, {
      kind: 'repository',
      repositoryId: 'orchestrator',
    });
    const unattached = composerDraftCacheKey(null, null);
    writeCachedComposerDraft(repository, {
      text: 'Review the profile routes',
      runtimeSelection: { model: 'gpt-5.6', reasoningMode: 'high' },
    });
    writeCachedComposerDraft(unattached, {
      text: 'Investigate separately',
      workingDirectory: 'C:/scratch',
    });

    expect(readCachedComposerDraft(repository)).toEqual({
      text: 'Review the profile routes',
      runtimeSelection: { model: 'gpt-5.6', reasoningMode: 'high' },
    });
    expect(readCachedComposerDraft(unattached)).toEqual({
      text: 'Investigate separately',
      workingDirectory: 'C:/scratch',
    });

    clearCachedComposerDraft(repository);
    expect(readCachedComposerDraft(repository)).toBeNull();
    expect(readCachedComposerDraft(unattached)?.text).toBe('Investigate separately');
  });
});
