import { tauriSessionDeepLinks } from './tauriSessionDeepLinks';
const bridge = vi.hoisted(() => ({
  getCurrent: vi.fn(),
  onOpenUrl: vi.fn(),
  stop: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-deep-link', () => bridge);

it('subscribes before reading startup links and handles later native activations', async () => {
  const listener = vi.fn();
  let receive!: (urls: string[]) => void;
  bridge.onOpenUrl.mockImplementation(async (callback) => {
    receive = callback;
    return bridge.stop;
  });
  bridge.getCurrent.mockResolvedValue(['codex-orchestrator://sessions/cold']);
  const stop = await tauriSessionDeepLinks.subscribe(listener);
  expect(listener).toHaveBeenCalledWith('cold');
  receive(['codex://threads/foreign', 'codex-orchestrator://sessions/warm']);
  expect(listener.mock.calls).toEqual([['cold'], ['warm']]);
  stop();
  expect(bridge.stop).toHaveBeenCalledOnce();
});
