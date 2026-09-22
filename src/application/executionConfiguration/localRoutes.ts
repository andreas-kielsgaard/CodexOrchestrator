import type { ExecutionBindingDto } from '../executionTargets/contracts';

export interface RegisteredLocalCodexProfile {
  readonly id: string;
  readonly homePath: string;
  readonly lifecycle: string;
  readonly selected: boolean;
}

/** A configured route exists independently of whether Codex currently starts or signs in. */
export interface HarnessInferenceRouteOption {
  readonly id: string;
  readonly selected: boolean;
  readonly label: string;
  readonly sourceLabel: string;
  readonly deviceLabel?: string;
  readonly harnessLabel?: string;
  readonly inferenceLabel?: string;
  readonly detail: string;
  readonly execution: ExecutionBindingDto;
}

export function localCodexRoutes(
  profiles: readonly RegisteredLocalCodexProfile[],
): readonly HarnessInferenceRouteOption[] {
  return profiles
    .filter((profile) => profile.lifecycle === 'active')
    .map((profile) => ({
      id: `local-codex:${profile.id}`,
      selected: profile.selected,
      label: profile.selected ? 'This device · selected Codex CLI' : 'This device · Codex CLI',
      sourceLabel: 'OpenAI via Codex CLI',
      deviceLabel: 'This device',
      harnessLabel: 'Codex CLI',
      inferenceLabel: 'OpenAI account via Codex CLI',
      detail: `${profile.homePath} · account configuration stays in this Codex profile`,
      execution: {
        deviceId: 'local',
        deviceName: 'This device',
        provider: 'codex' as const,
        configurationRef: profile.id,
        connection: { kind: 'local' as const },
      },
    }));
}
