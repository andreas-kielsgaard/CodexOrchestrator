import { useState } from 'react';
import type {
  ExecutionConfigurationClient,
  NativeCapabilityInventoryDto,
} from '../../application/executionConfiguration';

export function NativeCapabilityInventory({
  client,
  loadInventory,
}: {
  client: ExecutionConfigurationClient;
  loadInventory?: () => Promise<NativeCapabilityInventoryDto>;
}) {
  const [inventory, setInventory] = useState<NativeCapabilityInventoryDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (!loadInventory && !client.loadNativeCapabilityInventory) return null;
  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      setInventory(await (loadInventory ?? client.loadNativeCapabilityInventory!)());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  };
  return (
    <details className="native-capability-inventory">
      <summary>Capabilities inherited from Codex</summary>
      <p>
        Native skills, hooks, plugins and MCP tools remain available according to Codex
        configuration. Installed plugins may require services that this runtime does not supply.
      </p>
      <button type="button" disabled={loading} onClick={() => void load()}>
        {loading ? 'Reading Codex…' : 'Read native inventory'}
      </button>
      {error && <p role="alert">{error}</p>}
      {inventory && (
        <>
          {inventory.limitations.map((limitation) => (
            <p key={limitation}>{limitation}</p>
          ))}
          <ul>
            {inventory.entries.map((entry, index) => (
              <li key={`${entry.kind}:${entry.name}:${index}`}>
                <strong>{entry.name}</strong> — {entry.kind.replaceAll('_', ' ')} · {entry.origin} ·{' '}
                {entry.state}. {entry.support}
              </li>
            ))}
          </ul>
          {inventory.entries.length === 0 && <p>No native entries were observed.</p>}
        </>
      )}
    </details>
  );
}
