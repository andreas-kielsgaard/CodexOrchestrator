import { invoke } from '@tauri-apps/api/core';
import type { OtpCatalogueReader, OtpPackageDto } from '../../application/otp';

export function createTauriOtpCatalogueReader(
  call: typeof invoke = invoke,
): OtpCatalogueReader {
  return () => call<readonly OtpPackageDto[]>('list_otp_catalogue');
}

export const tauriOtpCatalogueReader = createTauriOtpCatalogueReader();
