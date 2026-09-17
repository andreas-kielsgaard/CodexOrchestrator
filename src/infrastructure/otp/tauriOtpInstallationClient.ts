import { invoke } from '@tauri-apps/api/core';
import type {
  JobAgentOtpInstallationDto,
  JobAgentOtpInstallationStatusDto,
  OtpInstallationClient,
} from '../../application/otp';

export function createTauriOtpInstallationClient(
  call: typeof invoke = invoke,
): OtpInstallationClient {
  return {
    readJobAgentInstallation: () =>
      call<JobAgentOtpInstallationStatusDto>('read_job_agent_otp_installation'),
    saveJobAgentInstallation: (installation: JobAgentOtpInstallationDto) =>
      call<JobAgentOtpInstallationStatusDto>('save_job_agent_otp_installation', {
        input: { installation },
      }),
  };
}

export const tauriOtpInstallationClient = createTauriOtpInstallationClient();
