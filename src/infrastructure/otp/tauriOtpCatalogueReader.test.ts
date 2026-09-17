import { vi } from 'vitest';
import { createTauriOtpCatalogueReader } from './tauriOtpCatalogueReader';

it('reads the product-wide OTP catalogue through its dedicated command', async () => {
  const invoke = vi.fn().mockResolvedValue([]);
  await createTauriOtpCatalogueReader(invoke)();
  expect(invoke).toHaveBeenCalledWith('list_otp_catalogue');
});
