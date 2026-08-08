import type { NativeProfileClient, NativeProfileQuery, NativeProfileReadiness } from './nativeProfileClient';

/** The only profile state admitted to an application runtime consumer. */
export interface NativeProfileApplicationResolution {
  readonly profileId: string;
  readonly codexHome: string;
  readonly readiness: NativeProfileReadiness;
}

export interface NativeProfileApplicationConsumer {
  resolve(selectedProfileId: string): Promise<NativeProfileApplicationResolution>;
}

/** Resolve a caller-owned selected id against one current, strictly decoded query. */
export function resolveNativeProfileApplicationConsumer(
  query: NativeProfileQuery,
  selectedProfileId: string,
): NativeProfileApplicationResolution {
  if (selectedProfileId.length === 0 || selectedProfileId.trim() !== selectedProfileId)
    throw new Error('Selected native profile id must be a non-empty trimmed string');
  const selected = query.profiles.filter((profile) => profile.id === selectedProfileId);
  if (selected.length !== 1) throw new Error('Selected native profile id is not present exactly once');
  const profile = selected[0];
  if (!profile.selected || profile.lifecycle !== 'active')
    throw new Error('Selected native profile is not currently validated');
  return { profileId: profile.id, codexHome: profile.homePath, readiness: profile.readiness };
}

export function createNativeProfileApplicationConsumer(
  client: Pick<NativeProfileClient, 'load'>,
): NativeProfileApplicationConsumer {
  return { resolve: async (selectedProfileId) => resolveNativeProfileApplicationConsumer(await client.load(), selectedProfileId) };
}
