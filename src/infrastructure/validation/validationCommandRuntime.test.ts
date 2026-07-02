import process from 'node:process';

import {
  createNodeValidationCommandProcessRunner,
  createValidationCommandRuntime,
  runValidationCommand,
  type ValidationCommandProcessRunInput,
  type ValidationCommandProcessRunResult,
  type ValidationCommandProcessRunner,
} from './validationCommandRuntime';

describe('createValidationCommandRuntime', () => {
  it('forwards the configured command, args, cwd, env, and chunk callbacks to the runner', async () => {
    const runner = new FakeValidationCommandProcessRunner({
      stdout: 'stdout chunk\n',
      stderr: 'stderr chunk\n',
      exitCode: 0,
      signal: null,
    });
    const runtime = createValidationCommandRuntime({ runner });
    const stdoutChunks: string[] = [];
    const stderrChunks: string[] = [];

    const result = await runtime.run({
      command: 'npm',
      args: ['run', 'lint'],
      cwd: 'C:/repo/worktree',
      env: { CI: '1', OPTIONAL_VALUE: undefined },
      onStdoutChunk: (chunk) => stdoutChunks.push(chunk),
      onStderrChunk: (chunk) => stderrChunks.push(chunk),
    });

    expect(runner.inputs).toEqual([
      {
        command: 'npm',
        args: ['run', 'lint'],
        cwd: 'C:/repo/worktree',
        env: { CI: '1', OPTIONAL_VALUE: undefined },
        onStdoutChunk: expect.any(Function) as (chunk: string) => void,
        onStderrChunk: expect.any(Function) as (chunk: string) => void,
      },
    ]);
    expect(stdoutChunks).toEqual(['stdout chunk\n']);
    expect(stderrChunks).toEqual(['stderr chunk\n']);
    expect(result).toEqual({
      stdout: 'stdout chunk\n',
      stderr: 'stderr chunk\n',
      exitCode: 0,
      signal: null,
    });
  });

  it('uses an empty args array when no args are configured', async () => {
    const runner = new FakeValidationCommandProcessRunner({
      stdout: '',
      stderr: '',
      exitCode: 0,
      signal: null,
    });
    const runtime = createValidationCommandRuntime({ runner });

    await runtime.run({
      command: 'npm',
      cwd: 'C:/repo/worktree',
    });

    expect(runner.inputs).toEqual([
      {
        command: 'npm',
        args: [],
        cwd: 'C:/repo/worktree',
      },
    ]);
  });
});

describe('runValidationCommand', () => {
  it('returns non-zero exit metadata without classifying validation status', async () => {
    const runner = new FakeValidationCommandProcessRunner({
      stdout: '',
      stderr: 'lint failed\n',
      exitCode: 2,
      signal: null,
    });

    const result = await runValidationCommand(
      {
        command: 'npm',
        args: ['run', 'lint'],
        cwd: 'C:/repo/worktree',
      },
      { runner },
    );

    expect(result).toEqual({
      stdout: '',
      stderr: 'lint failed\n',
      exitCode: 2,
      signal: null,
    });
  });

  it('propagates runner launch failures', async () => {
    const runner: ValidationCommandProcessRunner = {
      run: async () => {
        throw new Error('spawn npm ENOENT');
      },
    };

    await expect(
      runValidationCommand(
        {
          command: 'npm',
          args: ['run', 'test'],
          cwd: 'C:/repo/worktree',
        },
        { runner },
      ),
    ).rejects.toThrow('spawn npm ENOENT');
  });
});

describe('createNodeValidationCommandProcessRunner', () => {
  it('executes a command with forwarded args, cwd, env overrides, and raw stream capture', async () => {
    const runner = createNodeValidationCommandProcessRunner();
    const stdoutChunks: string[] = [];
    const stderrChunks: string[] = [];
    const previousParentEnv = process.env.VALIDATION_ADAPTER_PARENT_ENV;
    process.env.VALIDATION_ADAPTER_PARENT_ENV = 'parent-env';

    try {
      const result = await runner.run({
        command: process.execPath,
        args: [
          '-e',
          [
            'process.stdout.write(JSON.stringify({',
            'argv: process.argv.slice(1),',
            'cwd: process.cwd(),',
            'parentEnv: process.env.VALIDATION_ADAPTER_PARENT_ENV,',
            'childEnv: process.env.VALIDATION_ADAPTER_CHILD_ENV',
            "}) + '\\n');",
            "process.stderr.write('adapter stderr\\n');",
          ].join(''),
          'alpha',
          'two words',
        ],
        cwd: process.cwd(),
        env: {
          VALIDATION_ADAPTER_PARENT_ENV: 'overridden-env',
          VALIDATION_ADAPTER_CHILD_ENV: 'child-env',
        },
        onStdoutChunk: (chunk) => stdoutChunks.push(chunk),
        onStderrChunk: (chunk) => stderrChunks.push(chunk),
      });

      const payload = JSON.parse(result.stdout.trim()) as {
        argv: string[];
        cwd: string;
        parentEnv: string;
        childEnv: string;
      };

      expect(result.exitCode).toBe(0);
      expect(result.signal).toBeNull();
      expect(result.stdout).toBe(stdoutChunks.join(''));
      expect(result.stderr).toBe(stderrChunks.join(''));
      expect(result.stderr).toBe('adapter stderr\n');
      expect(payload.argv).toEqual(['alpha', 'two words']);
      expect(payload.cwd.replace(/\\/g, '/')).toBe(process.cwd().replace(/\\/g, '/'));
      expect(payload.parentEnv).toBe('overridden-env');
      expect(payload.childEnv).toBe('child-env');
    } finally {
      if (previousParentEnv === undefined) {
        delete process.env.VALIDATION_ADAPTER_PARENT_ENV;
      } else {
        process.env.VALIDATION_ADAPTER_PARENT_ENV = previousParentEnv;
      }
    }
  });

  it('returns normally when the process exits non-zero', async () => {
    const runner = createNodeValidationCommandProcessRunner();

    const result = await runner.run({
      command: process.execPath,
      args: ['-e', "process.stderr.write('validation failed\\n'); process.exit(7);"],
      cwd: process.cwd(),
    });

    expect(result).toEqual({
      stdout: '',
      stderr: 'validation failed\n',
      exitCode: 7,
      signal: null,
    });
  });

  it('rejects when the process cannot be launched', async () => {
    const runner = createNodeValidationCommandProcessRunner();

    await expect(
      runner.run({
        command: `missing-validation-command-${Date.now()}`,
        args: [],
        cwd: process.cwd(),
      }),
    ).rejects.toThrow();
  });
});

class FakeValidationCommandProcessRunner implements ValidationCommandProcessRunner {
  readonly inputs: ValidationCommandProcessRunInput[] = [];

  constructor(private readonly result: ValidationCommandProcessRunResult) {}

  async run(input: ValidationCommandProcessRunInput): Promise<ValidationCommandProcessRunResult> {
    this.inputs.push(input);
    input.onStdoutChunk?.(this.result.stdout);
    input.onStderrChunk?.(this.result.stderr);

    return this.result;
  }
}
