import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  CapabilityProfileDto,
  ExecutionConfigurationClient,
  RuntimeProfileSnapshotDto,
} from '../../application/executionConfiguration';
import {
  localExecutionBinding,
  type ExecutionTargetClient,
  type SessionExecutionSelectionDto,
  type SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
import type { SessionFolderTarget } from '../../application/agentSessions/organization';
import type { RepositoryBranchSource } from '../../application/branches';
import { resolveDefaultDraftTarget, type DraftBranchChoice } from './draftTargetResolver';
import {
  composerDraftCacheKey,
  readCachedComposerDraft,
  writeCachedComposerDraft,
} from './composerDraftCache';

export function selectionForTarget(
  target: SessionExecutionTargetDto,
): SessionExecutionSelectionDto {
  return {
    capabilityProfileId: target.capabilityProfileId,
    capabilityProfileRevision: target.capabilityProfileRevision,
    execution: target.execution,
    workspace: { kind: 'existing', target },
  };
}
export function useSessionTarget(
  client: ExecutionTargetClient | undefined,
  profiles: ExecutionConfigurationClient | undefined,
  sessionId: string | null,
  draftId?: string,
  folderTarget?: SessionFolderTarget | null,
  branchSource?: RepositoryBranchSource,
  repositoryId?: string | null,
) {
  const [selection, setSelectionState] = useState<SessionExecutionSelectionDto | null>(null);
  const [hydratedCacheKey, setHydratedCacheKey] = useState<string | null>(null);
  const [availableProfiles, setAvailableProfiles] = useState<readonly CapabilityProfileDto[]>([]);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [runtime, setRuntime] = useState<RuntimeProfileSnapshotDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [branchChoice, setBranchChoice] = useState<DraftBranchChoice | null>(null);
  const [resolvingBranch, setResolvingBranch] = useState(false);
  const dirty = useRef(false);
  const acknowledgedSession = useRef<string | null>(null);
  const cacheKey = composerDraftCacheKey(sessionId, folderTarget);
  const contextRef = useRef<string | undefined>(undefined);
  useEffect(() => {
    if (contextRef.current === cacheKey) return;
    contextRef.current = cacheKey;
    setHydratedCacheKey(null);
    if (sessionId && sessionId === acknowledgedSession.current) {
      acknowledgedSession.current = null;
      setHydratedCacheKey(cacheKey);
      return;
    }
    const cachedSelection = readCachedComposerDraft(cacheKey)?.executionSelection ?? null;
    const cachedBranchChoice = readCachedComposerDraft(cacheKey)?.branchChoice ?? null;
    dirty.current = cachedSelection !== null;
    setSelectionState(cachedSelection);
    setBranchChoice(cachedBranchChoice);
    setDeviceId(cachedSelection?.execution.deviceId ?? null);
    setHydratedCacheKey(cacheKey);
  }, [sessionId, cacheKey]);
  useEffect(() => {
    let current = true;
    if (!profiles) return;
    void Promise.all([
      profiles.listCapabilityProfiles(),
      profiles.loadDefaultCapabilityProfile?.() ?? Promise.resolve(null),
    ]).then(
      ([items, defaultId]) => {
        if (!current) return;
        setAvailableProfiles(items);
        if (!sessionId && !dirty.current) {
          const selected = items.find((item) => item.capabilityProfileId === defaultId);
          if (selected) {
            const execution = selected.execution ?? localExecutionBinding;
            setSelectionState({
              capabilityProfileId: selected.capabilityProfileId,
              capabilityProfileRevision: selected.revision,
              execution,
              workspace: { kind: 'auxiliary' },
            });
            setDeviceId(execution.deviceId);
          }
        }
      },
      (cause) => {
        if (current) setError(String(cause));
      },
    );
    return () => {
      current = false;
    };
  }, [profiles, sessionId, draftId]);
  const setSelection = useCallback((next: SessionExecutionSelectionDto | null) => {
    dirty.current = true;
    setSelectionState(next);
    if (!next || next.workspace.kind !== 'auxiliary') setBranchChoice(null);
    if (next) setDeviceId(next.execution.deviceId);
  }, []);
  useEffect(() => {
    if (hydratedCacheKey !== cacheKey) return;
    writeCachedComposerDraft(cacheKey, { executionSelection: selection, branchChoice });
  }, [branchChoice, cacheKey, hydratedCacheKey, selection]);
  const setTarget = useCallback(
    (target: SessionExecutionTargetDto | null) =>
      setSelection(target ? selectionForTarget(target) : null),
    [setSelection],
  );
  const adoptCurrent = useCallback(
    (target: SessionExecutionTargetDto | null, accepted?: SessionExecutionSelectionDto | null) => {
      if (dirty.current) return;
      const next = target ? selectionForTarget(target) : accepted;
      if (next) {
        setSelectionState((previous) =>
          JSON.stringify(previous) === JSON.stringify(next) ? previous : next,
        );
        setDeviceId(next.execution.deviceId);
      }
    },
    [],
  );
  const chooseProfile = useCallback(
    (id: string) => {
      const profile = availableProfiles.find((item) => item.capabilityProfileId === id);
      if (!profile) return;
      const execution = profile.execution ?? localExecutionBinding;
      const workspace =
        selection?.execution.deviceId === execution.deviceId
          ? selection.workspace
          : { kind: 'auxiliary' as const };
      const next = {
        capabilityProfileId: id,
        capabilityProfileRevision: profile.revision,
        execution,
      };
      setBranchChoice(null);
      setSelection({
        ...next,
        workspace:
          workspace.kind === 'existing'
            ? { kind: 'existing', target: { ...workspace.target, ...next } }
            : workspace,
      });
    },
    [availableProfiles, selection, setSelection],
  );
  const chooseDevice = useCallback(
    (id: string) => {
      const matches = availableProfiles.filter(
        (item) => (item.execution ?? localExecutionBinding).deviceId === id,
      );
      const match =
        matches.find((item) => item.capabilityProfileId === selection?.capabilityProfileId) ??
        (matches.length === 1 ? matches[0] : null);
      if (match) chooseProfile(match.capabilityProfileId);
      else {
        setSelection(null);
        setDeviceId(id);
      }
    },
    [availableProfiles, chooseProfile, selection, setSelection],
  );
  const target = selection?.workspace.kind === 'existing' ? selection.workspace.target : null;
  const profile =
    availableProfiles.find((item) => item.capabilityProfileId === selection?.capabilityProfileId) ??
    null;
  const branchResolutionKey =
    !sessionId && repositoryId && selection?.workspace.kind === 'auxiliary' && !branchChoice
      ? `${cacheKey}:${repositoryId}:${selection.capabilityProfileId}:${selection.execution.deviceId}`
      : null;
  useEffect(() => {
    let current = true;
    if (
      !branchResolutionKey ||
      !repositoryId ||
      !selection ||
      !client ||
      !branchSource ||
      hydratedCacheKey !== cacheKey
    )
      return;
    setResolvingBranch(true);
    setError(null);
    void resolveDefaultDraftTarget({
      source: branchSource,
      client,
      repositoryId,
      selection,
    })
      .then(
        (resolved) => {
          if (!current) return;
          setSelectionState(resolved.selection);
          setBranchChoice(resolved.branchChoice);
        },
        (cause) => {
          if (current) setError(String(cause));
        },
      )
      .finally(() => {
        if (current) setResolvingBranch(false);
      });
    return () => {
      current = false;
    };
  }, [
    branchResolutionKey,
    branchSource,
    cacheKey,
    client,
    hydratedCacheKey,
    repositoryId,
    selection,
  ]);
  useEffect(() => {
    let current = true;
    setRuntime(null);
    setError(null);
    setLoading(false);
    if (!selection || !client || branchResolutionKey) return;
    setLoading(true);
    void client
      .loadRuntime(selection.execution, target?.path)
      .then(
        (facts) => {
          if (current) setRuntime(facts.runtimeProfile);
        },
        (cause) => {
          if (current) setError(String(cause));
        },
      )
      .finally(() => {
        if (current) setLoading(false);
      });
    return () => {
      current = false;
    };
  }, [selection, target?.path, client, branchResolutionKey]);
  const acceptReady = useCallback(
    (accepted: SessionExecutionSelectionDto | null, target: SessionExecutionTargetDto | null) => {
      if (target)
        setSelectionState((current) =>
          JSON.stringify(current) === JSON.stringify(accepted)
            ? selectionForTarget(target)
            : current,
        );
    },
    [],
  );
  const preserveOnAcknowledgement = useCallback((id: string) => {
    acknowledgedSession.current = id;
  }, []);
  return {
    target,
    setTarget,
    selection,
    setSelection,
    runtime,
    profile,
    availableProfiles,
    deviceId,
    chooseDevice,
    chooseProfile,
    loading: loading || resolvingBranch,
    error,
    branchChoice,
    workspaceLabel:
      branchChoice?.label ??
      (selection?.workspace.kind === 'auxiliary' && !repositoryId ? 'Empty workspace' : undefined),
    adoptCurrent,
    acceptReady,
    preserveOnAcknowledgement,
  };
}
