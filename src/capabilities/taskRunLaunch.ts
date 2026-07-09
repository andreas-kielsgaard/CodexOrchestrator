import type {
  StartCodexTaskRunCommandInput,
  StartCodexTaskRunCommandResult,
} from '../application/commands/runtimeCommandClient';

export type {
  StartCodexTaskRunCommandInput,
  StartCodexTaskRunCommandResult,
} from '../application/commands/runtimeCommandClient';

export interface TaskRunLaunchCapability {
  startCodexTaskRun(
    input: StartCodexTaskRunCommandInput,
  ): Promise<StartCodexTaskRunCommandResult>;
}
