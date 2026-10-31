import { invoke } from '@tauri-apps/api/core';
import type { RuntimeCoordinatorClient } from '../../application/runtimeCoordinator/contracts';

export const tauriRuntimeCoordinatorClient: RuntimeCoordinatorClient = {
  overview: () => invoke('runtime_coordinator_overview'),
  dockerSettings: () => invoke('runtime_coordinator_docker_settings'),
  settings: (input) => invoke('runtime_coordinator_settings', { input }),
  connect: (token) => invoke('runtime_coordinator_connect', { token }),
  register: (input) => invoke('runtime_coordinator_register', { input }),
  refresh: (id) => invoke('runtime_coordinator_refresh', { id }),
  rename: (id, name) => invoke('runtime_coordinator_rename', { id, name }),
  power: (id, input) => invoke('runtime_coordinator_power', { id, input }),
};
