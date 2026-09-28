import { useCallback, useEffect, useState } from 'react';
import type {
  CapabilityProfileDto,
  ExecutionConfigurationClient,
  ProfileModelCatalogueDto,
} from '../../application/executionConfiguration';
import { executionRouteKey, executionRouteRef } from '../../application/executionTargets/contracts';
import type { IdentityManagementClient } from '../../application/identities';
import type { OtpCatalogueReader } from '../../application/otp';
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
  readOtpCatalogue?: OtpCatalogueReader,
) {
  const [runtime, setRuntime] = useState<RuntimeProfileViewModel | null>(null);
  const [profiles, setProfiles] = useState<readonly CapabilityProfileOption[]>([]);
  const [profileValues, setProfileValues] = useState<ReadonlyMap<string, CapabilityProfileDto>>(
    new Map(),
  );
  const [modelCatalogues, setModelCatalogues] = useState<
    Readonly<Record<string, ProfileModelCatalogueDto>>
  >({});
  const [identities, setIdentities] = useState<readonly AgentIdentityOption[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [runtimeSnapshot, capabilityProfiles, identityCatalog, otpPackages] = await Promise.all(
        [
          client.loadSelectedRuntimeProfile().catch(() => null),
          client.listCapabilityProfiles(),
          identityClient?.list() ?? Promise.resolve([]),
          // OTP availability is resolved when an agent uses a tool. The
          // designer can still edit profiles and recipes without catalogue data.
          readOtpCatalogue?.().catch(() => []) ?? Promise.resolve([]),
        ],
      );
      if (runtimeSnapshot) {
        const runtime = runtimeProfileViewModel(runtimeSnapshot);
        setRuntime({ ...runtime, catalogs: { ...runtime.catalogs, otpPackages } });
      } else {
        setRuntime(null);
      }
      setProfileValues(
        new Map(capabilityProfiles.map((profile) => [profile.capabilityProfileId, profile])),
      );
      const loadModels = client.loadProfileModelCatalogue;
      if (loadModels) {
        const routes = new Map(
          capabilityProfiles
            .flatMap((profile) => profile.routePolicies ?? [])
            .map((route) => {
              const reference = executionRouteRef(route.execution);
              return [executionRouteKey(reference), reference] as const;
            }),
        );
        const entries = await Promise.all(
          [...routes].map(async ([key, route]) => {
            try {
              return [key, await loadModels(route)] as const;
            } catch {
              return null;
            }
          }),
        );
        setModelCatalogues(Object.fromEntries(entries.filter((entry) => entry !== null)));
      } else {
        setModelCatalogues({});
      }
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
  }, [client, identityClient, readOtpCatalogue]);

  useEffect(() => {
    void reload();
  }, [reload]);

  return {
    runtime,
    profiles,
    profileValues,
    modelCatalogues,
    identities,
    error,
    loading,
    reload,
  } as const;
}
