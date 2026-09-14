import { useState } from 'react';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type { ExecutionTargetClient } from '../../application/executionTargets/contracts';
import type { RepositoryBranchSource } from '../../application/branches';
import type { NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';
import type { DraftWorkspace } from '../../components/draftWorkspace';
import type { CapabilityProfileDraft } from '../executionConfiguration/types';
import { ExecutionConfigurationScreen } from '../executionConfiguration';
import { NativeProfileSettings } from '../nativeProfiles/NativeProfileSettings';
import './technicalSettings.css';
export function TechnicalSettingsScreen({
  nativeClient,
  executionClient,
  targetClient,
  branchSource,
  workspace,
}: {
  readonly nativeClient: NativeProfileClient;
  readonly executionClient?: ExecutionConfigurationClient;
  readonly targetClient?: ExecutionTargetClient;
  readonly branchSource?: RepositoryBranchSource;
  readonly workspace?: DraftWorkspace<CapabilityProfileDraft>;
}) {
  const [section, setSection] = useState<'connections' | 'native'>(
    executionClient ? 'connections' : 'native',
  );
  return (
    <div className="technical-settings">
      <header>
        <h1>Technical Settings</h1>
        <nav aria-label="Technical settings sections">
          {executionClient && (
            <button
              type="button"
              aria-pressed={section === 'connections'}
              onClick={() => setSection('connections')}
            >
              Devices and Capability Profiles
            </button>
          )}
          <button
            type="button"
            aria-pressed={section === 'native'}
            onClick={() => setSection('native')}
          >
            Local Codex homes
          </button>
        </nav>
      </header>
      <div className="technical-settings__content">
        {section === 'connections' && executionClient ? (
          <ExecutionConfigurationScreen
            client={executionClient}
            targetClient={targetClient}
            branchSource={branchSource}
            workspace={workspace}
          />
        ) : (
          <NativeProfileSettings client={nativeClient} />
        )}
      </div>
    </div>
  );
}
