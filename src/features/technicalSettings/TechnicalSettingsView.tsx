import type { NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import { NativeProfileSettings } from '../nativeProfiles/NativeProfileSettings';
import './technicalSettings.css';

export function TechnicalSettingsView({
  nativeProfileClient,
}: {
  readonly nativeProfileClient?: NativeProfileClient;
}) {
  return (
    <main className="technical-settings" aria-label="Technical settings">
      <header>
        <p className="eyebrow">Technical Settings</p>
        <h1>Application infrastructure</h1>
        <p>Manage product-owned Codex home configuration.</p>
      </header>
      {nativeProfileClient && <NativeProfileSettings client={nativeProfileClient} />}
    </main>
  );
}
