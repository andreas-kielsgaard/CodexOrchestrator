import { useState } from 'react';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type { NativeProfileClient } from '../../infrastructure/agentProviders/codex/profiles/nativeProfileClient';
import type { OtpCatalogueReader, OtpInstallationClient } from '../../application/otp';
import type { ExecutionTargetClient } from '../../application/executionTargets/contracts';
import type { ClaudeSetupClient } from '../../infrastructure/agentProviders/claude/claudeSetupClient';
import type { JevCredentialClient } from '../../infrastructure/agentProviders/jev/jevCredentialClient';
import type { TechnicalSettingsSection } from '../../application/productNavigation';
import type { AgentCliUpdateClient } from '../../application/agentCliUpdates';
import { availableProviderSettings } from '../agentProviders/registrations';
import { OtpConfigurationPanel } from './OtpConfigurationPanel';
import { DeviceSetupOverview, InferenceSourceOverview } from './ExecutionSetupOverview';
import { AgentCliUpdateNotice } from './AgentCliUpdateNotice';
import './technicalSettings.css';
import { DeviceSynchronizationSettings } from './DeviceSynchronizationSettings';
export function TechnicalSettingsScreen({
  nativeClient,
  claudeClient,
  jevClient,
  executionClient,
  deviceClient,
  readOtpCatalogue,
  otpInstallations,
  section: controlledSection,
  onSectionChange,
  selectedCodexProfileId,
  onSelectedCodexProfileChange,
  cliUpdateClient,
}: {
  readonly nativeClient: NativeProfileClient;
  readonly claudeClient?: ClaudeSetupClient;
  readonly jevClient?: JevCredentialClient;
  readonly executionClient?: ExecutionConfigurationClient;
  readonly deviceClient?: ExecutionTargetClient;
  readonly readOtpCatalogue?: OtpCatalogueReader;
  readonly otpInstallations?: OtpInstallationClient;
  readonly section?: TechnicalSettingsSection;
  readonly onSectionChange?: (section: TechnicalSettingsSection) => void;
  readonly selectedCodexProfileId?: string | null;
  readonly onSelectedCodexProfileChange?: (profileId: string | null) => void;
  readonly cliUpdateClient?: AgentCliUpdateClient;
}) {
  const [localSection, setLocalSection] = useState<TechnicalSettingsSection>(
    executionClient ? 'devices' : 'native',
  );
  const section = controlledSection ?? localSection;
  const providerSettings = availableProviderSettings({
    nativeClient,
    claudeClient,
    jevClient,
    selectedCodexProfileId,
    onSelectedCodexProfileChange,
  });
  const setSection = (next: TechnicalSettingsSection) => {
    setLocalSection(next);
    onSectionChange?.(next);
  };
  return (
    <div className="technical-settings">
      <header>
        <h1>Technical Settings</h1>
        <nav aria-label="Technical settings sections">
          {executionClient && (
            <button
              type="button"
              aria-pressed={section === 'devices'}
              onClick={() => setSection('devices')}
            >
              Devices
            </button>
          )}
          {executionClient && (
            <button
              type="button"
              aria-pressed={section === 'inference'}
              onClick={() => setSection('inference')}
            >
              Inference sources
            </button>
          )}
          {providerSettings.map((registration) => (
            <button
              key={registration.provider}
              type="button"
              aria-pressed={section === registration.section}
              onClick={() => setSection(registration.section)}
            >
              {registration.label}
            </button>
          ))}
          {readOtpCatalogue && (
            <button
              type="button"
              aria-pressed={section === 'otp'}
              onClick={() => setSection('otp')}
            >
              OTP packages
            </button>
          )}
        </nav>
      </header>
      <div className="technical-settings__content">
        {section === 'devices' && <DeviceSynchronizationSettings />}
        {cliUpdateClient ? <AgentCliUpdateNotice client={cliUpdateClient} /> : null}
        {section === 'devices' && executionClient ? (
          <DeviceSetupOverview
            executionClient={executionClient}
            deviceClient={deviceClient}
            providers={providerSettings
              .filter((registration) => registration.provider !== 'jev')
              .map((registration) => registration.provider)}
            onConfigureProvider={(provider) => {
              const registration = providerSettings.find((entry) => entry.provider === provider);
              if (registration) setSection(registration.section);
            }}
          />
        ) : section === 'inference' && executionClient ? (
          <InferenceSourceOverview executionClient={executionClient} />
        ) : section === 'otp' && readOtpCatalogue ? (
          <OtpConfigurationPanel
            readCatalogue={readOtpCatalogue}
            installations={otpInstallations}
          />
        ) : (
          (providerSettings.find((registration) => registration.section === section)?.content ??
          providerSettings[0]?.content)
        )}
      </div>
    </div>
  );
}
