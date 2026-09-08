import { useCallback, useEffect, useState } from 'react';
import type {
  CapabilityProfileDto,
  ExecutionConfigurationClient,
} from '../../application/executionConfiguration';
import type { IdentityManagementClient } from '../../application/identities';
import { runtimeProfileViewModel } from './presentation';
import type {
  AgentIdentityOption,
  CapabilityProfileOption,
  RuntimeProfileViewModel,
} from './types';

/** Loads the runtime, capability profiles and identities used by configuration consumers. */
export function useExecutionConfigurationCatalog(
  client: ExecutionConfigurationClient,
  identityClient?: IdentityManagementClient,
) {
  const [runtime, setRuntime] = useState<RuntimeProfileViewModel | null>(null);
  const [profiles, setProfiles] = useState<readonly CapabilityProfileOption[]>([]);
  const [profileValues, setProfileValues] = useState<ReadonlyMap<string, CapabilityProfileDto>>(
    new Map(),
  );
  const [identities, setIdentities] = useState<readonly AgentIdentityOption[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [runtimeSnapshot, capabilityProfiles, identityCatalog] = await Promise.all([
        client.loadSelectedRuntimeProfile(),
        client.listCapabilityProfiles(),
        identityClient?.list() ?? Promise.resolve([]),
      ]);
      setRuntime(runtimeProfileViewModel(runtimeSnapshot));
      setProfileValues(
        new Map(capabilityProfiles.map((profile) => [profile.capabilityProfileId, profile])),
      );
      setProfiles(
        capabilityProfiles.map((profile) => ({
          id: profile.capabilityProfileId,
          label: profile.name,
          revision: profile.revision,
        })),
      );
      setIdentities(
        identityCatalog.map((identity) => ({
          id: identity.id,
          displayName: identity.displayName,
          color: identity.color,
          shape: identity.shape,
        })),
      );
    } catch (cause) {
      setError(String(cause));
    } finally {
      setLoading(false);
    }
  }, [client, identityClient]);

  useEffect(() => {
    void reload();
  }, [reload]);

  return { runtime, profiles, profileValues, identities, error, loading, reload } as const;
}
