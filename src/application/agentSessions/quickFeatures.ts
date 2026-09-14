import type { SessionFolderTarget } from './organization';

export interface AgentSessionQuickFeatures {
  readonly profileRef: string;
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
    /** The provider owns invocation syntax; the composer only inserts it. */
    readonly invocationText: string;
  }[];
  readonly defaults: { readonly model: string | null; readonly reasoningMode: string | null };
  readonly limitations: readonly string[];
}

export interface LoadAgentSessionQuickFeaturesInput {
  readonly executionTarget?: import('../executionTargets/contracts').SessionExecutionTargetDto;
  readonly sessionId: string | null;
  readonly workingDirectory: string | null;
  readonly folderTarget?: SessionFolderTarget;
}
