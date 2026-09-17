import { useCallback, useEffect, useState } from 'react';
import type { OtpCatalogueReader, OtpInstallationClient, OtpPackageDto } from '../../application/otp';
import { OtpPackageDetailPage } from '../otpCatalogue/OtpPackageDetailPage';
import { OtpPackageDirectory } from '../otpCatalogue/OtpPackageDirectory';

type JobAgentInstallationState = {
  readonly root: string;
  readonly python: string;
  readonly status: string;
  readonly detail: string;
};

export function OtpConfigurationPanel({ readCatalogue, installations }: { readonly readCatalogue?: OtpCatalogueReader; readonly installations?: OtpInstallationClient }) {
  const [packages, setPackages] = useState<readonly OtpPackageDto[]>([]);
  const [state, setState] = useState('loading');
  const [error, setError] = useState('');
  const [selectedPackageId, setSelectedPackageId] = useState<string | null>(null);
  const [jobAgent, setJobAgent] = useState<JobAgentInstallationState>({ root: '', python: '', status: 'unconfigured', detail: '' });
  const [saving, setSaving] = useState(false);
  const load = useCallback(async () => {
    setState('loading');
    try {
      if (!readCatalogue) throw new Error('The product OTP catalogue is unavailable.');
      const [nextPackages, installation] = await Promise.all([readCatalogue(), installations?.readJobAgentInstallation()]);
      setPackages(nextPackages);
      if (installation) setJobAgent({ root: installation.installation?.root ?? '', python: installation.installation?.python ?? '', status: installation.status, detail: installation.detail });
      setState('ready');
    } catch (cause) {
      setError(String(cause));
      setState('error');
    }
  }, [readCatalogue, installations]);
  useEffect(() => { void load(); }, [load]);
  const selected = packages.find((pkg) => pkg.id === selectedPackageId) ?? null;
  const saveJobAgent = () => {
    if (!installations || saving) return;
    setSaving(true);
    setError('');
    void installations.saveJobAgentInstallation({ root: jobAgent.root, python: jobAgent.python })
      .then((next) => setJobAgent({ root: next.installation?.root ?? jobAgent.root, python: next.installation?.python ?? jobAgent.python, status: next.status, detail: next.detail }))
      .catch((cause) => setError(String(cause)))
      .finally(() => setSaving(false));
  };
  return (
    <div className="otp-configuration">
      {selected ? (
        <OtpPackageDetailPage
          pkg={selected}
          onBack={() => setSelectedPackageId(null)}
          packageConfiguration={selected.id === 'job_agent' && installations ? (
            <JobAgentInstallationForm state={jobAgent} saving={saving} error={error} onChange={setJobAgent} onSave={saveJobAgent} />
          ) : undefined}
        />
      ) : (
        <>
          <header>
            <div><h2>OTP configuration</h2><p>Orchestration Tool Packages imported by this product instance.</p></div>
            <button type="button" disabled={state === 'loading'} onClick={() => void load()}>Refresh packages</button>
          </header>
          {state === 'loading' ? <p role="status">Loading packages…</p> : null}
          {state === 'error' ? <p role="alert">{error}</p> : null}
          {state === 'ready' && packages.length === 0 ? <p>No OTP packages are imported.</p> : null}
          {state === 'ready' && packages.length ? <OtpPackageDirectory packages={packages} onOpen={setSelectedPackageId} /> : null}
        </>
      )}
    </div>
  );
}

function JobAgentInstallationForm({ state, saving, error, onChange, onSave }: { readonly state: JobAgentInstallationState; readonly saving: boolean; readonly error: string; readonly onChange: (next: JobAgentInstallationState) => void; readonly onSave: () => void }) {
  return (
    <form className="otp-configuration__installation" onSubmit={(event) => { event.preventDefault(); onSave(); }}>
      <header><h3>Local installation</h3><p data-status={state.status}>{state.status}: {state.detail}</p></header>
      <label>Job Agent root<input value={state.root} onChange={(event) => onChange({ ...state, root: event.currentTarget.value })} placeholder="C:\path\to\Job Agent" /></label>
      <label>Python command<input value={state.python} onChange={(event) => onChange({ ...state, python: event.currentTarget.value })} placeholder="python" /></label>
      {error ? <p role="alert">{error}</p> : null}
      <button type="submit" disabled={saving}>{saving ? 'Verifying…' : 'Save and verify'}</button>
    </form>
  );
}
