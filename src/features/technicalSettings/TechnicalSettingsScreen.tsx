import { useState } from 'react';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type { NativeProfileClient } from '../../infrastructure/agentProviders/codex/profiles/nativeProfileClient';
import type { OtpCatalogueReader, OtpInstallationClient } from '../../application/otp';
import type { ExecutionTargetClient } from '../../application/executionTargets/contracts';
import type { ClaudeSetupClient } from '../../infrastructure/agentProviders/claude/claudeSetupClient';
import type { TechnicalSettingsSection } from '../../application/productNavigation';
import { ClaudeSetupSettings } from '../agentProviders/claude/ClaudeSetupSettings';
import { NativeProfileSettings } from '../agentProviders/codex/profiles/NativeProfileSettings';
import { OtpConfigurationPanel } from './OtpConfigurationPanel';
import { DeviceSetupOverview, InferenceSourceOverview } from './ExecutionSetupOverview';
import './technicalSettings.css';
export function TechnicalSettingsScreen({
  nativeClient,
  claudeClient,
  executionClient,
  deviceClient,
  readOtpCatalogue,
  otpInstallations,
  section: controlledSection,
  onSectionChange,
  selectedCodexProfileId,
  onSelectedCodexProfileChange,
}: {
  readonly nativeClient: NativeProfileClient;
  readonly claudeClient?: ClaudeSetupClient;
  readonly executionClient?: ExecutionConfigurationClient;
  readonly deviceClient?: ExecutionTargetClient;
  readonly readOtpCatalogue?: OtpCatalogueReader;
  readonly otpInstallations?: OtpInstallationClient;
  readonly section?: TechnicalSettingsSection;
  readonly onSectionChange?: (section: TechnicalSettingsSection) => void;
  readonly selectedCodexProfileId?: string | null;
  readonly onSelectedCodexProfileChange?: (profileId: string | null) => void;
}) {
  const [localSection, setLocalSection] = useState<TechnicalSettingsSection>(
    executionClient ? 'devices' : 'native',
  );
  const section = controlledSection ?? localSection;
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
          <button
            type="button"
            aria-pressed={section === 'native'}
            onClick={() => setSection('native')}
          >
            Codex profiles
          </button>
          {claudeClient && (
            <button
              type="button"
              aria-pressed={section === 'claude'}
              onClick={() => setSection('claude')}
            >
              Claude setups
            </button>
          )}
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
        {section === 'devices' && executionClient ? (
          <DeviceSetupOverview
            executionClient={executionClient}
            deviceClient={deviceClient}
            providers={claudeClient ? ['codex', 'claude'] : ['codex']}
            onConfigureProvider={(provider) =>
              setSection(provider === 'claude' ? 'claude' : 'native')
            }
          />
        ) : section === 'inference' && executionClient ? (
          <InferenceSourceOverview executionClient={executionClient} />
        ) : section === 'claude' && claudeClient ? (
          <ClaudeSetupSettings client={claudeClient} />
        ) : section === 'otp' && readOtpCatalogue ? (
          <OtpConfigurationPanel
            readCatalogue={readOtpCatalogue}
            installations={otpInstallations}
          />
        ) : (
          <NativeProfileSettings
            client={nativeClient}
            selectedProfileId={selectedCodexProfileId}
            onSelectedProfileChange={onSelectedCodexProfileChange}
          />
        )}
      </div>
    </div>
  );
}
