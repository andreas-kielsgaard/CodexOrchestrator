import { useEffect, useState } from 'react';
import {
  agentProviderDescriptor,
  displayFolderPath,
  type ProviderSetupDto,
} from '../../application/agentProviders';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
/** Every provider's setups; each provider owns its own setup screen. */
function useProviderSetups(client: ExecutionConfigurationClient | undefined) {
  const [setups, setSetups] = useState<readonly ProviderSetupDto[]>([]);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    if (!client?.listProviderSetups) return;
    void client.listProviderSetups().then(
      (next) => {
        if (active) setSetups(next);
      },
      (cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      },
    );
    return () => {
      active = false;
    };
  }, [client]);
  return { setups, error };
}

const SETUP_STATE_LABELS: Record<ProviderSetupDto['state'], string> = {
  ready: 'Ready',
  needs_login: 'Needs sign-in',
  unavailable: 'Unavailable',
};

export function HarnessSetupOverview({
  executionClient,
  providers,
  onConfigureProvider,
}: {
  readonly executionClient?: ExecutionConfigurationClient;
  /** Providers with a setup screen in Technical Settings. */
  readonly providers: readonly string[];
  readonly onConfigureProvider: (provider: string) => void;
}) {
  const { setups, error } = useProviderSetups(executionClient);
  return (
    <section className="execution-setup-overview" aria-labelledby="device-setup-title">
      <header>
        <p>Execution setup</p>
        <h2 id="device-setup-title">Agent setups</h2>
        <span>{setups.length} configured harnesses</span>
      </header>
      <p>Configure the provider harnesses that Orchid uses to run agent work.</p>
      {setups.map((setup) => {
        const descriptor = agentProviderDescriptor(setup.provider);
        const key = `${setup.deviceId}-${setup.provider}-${setup.configurationId}`;
        return (
          <section
            className="execution-setup-overview__card"
            aria-labelledby={`harness-${key}`}
            key={key}
          >
            <div>
              <p>Harness</p>
              <h3 id={`harness-${key}`}>
                {descriptor.harnessLabel}
                {setup.selected ? ' · selected' : ''}
              </h3>
              <span>
                {setup.deviceId === 'local' ? 'Runs on this device' : `Runs on ${setup.deviceId}`} ·{' '}
                {SETUP_STATE_LABELS[setup.state]}
              </span>
            </div>
            {providers.includes(setup.provider) ? (
              <button type="button" onClick={() => onConfigureProvider(setup.provider)}>
                Configure this harness
              </button>
            ) : null}
            <p>
              {displayFolderPath(setup.folder)}
              {setup.executable ? ` · ${setup.executable}` : ''} · connected inference source:{' '}
              {descriptor.inferenceLabel}. Account credentials remain in this device-local{' '}
              {descriptor.configurationLabel}.{setup.detail ? ` ${setup.detail}` : ''}
            </p>
          </section>
        );
      })}
      {!setups.length && !error ? (
        <section className="execution-setup-overview__card" aria-label="No configured harnesses">
          <div>
            <p>Harness</p>
            <h3>No harness configured</h3>
          </div>
          {providers.map((provider) => (
            <button key={provider} type="button" onClick={() => onConfigureProvider(provider)}>
              Add local {agentProviderDescriptor(provider).harnessLabel} harness
            </button>
          ))}
          <p>Add a provider setup to create the first local harness.</p>
        </section>
      ) : null}
      {error ? (
        <p className="execution-setup-overview__error">Could not load harnesses: {error}</p>
      ) : null}
    </section>
  );
}

export function InferenceSourceOverview({
  executionClient,
}: {
  readonly executionClient?: ExecutionConfigurationClient;
}) {
  const { setups, error } = useProviderSetups(executionClient);
  return (
    <section className="execution-setup-overview" aria-labelledby="inference-source-title">
      <header>
        <p>Execution setup</p>
        <h2 id="inference-source-title">Inference sources</h2>
        <span>{setups.length} connected locally</span>
      </header>
      <p>
        Sources name the account or provider path a harness may use. Orchid stores display metadata
        and bindings, never a provider account credential or API key.
      </p>
      {setups.map((setup) => {
        const descriptor = agentProviderDescriptor(setup.provider);
        const key = `${setup.deviceId}-${setup.provider}-${setup.configurationId}`;
        return (
          <section
            className="execution-setup-overview__card"
            aria-labelledby={`source-${key}`}
            key={key}
          >
            <div>
              <p>Inference source</p>
              <h3 id={`source-${key}`}>{descriptor.inferenceLabel}</h3>
              <span>
                Connected to {descriptor.harnessLabel} · {displayFolderPath(setup.folder)}
              </span>
            </div>
            <p>
              This is the source binding for one harness configuration. A second account needs its
              own authenticated {descriptor.configurationLabel} and therefore a separate harness
              connection.
            </p>
          </section>
        );
      })}
      {!setups.length && !error ? (
        <p className="execution-setup-overview__note">
          No local source is available until a local harness is configured.
        </p>
      ) : null}
      {error ? (
        <p className="execution-setup-overview__error">Could not load sources: {error}</p>
      ) : null}
    </section>
  );
}
