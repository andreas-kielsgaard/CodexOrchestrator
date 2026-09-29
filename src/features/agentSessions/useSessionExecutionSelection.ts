import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  AgentSessionProfileClient,
  PinnedAgentSessionProfileDto,
} from '../../application/agentSessions';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';
import type { SessionFolderTarget } from '../../application/agentSessions/organization';
import {
  composerDraftCacheKey,
  readLastSentRuntimeSelection,
  readCachedComposerDraft,
  writeLastSentRuntimeSelection,
  writeCachedComposerDraft,
} from './composerDraftCache';

const inherited = (): PerMessageRuntimeSelection => ({ model: null, reasoningMode: null });

/** Shared configuration read and message-local selection state for every profiled Session view. */
export function useSessionExecutionSelection(
  client: AgentSessionProfileClient | undefined,
  sessionId: string | null,
  draftId?: string,
  folderTarget?: SessionFolderTarget | null,
) {
  const [profile, setProfile] = useState<PinnedAgentSessionProfileDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selection, setSelection] = useState(inherited);
  const [hydratedCacheKey, setHydratedCacheKey] = useState<string | null>(null);
  const currentSession = useRef(sessionId ?? draftId);
  currentSession.current = sessionId ?? draftId;
  const cacheKey = composerDraftCacheKey(sessionId, folderTarget);
  const previousContext = useRef<string | null>(null);
  useEffect(() => {
    let active = true;
    setProfile(null);
    setError(null);
    setHydratedCacheKey(null);
    const cached = readCachedComposerDraft(cacheKey)?.runtimeSelection;
    if (previousContext.current !== cacheKey) {
      setSelection(
        cached ?? (sessionId ? inherited() : readLastSentRuntimeSelection() ?? inherited()),
      );
    }
    previousContext.current = cacheKey;
    setHydratedCacheKey(cacheKey);
    if (client && sessionId)
      void client.loadPinnedProfile(sessionId).then(
        (value) => {
          if (active) setProfile(value);
        },
        (cause) => {
          if (active) setError(cause instanceof Error ? cause.message : String(cause));
        },
      );
    return () => {
      active = false;
    };
  }, [client, sessionId, draftId, cacheKey]);
  useEffect(() => {
    if (hydratedCacheKey !== cacheKey) return;
    writeCachedComposerDraft(cacheKey, { runtimeSelection: selection });
  }, [cacheKey, hydratedCacheKey, selection]);
  const reloadProfile = useCallback(async () => {
    if (!client || !sessionId) return;
    try {
      const value = await client.loadPinnedProfile(sessionId);
      if (currentSession.current === sessionId) {
        setProfile(value);
        setError(null);
      }
    } catch (cause) {
      if (currentSession.current === sessionId)
        setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, [client, sessionId]);
  const afterAccepted = (
    acceptedSessionId: string,
    acceptedSelection: PerMessageRuntimeSelection,
  ) => {
    if (currentSession.current !== (sessionId ?? draftId)) return;
    writeCachedComposerDraft(composerDraftCacheKey(acceptedSessionId), {
      runtimeSelection: acceptedSelection,
    });
    writeLastSentRuntimeSelection(acceptedSelection);
    setSelection(acceptedSelection);
  };
  return {
    profile,
    error,
    reloadProfile,
    selection,
    setSelection,
    execution: client ? { client, selection, setSelection, afterAccepted } : undefined,
  };
}
