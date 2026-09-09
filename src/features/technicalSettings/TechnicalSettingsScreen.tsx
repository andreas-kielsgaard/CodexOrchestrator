import { useState } from 'react';
import type { OtpCatalogueReader } from '../../application/otp';
import type { NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import { NativeProfileSettings } from '../nativeProfiles/NativeProfileSettings';
import { OtpConfigurationPanel } from './OtpConfigurationPanel';
import './technicalSettings.css';

export function TechnicalSettingsScreen({
  client,
  readOtpCatalogue,
}: {
  readonly client: NativeProfileClient;
  readonly readOtpCatalogue?: OtpCatalogueReader;
}) {
  const [tab, setTab] = useState('homes');
  return (
    <main className="technical-settings">
      <h1>Technical Settings</h1>
      <div role="tablist" aria-label="Technical Settings">
        {[
          ['homes', 'Codex home profiles'],
          ['otp', 'OTP configuration'],
        ].map(([id, label]) => (
          <button
            key={id}
            type="button"
            id={`settings-tab-${id}`}
            role="tab"
            aria-selected={tab === id}
            aria-controls={`settings-panel-${id}`}
            tabIndex={tab === id ? 0 : -1}
            onClick={() => setTab(id)}
            onKeyDown={(event) => {
              if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) {
                event.preventDefault();
                const next =
                  event.key === 'Home'
                    ? 'homes'
                    : event.key === 'End'
                      ? 'otp'
                      : tab === 'homes'
                        ? 'otp'
                        : 'homes';
                setTab(next);
                document.getElementById(`settings-tab-${next}`)?.focus();
              }
            }}
          >
            {label}
          </button>
        ))}
      </div>
      <section
        role="tabpanel"
        id="settings-panel-homes"
        aria-labelledby="settings-tab-homes"
        hidden={tab !== 'homes'}
      >
        <NativeProfileSettings client={client} embedded />
      </section>
      <section
        role="tabpanel"
        id="settings-panel-otp"
        aria-labelledby="settings-tab-otp"
        hidden={tab !== 'otp'}
      >
        <OtpConfigurationPanel readCatalogue={readOtpCatalogue} />
      </section>
    </main>
  );
}
