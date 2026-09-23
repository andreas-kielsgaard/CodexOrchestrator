import { useState } from 'react';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type { NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import type { OtpCatalogueReader, OtpInstallationClient } from '../../application/otp';
import { NativeProfileSettings } from '../nativeProfiles/NativeProfileSettings';
import { OtpConfigurationPanel } from './OtpConfigurationPanel';
import { DeviceSetupOverview, InferenceSourceOverview } from './ExecutionSetupOverview';
import './technicalSettings.css';
export function TechnicalSettingsScreen({
  nativeClient,
  executionClient,
  readOtpCatalogue,
  otpInstallations,
}: {
  readonly nativeClient: NativeProfileClient;
  readonly executionClient?: ExecutionConfigurationClient;
  readonly readOtpCatalogue?: OtpCatalogueReader;
  readonly otpInstallations?: OtpInstallationClient;
}) {
  const [section, setSection] = useState<'devices' | 'inference' | 'native' | 'otp'>(
    executionClient ? 'devices' : 'native',
  );
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
          <NativeProfileSettings client={nativeClient} />
        )}
      </div>
    </div>
  );
}
