export interface BackendMaintenanceResult {
  status: 'current' | 'restarting' | 'failed';
  stale: boolean;
  checkedAt: string;
  newestSourcePath?: string;
  newestSourceModifiedAt?: string;
  executableModifiedAt?: string;
  message: string;
}

export interface BackendMaintenanceCapability {
  checkAndReopenBackend(): Promise<BackendMaintenanceResult>;
}
