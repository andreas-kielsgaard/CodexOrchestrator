import type { WorktreeReviewClient } from '../../application/worktreeReview';
import type { NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import { WorktreeReviewSettings } from '../humanReviewLauncher/WorktreeReviewSettings';
import { NativeProfileSettings } from '../nativeProfiles/NativeProfileSettings';
import './technicalSettings.css';

export function TechnicalSettingsView({
  nativeProfileClient,
  worktreeReviewClient,
}: {
  readonly nativeProfileClient?: NativeProfileClient;
  readonly worktreeReviewClient?: WorktreeReviewClient;
}) {
  return (
    <main className="technical-settings" aria-label="Technical settings">
      <header>
        <p className="eyebrow">Technical Settings</p>
        <h1>Application infrastructure</h1>
        <p>Manage product-owned runtime, cleanup, and Codex home configuration.</p>
      </header>
      {worktreeReviewClient && <WorktreeReviewSettings client={worktreeReviewClient} />}
      {nativeProfileClient && <NativeProfileSettings client={nativeProfileClient} />}
    </main>
  );
}
