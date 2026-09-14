export function formatSessionDeepLink(sessionId: string): string {
  return `codex-orchestrator://sessions/${encodeURIComponent(sessionId)}`;
}
export function parseSessionDeepLink(value: string): string | null {
  try {
    const url = new URL(value);
    if (
      url.protocol !== 'codex-orchestrator:' ||
      url.hostname !== 'sessions' ||
      url.search ||
      url.hash ||
      url.username ||
      url.password ||
      url.port
    )
      return null;
    const id = decodeURIComponent(url.pathname.slice(1));
    return id && !id.includes('/') && !/[\\\s]/.test(id) ? id : null;
  } catch {
    return null;
  }
}
export interface SessionDeepLinkSource {
  subscribe(listener: (sessionId: string) => void): Promise<() => void>;
}
