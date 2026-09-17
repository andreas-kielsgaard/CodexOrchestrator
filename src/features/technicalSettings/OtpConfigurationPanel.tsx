import { useCallback, useEffect, useState } from 'react';
import type {
  OtpCatalogueReader,
  OtpInstallationClient,
  OtpPackageDto,
} from '../../application/otp';

export function OtpConfigurationPanel({
  readCatalogue,
  installations,
}: {
  readonly readCatalogue?: OtpCatalogueReader;
  readonly installations?: OtpInstallationClient;
}) {
  const [packages, setPackages] = useState<readonly OtpPackageDto[]>([]);
  const [state, setState] = useState('loading');
  const [error, setError] = useState('');
  const [jobAgent, setJobAgent] = useState({
    root: '',
    python: '',
    status: 'unconfigured',
    detail: '',
  });
  const [saving, setSaving] = useState(false);
  const load = useCallback(async () => {
    setState('loading');
    try {
      if (!readCatalogue) throw new Error('The product OTP catalogue is unavailable.');
      const [nextPackages, installation] = await Promise.all([
        readCatalogue(),
        installations?.readJobAgentInstallation(),
      ]);
      setPackages(nextPackages);
      if (installation) {
        setJobAgent({
          root: installation.installation?.root ?? '',
          python: installation.installation?.python ?? '',
          status: installation.status,
          detail: installation.detail,
        });
      }
      setState('ready');
    } catch (cause) {
      setError(String(cause));
      setState('error');
    }
  }, [readCatalogue, installations]);
  useEffect(() => {
    void load();
  }, [load]);
  return (
    <div className="otp-configuration">
      <header>
        <h2>OTP configuration</h2>
        <button type="button" disabled={state === 'loading'} onClick={() => void load()}>
          Refresh packages
        </button>
      </header>
      <p>Orchestration Tool Packages imported by this product instance.</p>
      {state === 'loading' ? (
        <p role="status">Loading packages…</p>
      ) : state === 'error' ? (
        <p role="alert">{error}</p>
      ) : packages.length === 0 ? (
        <p>No OTP packages are imported.</p>
      ) : (
        packages.map((pkg) => (
          <article key={pkg.id}>
            <header>
              <h3>{pkg.id}</h3>
              <span>Imported</span>
            </header>
            {pkg.tools.length ? (
              <ul>
                {pkg.tools.map((tool) => (
                  <li key={tool.id}>{tool.name}</li>
                ))}
              </ul>
            ) : null}
            {pkg.agentMcpServers.map((server) => (
              <div key={server.serverName}>
                <h4>{server.name}</h4>
                <p>{server.description}</p>
                <p>{server.tools.length} agent MCP tools available.</p>
              </div>
            ))}
            {pkg.id === 'job_agent' && installations ? (
              <form
                className="otp-configuration__installation"
                onSubmit={(event) => {
                  event.preventDefault();
                  setSaving(true);
                  setError('');
                  void installations
                    .saveJobAgentInstallation({
                      root: jobAgent.root,
                      python: jobAgent.python,
                    })
                    .then((next) =>
                      setJobAgent({
                        root: next.installation?.root ?? jobAgent.root,
                        python: next.installation?.python ?? jobAgent.python,
                        status: next.status,
                        detail: next.detail,
                      }),
                    )
                    .catch((cause) => setError(String(cause)))
                    .finally(() => setSaving(false));
                }}
              >
                <h4>Local installation</h4>
                <p data-status={jobAgent.status}>{jobAgent.status}: {jobAgent.detail}</p>
                <label>
                  Job Agent root
                  <input
                    value={jobAgent.root}
                    onChange={(event) =>
                      setJobAgent((current) => ({ ...current, root: event.currentTarget.value }))
                    }
                    placeholder="C:\path\to\Job Agent"
                  />
                </label>
                <label>
                  Python command
                  <input
                    value={jobAgent.python}
                    onChange={(event) =>
                      setJobAgent((current) => ({ ...current, python: event.currentTarget.value }))
                    }
                    placeholder="python"
                  />
                </label>
                <button type="submit" disabled={saving}>
                  {saving ? 'Verifying…' : 'Save and verify'}
                </button>
              </form>
            ) : null}
          </article>
        ))
      )}
    </div>
  );
}
