import { DatabaseSync } from 'node:sqlite';

import { collectTaskDiff } from '../application/diffCollection';
import { composeCodexTaskRun } from '../application/runComposition';
import { runTaskValidationCommand } from '../application/validationCommandRunner';
import type { EntityId, IsoDateTime, Project } from '../domain/model';
import type { RepoSyncPlanIdProvider } from '../domain/repoSyncPlanApplier';
import type {
  AppSqliteStoreBundle,
  AppSqliteStoreBundleProviders,
  AppSqliteWriteStoreProviders,
} from './sqlite/appStore';
import type { AppSqliteMigrationDatabase } from './sqlite/migrationCoordinator';
import { openLocalAppSqliteDatabase, type LocalAppSqliteDatabase } from './sqlite/localAppDatabase';
import {
  createDefaultLocalRuntimeRepoRegistryProviders,
  openLocalRuntimeServiceComposition,
} from './localRuntimeComposition';
import type {
  GitProcessRunInput,
  GitProcessRunResult,
  GitProcessRunner,
} from './git/localGitRuntime';
import type {
  CodexProcessRunInput,
  CodexProcessRunResult,
  CodexProcessRunner,
} from './codex/codexRuntime';
import type {
  ValidationCommandProcessRunInput,
  ValidationCommandProcessRunResult,
  ValidationCommandProcessRunner,
} from './validation/validationCommandRuntime';

const now = '2026-07-02T12:00:00.000Z' as IsoDateTime;

describe('local runtime service composition', () => {
  it('opens one local app database and wires store-backed services over the same bundle', async () => {
    const db = new DatabaseSync(':memory:');
    const gitRunner = new FakeGitProcessRunner({
      stdout: 'diff --git a/src/example.ts b/src/example.ts\n',
      stderr: '',
      exitCode: 0,
      signal: null,
    });
    const codexRunner = new FakeCodexProcessRunner({
      stdout: completedJsonl,
      stderr: '',
      exitCode: 0,
      signal: null,
    });
    const validationRunner = new FakeValidationProcessRunner({
      stdout: 'tests passed\n',
      stderr: '',
      exitCode: 0,
      signal: null,
    });
    let openCount = 0;
    const composition = openLocalRuntimeServiceComposition(':memory:', {
      openDatabase: (databasePath, options) => {
        openCount += 1;
        return openLocalAppSqliteDatabase(databasePath, {
          ...options,
          openConnection: () => db,
        });
      },
      database: {
        initialize: { migrations: { appliedAt: deterministicAppliedAt } },
        providers: createProviders(),
      },
      git: { processRunner: gitRunner },
      codex: { runner: codexRunner },
      validation: { runner: validationRunner },
      repoRegistry: {
        ids: deterministicRepoSyncIds('repo-sync'),
        clock: fixedClock('2026-07-02T12:03:00.000Z'),
      },
    });

    try {
      expect(openCount).toBe(1);
      expect(composition.stores).toBe(composition.database.stores);
      expect(composition.services.taskRunLifecycleRecorder.openTaskDashboardStore).toBe(
        composition.stores.openTaskDashboard,
      );
      expect(composition.services.runCompositionService.recorder).toBe(
        composition.services.taskRunLifecycleRecorder,
      );
      expect(composition.services.repoRegistryScanService.store).toBe(composition.stores.repoSync);
      expect(composition.services.taskWorktreeSelectionService.worktreeCreator).toBe(
        composition.runtimes.git.worktreeCreator,
      );
      expect(composition.services.diffCollectionService.diffProvider).toBe(
        composition.runtimes.git.diffProvider,
      );
      expect(composition.services.validationCommandRunnerService.runtime).toBe(
        composition.runtimes.validation,
      );

      seedProject(composition.database.db, {
        id: 'project-1',
        name: 'Codex Orchestrator',
        createdAt: now,
        updatedAt: now,
      });

      const createdDashboard = await composition.services.taskDashboardClient.createTask({
        projectId: 'project-1',
        title: 'Compose local runtime services',
        summary: 'Use the SQLite-backed service composition boundary.',
      });
      const taskId = createdDashboard.groups.flatMap((group) => group.tasks)[0]?.id;

      if (taskId === undefined) {
        throw new Error('Expected created dashboard task');
      }

      const run = await composeCodexTaskRun(composition.services.runCompositionService, {
        taskId,
        prompt: 'Finish the local runtime composition slice',
        cwd: 'C:/worktrees/codex-orchestrator',
        startedAt: '2026-07-02T12:10:00.000Z' as IsoDateTime,
        completedAt: '2026-07-02T12:20:00.000Z' as IsoDateTime,
      });
      const diff = await collectTaskDiff(composition.services.diffCollectionService, {
        taskId,
        worktreePath: 'C:/worktrees/codex-orchestrator',
      });
      const validation = await runTaskValidationCommand(
        composition.services.validationCommandRunnerService,
        {
          taskId,
          command: 'npm',
          args: ['run', 'test'],
          cwd: 'C:/worktrees/codex-orchestrator',
          startedAt: '2026-07-02T12:21:00.000Z' as IsoDateTime,
          completedAt: '2026-07-02T12:22:00.000Z' as IsoDateTime,
        },
      );
      const reloadedDashboard = await composition.services.taskDashboardClient.loadDashboard();
      const artifacts = await composition.stores.artifact.queryArtifacts({ taskId });
      const events = await composition.stores.event.queryEvents({ taskId });
      const taskRuns = await composition.stores.taskRun.queryTaskRuns({ taskId });
      const validationRuns = await composition.stores.validationRun.queryValidationRuns({ taskId });

      expect(run.status).toBe('completed');
      expect(diff.diff).toBe('diff --git a/src/example.ts b/src/example.ts\n');
      expect(validation.status).toBe('passed');
      expect(codexRunner.calls).toEqual([
        expect.objectContaining({
          command: 'codex',
          args: ['exec', '--json', 'Finish the local runtime composition slice'],
          cwd: 'C:/worktrees/codex-orchestrator',
        }),
      ]);
      expect(gitRunner.calls).toEqual([
        {
          command: 'git',
          args: ['diff', '--binary', 'HEAD', '--'],
          cwd: 'C:/worktrees/codex-orchestrator',
        },
      ]);
      expect(validationRunner.calls).toEqual([
        {
          command: 'npm',
          args: ['run', 'test'],
          cwd: 'C:/worktrees/codex-orchestrator',
        },
      ]);
      expect(reloadedDashboard.groups.flatMap((group) => group.tasks)[0]).toMatchObject({
        id: taskId,
        executionState: 'completed',
        attentionState: 'needs_review',
      });
      expect(taskRuns).toEqual([expect.objectContaining({ id: 'task-run-1' })]);
      expect(artifacts.map((artifact) => artifact.kind).sort()).toEqual([
        'diff',
        'final_response',
        'raw_event_stream',
        'validation_log',
      ]);
      expect(events.map((event) => event.kind)).toEqual([
        'run_started',
        'artifact_created',
        'run_completed',
        'artifact_created',
        'validation_started',
        'artifact_created',
        'validation_completed',
      ]);
      expect(validationRuns).toEqual([
        expect.objectContaining({
          id: 'validation-run-1',
          status: 'passed',
          outputArtifactId: 'artifact-4',
        }),
      ]);

      composition.close();
      composition.dispose();
      expect(() => db.prepare('SELECT 1')).toThrow();
    } finally {
      composition.close();
    }
  });

  it('provides deterministic override points for repo registry ID and clock providers', () => {
    const defaults = createDefaultLocalRuntimeRepoRegistryProviders();
    const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

    expect(defaults.ids.repoId({} as Parameters<typeof defaults.ids.repoId>[0])).toMatch(
      uuidPattern,
    );
    expect(defaults.ids.branchId({} as Parameters<typeof defaults.ids.branchId>[0])).toMatch(
      uuidPattern,
    );
    expect(defaults.ids.worktreeId({} as Parameters<typeof defaults.ids.worktreeId>[0])).toMatch(
      uuidPattern,
    );
    expect(Date.parse(defaults.clock.now())).not.toBeNaN();
  });

  it('forwards close and dispose through injected database handle methods', () => {
    const fakeDatabase = {
      db: {},
      stores: fakeStoreBundle(),
      closed: false,
      disposed: false,
      close() {
        this.closed = true;
      },
      dispose() {
        this.disposed = true;
      },
    };
    const composition = openLocalRuntimeServiceComposition(':memory:', {
      openDatabase: () => fakeDatabase as unknown as LocalAppSqliteDatabase,
    });

    composition.close();
    composition.dispose();

    expect(fakeDatabase.closed).toBe(true);
    expect(fakeDatabase.disposed).toBe(true);
  });
});

const completedJsonl = [
  JSON.stringify({ type: 'thread.started', thread_id: 'thread-038' }),
  JSON.stringify({
    type: 'item.completed',
    item: { type: 'agent_message', text: 'Local runtime composition is ready.' },
  }),
  JSON.stringify({ type: 'turn.completed', usage: { total_tokens: 42 } }),
].join('\n');

class FakeGitProcessRunner implements GitProcessRunner {
  readonly calls: GitProcessRunInput[] = [];

  constructor(private readonly result: GitProcessRunResult) {}

  async run(input: GitProcessRunInput): Promise<GitProcessRunResult> {
    this.calls.push({
      command: input.command,
      args: [...input.args],
      cwd: input.cwd,
    });

    return { ...this.result };
  }
}

class FakeCodexProcessRunner implements CodexProcessRunner {
  readonly calls: Array<Omit<CodexProcessRunInput, 'onStdoutChunk' | 'onStderrChunk'>> = [];

  constructor(private readonly result: CodexProcessRunResult) {}

  async run(input: CodexProcessRunInput): Promise<CodexProcessRunResult> {
    this.calls.push({
      command: input.command,
      args: [...input.args],
      ...(input.cwd === undefined ? {} : { cwd: input.cwd }),
      ...(input.env === undefined ? {} : { env: { ...input.env } }),
    });

    return { ...this.result };
  }
}

class FakeValidationProcessRunner implements ValidationCommandProcessRunner {
  readonly calls: Array<
    Omit<ValidationCommandProcessRunInput, 'env' | 'onStdoutChunk' | 'onStderrChunk'>
  > = [];

  constructor(private readonly result: ValidationCommandProcessRunResult) {}

  async run(input: ValidationCommandProcessRunInput): Promise<ValidationCommandProcessRunResult> {
    this.calls.push({
      command: input.command,
      args: [...input.args],
      cwd: input.cwd,
    });

    return { ...this.result };
  }
}

function deterministicAppliedAt(_migration: unknown, position: number): string {
  return `2026-07-02T12:01:${position.toString().padStart(2, '0')}.000Z`;
}

function seedProject(db: AppSqliteMigrationDatabase, project: Project): void {
  db.prepare(
    `
INSERT INTO projects (id, name, description, created_at, updated_at)
VALUES (?, ?, ?, ?, ?)
`,
  ).run(
    project.id,
    project.name,
    project.description ?? null,
    project.createdAt,
    project.updatedAt,
  );
}

function createProviders(): AppSqliteStoreBundleProviders {
  return {
    openTask: createStoreProvider('task'),
    event: createStoreProvider('event'),
    taskRun: createStoreProvider('task-run'),
    conversation: createStoreProvider('conversation'),
    artifact: createStoreProvider('artifact'),
    validationRun: createStoreProvider('validation-run'),
  };
}

function createStoreProvider(prefix: string): AppSqliteWriteStoreProviders {
  let idIndex = 0;
  let timeIndex = 0;

  return {
    ids: {
      nextId: () => `${prefix}-${++idIndex}` as EntityId,
    },
    clock: {
      now: () =>
        `2026-07-02T12:02:${(timeIndex++).toString().padStart(2, '0')}.000Z` as IsoDateTime,
    },
  };
}

function fixedClock(nowValue: IsoDateTime): { now(): IsoDateTime } {
  return {
    now: () => nowValue,
  };
}

function deterministicRepoSyncIds(prefix: string): RepoSyncPlanIdProvider {
  let index = 0;

  return {
    repoId: () => `${prefix}-repo-${++index}` as EntityId,
    branchId: () => `${prefix}-branch-${++index}` as EntityId,
    worktreeId: () => `${prefix}-worktree-${++index}` as EntityId,
  };
}

function fakeStoreBundle(): AppSqliteStoreBundle {
  return {
    repoSync: {},
    openTaskDashboard: {},
    openTaskWrite: {},
    event: {},
    taskRun: {},
    conversation: {},
    artifact: {},
    validationRun: {},
  } as AppSqliteStoreBundle;
}
