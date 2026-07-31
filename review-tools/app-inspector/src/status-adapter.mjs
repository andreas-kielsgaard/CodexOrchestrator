export async function inspectStatusEndpoint(baseUrl) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 1200);
  try {
    const root = new URL(baseUrl);
    const [healthResponse, statusResponse] = await Promise.all([
      fetch(new URL('/health', root), { signal: controller.signal }),
      fetch(new URL('/status', root), { signal: controller.signal }),
    ]);
    if (!healthResponse.ok || !statusResponse.ok) {
      return unavailable(
        `The endpoint responded, but health/status returned HTTP ${healthResponse.status}/${statusResponse.status}.`,
      );
    }
    const health = await healthResponse.json();
    const status = await statusResponse.json();
    const recognized = health?.ok === true && status?.statusVersion === 1;
    return {
      disposition: 'observed',
      source: 'loopback HTTP development status endpoint',
      value: {
        baseUrl: root.origin,
        recognizedContract: recognized,
        health,
        status,
        applicationRelationship: {
          disposition: 'unavailable',
          reason:
            'Status contract v1 carries no PID, executable, worktree, or instance identity, so reachability does not prove ownership by the selected app.',
        },
      },
    };
  } catch (error) {
    return unavailable(
      `No development status endpoint was observed at ${baseUrl}: ${error instanceof Error ? error.message : String(error)}`,
    );
  } finally {
    clearTimeout(timeout);
  }
}

function unavailable(reason) {
  return { disposition: 'unavailable', reason };
}
