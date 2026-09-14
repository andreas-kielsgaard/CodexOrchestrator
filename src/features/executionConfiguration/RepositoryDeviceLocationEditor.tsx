import { useEffect, useState } from 'react';
import type { RepositoryBranchSource } from '../../application/branches';
import type { RegisteredRepository } from '../../application/repositoryCatalog';
import type {
  ExecutionBindingDto,
  ExecutionTargetClient,
  RepositoryDeviceLocationDto,
} from '../../application/executionTargets/contracts';
export function RepositoryDeviceLocationEditor({
  execution,
  client,
  source,
}: {
  readonly execution: ExecutionBindingDto;
  readonly client: ExecutionTargetClient;
  readonly source: RepositoryBranchSource;
}) {
  const [repositories, setRepositories] = useState<readonly RegisteredRepository[]>([]);
  const [locations, setLocations] = useState<readonly RepositoryDeviceLocationDto[]>([]);
  const [repositoryId, setRepositoryId] = useState('');
  const [path, setPath] = useState('');
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    void Promise.all([source.listRepositories(), client.listRepositoryLocations()]).then(
      ([repos, mappings]) => {
        if (current) {
          setRepositories(repos);
          setLocations(mappings);
        }
      },
      (cause) => {
        if (current) setMessage(String(cause));
      },
    );
    return () => {
      current = false;
    };
  }, [source, client]);
  useEffect(() => {
    setPath(
      locations.find(
        (item) => item.repositoryId === repositoryId && item.deviceId === execution.deviceId,
      )?.repositoryRoot ?? '',
    );
  }, [repositoryId, execution.deviceId, locations]);
  return (
    <section className="repository-device-location" aria-label="Repository location on device">
      <h3>Repository on this device</h3>
      <p>
        Connect a registered project to its existing clone on{' '}
        {execution.deviceName || 'this device'}.
      </p>
      <label className="execution-configuration__field">
        <span>Repository / project</span>
        <select
          aria-label="Mapped repository"
          value={repositoryId}
          onChange={(event) => setRepositoryId(event.target.value)}
        >
          <option value="">Choose a repository…</option>
          {repositories.map((repo) => (
            <option key={repo.repositoryId} value={repo.repositoryId}>
              {repo.name} — {repo.locationLabel}
            </option>
          ))}
        </select>
      </label>
      {repositoryId &&
        (execution.connection.kind === 'local' ? (
          <p>
            Local location:{' '}
            <code>
              {repositories.find((repo) => repo.repositoryId === repositoryId)?.locationLabel}
            </code>
          </p>
        ) : (
          <>
            <label className="execution-configuration__field">
              <span>Existing clone path on device</span>
              <input
                aria-label="Repository path on device"
                value={path}
                placeholder="/root/projects/repository"
                onChange={(event) => setPath(event.target.value)}
              />
            </label>
            <button
              type="button"
              disabled={saving || !path.trim() || !execution.deviceId.trim()}
              onClick={() => {
                setSaving(true);
                setMessage(null);
                const input = {
                  repositoryId,
                  deviceId: execution.deviceId,
                  repositoryRoot: path.trim(),
                };
                void client
                  .saveRepositoryLocation(input)
                  .then(
                    () => {
                      setLocations((items) => [
                        ...items.filter(
                          (item) =>
                            item.repositoryId !== repositoryId ||
                            item.deviceId !== execution.deviceId,
                        ),
                        input,
                      ]);
                      setMessage('Repository location saved.');
                    },
                    (cause) => setMessage(String(cause)),
                  )
                  .finally(() => setSaving(false));
              }}
            >
              {saving ? 'Saving location…' : 'Save repository location'}
            </button>
          </>
        ))}
      {message && <p role="status">{message}</p>}
    </section>
  );
}
