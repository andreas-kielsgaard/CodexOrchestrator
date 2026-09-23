import { useEffect, useState } from 'react';
import { localCodexRoutes } from '../../application/executionConfiguration';
import type {
  NativeProfile,
  NativeProfileClient,
} from '../../infrastructure/nativeProfiles/nativeProfileClient';

function useLocalHarnesses(client: NativeProfileClient) {
  const [harnesses, setHarnesses] = useState<readonly NativeProfile[]>([]);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    void client.load().then(
      (query) => {
        if (active) {
          const routeIds = new Set(
            localCodexRoutes(query.profiles).map((route) => route.execution.configurationRef),
          );
          setHarnesses(query.profiles.filter((profile) => routeIds.has(profile.id)));
        }
      },
      (cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      },
    );
    return () => {
      active = false;
    };
  }, [client]);
  return { harnesses, error };
}

export function DeviceSetupOverview({
  nativeClient,
  onOpenCodexHarness,
}: {
  readonly nativeClient: NativeProfileClient;
  readonly onOpenCodexHarness: () => void;
}) {
  const { harnesses, error } = useLocalHarnesses(nativeClient);
  return (
    <section className="execution-setup-overview" aria-labelledby="device-setup-title">
      <header>
        <p>Execution setup</p>
        <h2 id="device-setup-title">Devices</h2>
        <span>
          This device · {harnesses.length} configured harness{harnesses.length === 1 ? '' : 'es'}
        </span>
      </header>
      <p>
        A device is an Orchid execution identity. Its harnesses run locally; developer access
        configuration and connection secrets are never Orchid profile settings.
      </p>
      {harnesses.map((harness) => (
        <section
          className="execution-setup-overview__card"
          aria-labelledby={`local-harness-${harness.id}`}
          key={harness.id}
        >
          <div>
            <p>Harness</p>
            <h3 id={`local-harness-${harness.id}`}>
              Codex CLI{harness.selected ? ' · selected' : ''}
            </h3>
            <span>Runs on this device</span>
          </div>
          <button type="button" onClick={onOpenCodexHarness}>
            Configure this harness
          </button>
          <p>
            {harness.homePath} · connected inference source: OpenAI via this Codex CLI
            configuration. Account credentials remain in this device-local Codex profile.
          </p>
        </section>
      ))}
      {!harnesses.length && !error ? (
        <section className="execution-setup-overview__card" aria-label="No configured harnesses">
          <div>
            <p>Harness</p>
            <h3>No Codex CLI harness configured</h3>
          </div>
          <button type="button" onClick={onOpenCodexHarness}>
            Add local Codex harness
          </button>
          <p>Add a Codex profile to create the first local harness.</p>
        </section>
      ) : null}
      {error ? (
        <p className="execution-setup-overview__error">Could not load harnesses: {error}</p>
      ) : null}
      <p className="execution-setup-overview__note">
        Enrolled remote Device Agents will appear here when Orchid Network control is available.
      </p>
    </section>
  );
}

export function InferenceSourceOverview({
  nativeClient,
}: {
  readonly nativeClient: NativeProfileClient;
}) {
  const { harnesses, error } = useLocalHarnesses(nativeClient);
  return (
    <section className="execution-setup-overview" aria-labelledby="inference-source-title">
      <header>
        <p>Execution setup</p>
        <h2 id="inference-source-title">Inference sources</h2>
        <span>{harnesses.length} connected locally</span>
      </header>
      <p>
        Sources name the account or provider path a harness may use. Orchid stores display metadata
        and bindings, never a Codex account credential or API key.
      </p>
      {harnesses.map((harness) => (
        <section
          className="execution-setup-overview__card"
          aria-labelledby={`openai-source-${harness.id}`}
          key={harness.id}
        >
          <div>
            <p>Inference source</p>
            <h3 id={`openai-source-${harness.id}`}>OpenAI via Codex CLI</h3>
            <span>Connected to Codex CLI · {harness.homePath}</span>
          </div>
          <p>
            This is the source binding for one harness configuration. A second account needs its own
            authenticated Codex profile and therefore a separate harness connection.
          </p>
        </section>
      ))}
      {!harnesses.length && !error ? (
        <p className="execution-setup-overview__note">
          No local source is available until a local Codex CLI harness is configured.
        </p>
      ) : null}
      {error ? (
        <p className="execution-setup-overview__error">Could not load sources: {error}</p>
      ) : null}
    </section>
  );
}
