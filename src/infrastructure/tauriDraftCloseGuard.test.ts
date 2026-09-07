import { tauriDraftCloseGuard } from './tauriDraftCloseGuard';
const mocks = vi.hoisted(() => ({ native: false, onCloseRequested: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => mocks.native }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ onCloseRequested: mocks.onCloseRequested }),
}));

it('keeps browser startup independent of native window APIs', async () => {
  mocks.native = false;
  const stop = await tauriDraftCloseGuard(() => true);
  stop();
  expect(mocks.onCloseRequested).not.toHaveBeenCalled();
});

it('allows clean close and honors the user choice for dirty native windows', async () => {
  mocks.native = true;
  let dirty = false;
  let handler!: (event: { preventDefault(): void }) => void;
  mocks.onCloseRequested.mockImplementation(async (next) => {
    handler = next;
    return () => {};
  });
  const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
  await tauriDraftCloseGuard(() => dirty);
  const event = { preventDefault: vi.fn() };
  handler(event);
  expect(confirm).not.toHaveBeenCalled();
  expect(event.preventDefault).not.toHaveBeenCalled();
  dirty = true;
  handler(event);
  expect(event.preventDefault).toHaveBeenCalledOnce();
  confirm.mockReturnValue(true);
  handler(event);
  expect(event.preventDefault).toHaveBeenCalledOnce();
  confirm.mockRestore();
});
