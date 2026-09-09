import { useCallback, useEffect, useState } from 'react';
import type { OtpCatalogueReader, OtpPackageDto } from '../../application/otp';

export function OtpConfigurationPanel({
  readCatalogue,
}: {
  readonly readCatalogue?: OtpCatalogueReader;
}) {
  const [packages, setPackages] = useState<readonly OtpPackageDto[]>([]);
  const [state, setState] = useState('loading');
  const [error, setError] = useState('');
  const load = useCallback(async () => {
    setState('loading');
    try {
      if (!readCatalogue) throw new Error('The product OTP catalogue is unavailable.');
      setPackages(await readCatalogue());
      setState('ready');
    } catch (cause) {
      setError(String(cause));
      setState('error');
    }
  }, [readCatalogue]);
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
            <ul>
              {pkg.tools.map((tool) => (
                <li key={tool.id}>{tool.name}</li>
              ))}
            </ul>
          </article>
        ))
      )}
    </div>
  );
}
