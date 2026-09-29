import type { ProfileRoutePolicyDto } from '../../application/executionConfiguration';
import { providerUiRegistration } from './registrations';

export interface ProviderRouteSettingsProps {
  readonly route: ProfileRoutePolicyDto;
  readonly onChange: (route: ProfileRoutePolicyDto) => void;
}

export function ProviderRouteSettings({ route, onChange }: ProviderRouteSettingsProps) {
  const Settings = providerUiRegistration(route.execution.provider)?.routeSettings;
  return Settings ? <Settings route={route} onChange={onChange} /> : null;
}
