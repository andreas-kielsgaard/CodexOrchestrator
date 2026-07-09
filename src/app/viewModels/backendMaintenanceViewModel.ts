export type BackendMaintenanceViewStatus =
  | 'idle'
  | 'checking'
  | 'current'
  | 'restarting'
  | 'failed';

export interface BackendMaintenanceProjectionInput {
  status: BackendMaintenanceViewStatus;
  message: string;
  newestSourcePath?: string;
  available: boolean;
}

export interface BackendMaintenanceViewModel {
  status: BackendMaintenanceViewStatus;
  label: string;
  message: string;
  title?: string;
  available: boolean;
  busy: boolean;
  disabled: boolean;
}

const backendMaintenanceLabels = {
  idle: 'Rust backend',
  checking: 'Checking backend',
  current: 'Backend current',
  restarting: 'Reopening backend',
  failed: 'Backend check failed',
} satisfies Record<BackendMaintenanceViewStatus, string>;

export function createBackendMaintenanceViewModel(
  state: BackendMaintenanceProjectionInput,
): BackendMaintenanceViewModel {
  const busy = state.status === 'checking';
  const title = state.newestSourcePath ?? state.message;

  return {
    status: state.status,
    label: backendMaintenanceLabels[state.status],
    message: state.message,
    ...(title === undefined ? {} : { title }),
    available: state.available,
    busy,
    disabled: !state.available || busy,
  };
}
