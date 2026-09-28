import type { ComponentType, ReactNode } from 'react';
import type { TechnicalSettingsSection } from '../../application/productNavigation';
import type { ClaudeSetupClient } from '../../infrastructure/agentProviders/claude/claudeSetupClient';
import type { NativeProfileClient } from '../../infrastructure/agentProviders/codex/profiles/nativeProfileClient';
import { ClaudeSetupSettings } from './claude/ClaudeSetupSettings';
import { CodexPersonalityField } from './codex/CodexPersonalityField';
import { NativeProfileSettings } from './codex/profiles/NativeProfileSettings';
import type { ProviderRouteSettingsProps } from './ProviderRouteSettings';

type ProviderSettingsSection = Extract<TechnicalSettingsSection, 'native' | 'claude'>;

interface ProviderUiRegistration {
  readonly provider: string;
  readonly settingsSection: ProviderSettingsSection;
  readonly settingsLabel: string;
  readonly routeSettings?: ComponentType<ProviderRouteSettingsProps>;
}

/** One explicit feature-layer composition point for provider-owned UI. */
const PROVIDERS: readonly ProviderUiRegistration[] = [
  {
    provider: 'codex',
    settingsSection: 'native',
    settingsLabel: 'Codex profiles',
    routeSettings: CodexPersonalityField,
  },
  {
    provider: 'claude',
    settingsSection: 'claude',
    settingsLabel: 'Claude setups',
  },
];

export function providerUiRegistration(provider: string): ProviderUiRegistration | undefined {
  return PROVIDERS.find((registration) => registration.provider === provider);
}

export interface AvailableProviderSettings {
  readonly provider: string;
  readonly section: ProviderSettingsSection;
  readonly label: string;
  readonly content: ReactNode;
}

export function availableProviderSettings(input: {
  readonly nativeClient: NativeProfileClient;
  readonly claudeClient?: ClaudeSetupClient;
  readonly selectedCodexProfileId?: string | null;
  readonly onSelectedCodexProfileChange?: (profileId: string | null) => void;
}): readonly AvailableProviderSettings[] {
  return PROVIDERS.flatMap((registration) => {
    if (registration.provider === 'codex') {
      return [
        {
          provider: registration.provider,
          section: registration.settingsSection,
          label: registration.settingsLabel,
          content: (
            <NativeProfileSettings
              client={input.nativeClient}
              selectedProfileId={input.selectedCodexProfileId}
              onSelectedProfileChange={input.onSelectedCodexProfileChange}
            />
          ),
        },
      ];
    }
    if (registration.provider === 'claude' && input.claudeClient) {
      return [
        {
          provider: registration.provider,
          section: registration.settingsSection,
          label: registration.settingsLabel,
          content: <ClaudeSetupSettings client={input.claudeClient} />,
        },
      ];
    }
    return [];
  });
}
