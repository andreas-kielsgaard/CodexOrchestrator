export type AgentCliUpdatePhase =
  | 'checking'
  | 'updating'
  | 'current'
  | 'updated'
  | 'controller_managed'
  | 'failed'
  | 'unavailable';

export interface AgentCliUpdateStatusDto {
  readonly provider: string;
  readonly label: string;
  readonly phase: AgentCliUpdatePhase;
  readonly channel: string;
  readonly previousVersion: string | null;
  readonly installedVersion: string | null;
  readonly checkedAt: string | null;
  readonly summary: string;
  readonly notes: readonly string[];
}

export interface AgentCliUpdateOverviewDto {
  readonly providers: readonly AgentCliUpdateStatusDto[];
}

export interface AgentCliUpdateClient {
  overview(): Promise<AgentCliUpdateOverviewDto>;
}
