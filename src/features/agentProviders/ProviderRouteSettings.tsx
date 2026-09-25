import type { ComponentType } from 'react';
import type { ProfileRoutePolicyDto } from '../../application/executionConfiguration';
import { CodexPersonalityField } from './codex/CodexPersonalityField';

export interface ProviderRouteSettingsProps {
  readonly route: ProfileRoutePolicyDto;
  readonly onChange: (route: ProfileRoutePolicyDto) => void;
}

/** Explicit provider composition. A provider without native route settings renders nothing. */
const ROUTE_SETTINGS: Readonly<Record<string, ComponentType<ProviderRouteSettingsProps>>> = {
  codex: CodexPersonalityField,
};

export function ProviderRouteSettings({ route, onChange }: ProviderRouteSettingsProps) {
  const Settings = ROUTE_SETTINGS[route.execution.provider];
  return Settings ? <Settings route={route} onChange={onChange} /> : null;
}
