import type { ProviderConfigurationRefDto } from '../agentProviders/contracts';
import type { SessionFolderTarget } from './organization';

export interface AgentSessionQuickFeatures {
  readonly configuration: import('../agentProviders/contracts').ProviderConfigurationRefDto | null;
  readonly models: readonly {
    readonly id: string;
    readonly label: string;
    readonly description: string;
    readonly defaultReasoningMode: string | null;
    readonly reasoningModes: readonly { readonly id: string; readonly description: string }[];
  }[];
  readonly skills: readonly {
    readonly id: string;
    readonly name: string;
    readonly description: string;
    /** Orchid's `$name` mention; the selected provider delivers the skill natively. */
    readonly invocationText: string;
  }[];
  readonly defaults: { readonly model: string | null; readonly reasoningMode: string | null };
  readonly limitations: readonly string[];
}

export interface LoadAgentSessionQuickFeaturesInput {
  readonly executionTarget?: import('../executionTargets/contracts').SessionExecutionTargetDto;
  readonly sessionId: string | null;
  readonly workingDirectory: string | null;
  readonly configuration?: ProviderConfigurationRefDto;
  readonly folderTarget?: SessionFolderTarget;
}
