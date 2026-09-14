import { useEffect, useState } from 'react';
import type {
  CapabilityProfileDto,
  ExecutionConfigurationClient,
  RuntimeProfileSnapshotDto,
} from '../../application/executionConfiguration';
import type {
  ExecutionTargetClient,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';

export function useSessionTarget(
  client: ExecutionTargetClient | undefined,
  profiles: ExecutionConfigurationClient | undefined,
  sessionId: string | null,
  draftId?: string,
) {
  const [target, setTarget] = useState<SessionExecutionTargetDto | null>(null);
  const [runtime, setRuntime] = useState<RuntimeProfileSnapshotDto | null>(null);
  const [profile, setProfile] = useState<CapabilityProfileDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setTarget(null);
  }, [sessionId, draftId]);
  useEffect(() => {
    let current = true;
    setRuntime(null);
    setProfile(null);
    setError(null);
    setLoading(false);
    if (!target || !client || !profiles || sessionId) return;
    setLoading(true);
    void Promise.all([
      client.loadRuntime(target.execution, target.path),
      profiles.loadCapabilityProfile(target.capabilityProfileId),
    ])
      .then(
        ([facts, saved]) => {
          if (current) {
            setRuntime(facts.runtimeProfile);
            setProfile(saved);
          }
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
  }, [target, client, profiles, sessionId]);
  return { target, setTarget, runtime, profile, loading, error };
}
