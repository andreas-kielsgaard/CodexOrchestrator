import { useState } from 'react';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type { NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import type { OtpCatalogueReader, OtpInstallationClient } from '../../application/otp';
import type { ExecutionTargetClient } from '../../application/executionTargets/contracts';
import { NativeProfileSettings } from '../nativeProfiles/NativeProfileSettings';
import { OtpConfigurationPanel } from './OtpConfigurationPanel';
import { DeviceSetupOverview, InferenceSourceOverview } from './ExecutionSetupOverview';
import './technicalSettings.css';
export function TechnicalSettingsScreen({
  nativeClient,
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
  readonly executionClient?: ExecutionConfigurationClient;
  readonly deviceClient?: ExecutionTargetClient;
  readonly readOtpCatalogue?: OtpCatalogueReader;
  readonly otpInstallations?: OtpInstallationClient;
  readonly section?: 'devices' | 'inference' | 'native' | 'otp';
  readonly onSectionChange?: (section: 'devices' | 'inference' | 'native' | 'otp') => void;
  readonly selectedCodexProfileId?: string | null;
  readonly onSelectedCodexProfileChange?: (profileId: string | null) => void;
}) {
  const [localSection, setLocalSection] = useState<'devices' | 'inference' | 'native' | 'otp'>(
    executionClient ? 'devices' : 'native',
  );
  const section = controlledSection ?? localSection;
  const setSection = (next: 'devices' | 'inference' | 'native' | 'otp') => {
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
            nativeClient={nativeClient}
            deviceClient={deviceClient}
            onOpenCodexHarness={() => setSection('native')}
          />
        ) : section === 'inference' && executionClient ? (
          <InferenceSourceOverview nativeClient={nativeClient} />
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
