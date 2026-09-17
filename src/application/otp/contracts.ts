export interface OtpCapabilityRefDto {
  readonly package: string;
  readonly tool: string;
}
export interface OtpOutputRefDto {
  readonly capability: OtpCapabilityRefDto;
  readonly output: string;
}
export interface OtpOutputDto {
  readonly id: string;
  readonly name: string;
  readonly kind: 'data' | 'session_request' | 'session_stop_request';
  readonly schema: {
    readonly type: string;
    readonly properties?: Readonly<Record<string, { readonly type: string }>>;
  };
}
export interface OtpConfigurationFieldDto {
  readonly key: string;
  readonly label: string;
  readonly choices: readonly string[];
  readonly defaultValue: string;
  readonly when: readonly [string, string] | null;
}
export interface OtpToolDto {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly entrypoint:
    | { readonly kind: 'mcp'; readonly inputSchema: Readonly<Record<string, unknown>> }
    | { readonly kind: 'session_event'; readonly event: 'invocation_terminal' }
    | { readonly kind: 'action'; readonly usesPrompt: boolean };
  readonly outputs: readonly OtpOutputDto[];
  readonly configuration: readonly OtpConfigurationFieldDto[];
}
export interface OtpAgentMcpToolDto {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly capability: string;
}
export interface OtpAgentMcpServerDto {
  readonly serverName: string;
  readonly name: string;
  readonly description: string;
  readonly tools: readonly OtpAgentMcpToolDto[];
  readonly configuration: readonly OtpConfigurationFieldDto[];
}
export interface OtpPackageDto {
  readonly id: string;
  readonly contractVersion: number;
  readonly requestedHandles: readonly ('definitions' | 'node_sessions' | 'emit_output')[];
  readonly tools: readonly OtpToolDto[];
  readonly agentMcpServers: readonly OtpAgentMcpServerDto[];
}
export type OtpCatalogueReader = () => Promise<readonly OtpPackageDto[]>;

export interface JobAgentOtpInstallationDto {
  readonly root: string;
  readonly python: string;
}
export interface JobAgentOtpInstallationStatusDto {
  readonly installation: JobAgentOtpInstallationDto | null;
  readonly status: 'unconfigured' | 'verified' | 'incompatible';
  readonly detail: string;
}
export interface OtpInstallationClient {
  readJobAgentInstallation(): Promise<JobAgentOtpInstallationStatusDto>;
  saveJobAgentInstallation(
    installation: JobAgentOtpInstallationDto,
  ): Promise<JobAgentOtpInstallationStatusDto>;
}
