import type { OrchestrationBuildPackage } from '../application/orchestrationClient';
import { createTauriOrchestrationClient } from './tauriOrchestrationClient';

describe('createTauriOrchestrationClient', () => {
  it('loads persisted orchestration drafts through the Tauri registry command', async () => {
    const expectedBuild = persistedBuildPackage();
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const client = createTauriOrchestrationClient(
      async <T>(command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return {
          orchestrations: [],
          buildPackages: [expectedBuild],
          clientState: expectedBuild.clientState,
        } as T;
      },
    );

    const registry = await client.loadOrchestrations();

    expect(registry.buildPackages).toEqual([expectedBuild]);
    expect(registry.buildPackages[0].clientState).toMatchObject({
      status: 'integration_pending',
      provenance: 'persisted_snapshot',
      persisted: true,
      runtimeSupported: false,
    });
    expect(calls).toEqual([{ command: 'load_orchestration_registry', args: undefined }]);
  });

  it('sends draft creation and update inputs without adding local runtime facts', async () => {
    const expectedBuild = persistedBuildPackage();
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const client = createTauriOrchestrationClient(
      async <T>(command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return expectedBuild as T;
      },
    );

    await client.createDraft({
      title: 'Persist draft',
      folderPath: 'C:/orchestrations/persist-draft',
      prompt: 'Keep this prompt after reload.',
      files: [{ id: 'file-1', name: 'handoff.md', size: 12 }],
    });
    await client.addDraftNote({ buildPackageId: 'build-1', body: 'Local context note' });
    await client.attachDraftFiles({
      buildPackageId: 'build-1',
      files: [{ id: 'file-2', name: 'roadmap.md', size: 24 }],
    });
    await client.startPlanBuilderRun({ buildPackageId: 'build-1' });

    expect(calls).toEqual([
      {
        command: 'create_orchestration_draft',
        args: {
          input: {
            title: 'Persist draft',
            folderPath: 'C:/orchestrations/persist-draft',
            prompt: 'Keep this prompt after reload.',
            files: [{ id: 'file-1', name: 'handoff.md', size: 12 }],
          },
        },
      },
      {
        command: 'add_orchestration_draft_note',
        args: { input: { buildPackageId: 'build-1', body: 'Local context note' } },
      },
      {
        command: 'attach_orchestration_draft_files',
        args: {
          input: {
            buildPackageId: 'build-1',
            files: [{ id: 'file-2', name: 'roadmap.md', size: 24 }],
          },
        },
      },
      {
        command: 'start_orchestration_plan_builder_run',
        args: { input: { buildPackageId: 'build-1' } },
      },
    ]);
  });

  it('preserves explicit unsupported runtime responses from Tauri commands', async () => {
    const expectedBuild = {
      ...persistedBuildPackage(),
      clientState: {
        ...persistedBuildPackage().clientState,
        provenance: 'unsupported',
        notices: [
          {
            id: 'missing-plan-builder-runtime',
            kind: 'blocker',
            title: 'Plan-builder route required',
            message: 'Plan builder cannot start because no explicit route exists.',
            truth: { status: 'blocked', provenance: 'unsupported' },
          },
        ],
      },
    } satisfies OrchestrationBuildPackage;
    const client = createTauriOrchestrationClient(async <T>() => expectedBuild as T);

    const result = await client.requestBuildStage({
      buildPackageId: 'build-1',
      stageId: 'plan-builder',
    });

    expect(result.stages.some((stage) => stage.state.status === 'completed')).toBe(false);
    expect(result.generatedFiles.some((file) => file.state.provenance === 'backend_response')).toBe(
      false,
    );
    expect(result.clientState.notices[0]).toMatchObject({
      kind: 'blocker',
      truth: { status: 'blocked', provenance: 'unsupported' },
    });
  });

  it('surfaces unsupported live snapshot lookup errors instead of returning null', async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const client = createTauriOrchestrationClient(
      async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        throw new Error(
          'Live orchestration snapshot loading is integration-pending: persisted drafts are available through load_orchestration_registry.',
        );
      },
    );

    await expect(client.loadOrchestration('orch-1')).rejects.toThrow('integration-pending');
    expect(calls).toEqual([{ command: 'load_orchestration', args: { id: 'orch-1' } }]);
  });

  it('surfaces Tauri command failures to the caller', async () => {
    const client = createTauriOrchestrationClient(async () => {
      throw new Error('Unable to persist orchestration draft: database unavailable');
    });

    await expect(
      client.createDraft({
        title: 'Persist draft',
        folderPath: 'C:/orchestrations/persist-draft',
        prompt: 'Keep this prompt after reload.',
        files: [],
      }),
    ).rejects.toThrow('database unavailable');
  });
});

function persistedBuildPackage(): OrchestrationBuildPackage {
  return {
    id: 'build-1',
    title: 'Persist draft',
    folderPath: 'C:/orchestrations/persist-draft',
    sourcePrompt: 'Keep this prompt after reload.',
    createdAt: '2026-07-07T10:00:00.000Z',
    updatedAt: '2026-07-07T10:00:00.000Z',
    clientState: {
      id: 'build-1',
      status: 'integration_pending',
      provenance: 'persisted_snapshot',
      currentAction:
        'Draft is persisted locally; plan-builder runtime and Codex threads are not connected yet.',
      updatedAt: '2026-07-07T10:00:00.000Z',
      persisted: true,
      runtimeSupported: false,
      notices: [],
    },
    messages: [],
    files: [],
    stages: [
      {
        id: 'plan-builder',
        title: 'Plan Builder',
        state: { status: 'integration_pending', provenance: 'unsupported' },
        summary: 'Prompt is saved; no plan-builder output exists yet.',
        detail: 'Backend plan-builder integration is still pending.',
      },
    ],
    stageRuns: [],
    runtimeRoutes: [
      {
        stageId: 'plan-builder',
        status: 'blocked',
        truth: { status: 'blocked', provenance: 'unsupported' },
        reason: 'No explicit Open Task/worktree runtime route is linked.',
        runtimeCommand: 'startCodexTaskRun',
        updatedAt: '2026-07-07T10:00:00.000Z',
      },
    ],
    generatedFiles: [
      {
        name: 'orchestration-plan.json',
        purpose: 'Expected strategic problem map after real plan-builder output exists.',
        state: { status: 'draft', provenance: 'unsupported' },
      },
    ],
    planPreview: ['Keep this prompt after reload.'],
  };
}
