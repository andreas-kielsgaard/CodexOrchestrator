import type { EntityId, IsoDateTime } from '../domain/model';
import { AgentCLISessionInterface } from './agentCLISessionInterface';
import {
  CLIInstanceHandler,
  type CLIInstanceOpenInput,
  type CLIInstanceRunResult,
  type CLIInstanceRunner,
} from './cliInstanceHandler';

describe('AgentCLISessionInterface', () => {
  it('maps prompts and session ids to codex exec resume arguments', async () => {
    const runner = new CapturingRunner();
    const cli = new AgentCLISessionInterface(new CLIInstanceHandler(runner));

    await cli.deliverPrompt({
      sessionId: 'agent-session-real' as EntityId,
      prompt: 'Continue',
      additionalArgs: ['--model', 'gpt-5.5'],
      cwd: 'C:/repo',
      env: { TEST: '1' },
    });

    expect(runner.inputs).toEqual([
      {
        sessionId: 'agent-session-real',
        command: 'codex',
        args: ['exec', '--json', '--model', 'gpt-5.5', 'resume', 'agent-session-real', 'Continue'],
        cwd: 'C:/repo',
        env: { TEST: '1' },
      },
    ]);
  });
});

class CapturingRunner implements CLIInstanceRunner {
  inputs: CLIInstanceOpenInput[] = [];

  async run(input: CLIInstanceOpenInput): Promise<CLIInstanceRunResult> {
    this.inputs.push(input);
    return {
      sessionId: input.sessionId ?? ('agent-session-created' as EntityId),
      status: 'completed',
      command: input.command,
      args: [...input.args],
      stdout: '',
      stderr: '',
      startedAt: '2026-07-09T10:00:00.000Z' as IsoDateTime,
      completedAt: '2026-07-09T10:00:01.000Z' as IsoDateTime,
      exitCode: 0,
    };
  }
}
