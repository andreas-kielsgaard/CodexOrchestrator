import { describe, expect, it } from 'vitest';
import { createLocalOrchestrationClient } from './localOrchestrationClient';

describe('createLocalOrchestrationClient', () => {
  it('creates local-only orchestration drafts with integration-pending truth', async () => {
    let id = 1;
    const client = createLocalOrchestrationClient({
      now: () => '2026-07-07T10:00:00.000Z',
      nextId: (prefix) => `${prefix}-${id++}`,
    });

    const buildPackage = await client.createDraft({
      title: 'Migration orchestration',
      folderPath: 'C:/orchestrations/migration',
      prompt: 'Stabilize the repo before live orchestration.',
      files: [{ id: 'file-1', name: 'handoff.md', size: 10 }],
    });

    expect(buildPackage.clientState).toMatchObject({
      id: buildPackage.id,
      status: 'integration_pending',
      provenance: 'unsupported',
      persisted: false,
      runtimeSupported: false,
    });
    expect(buildPackage.stages[0].state).toEqual({
      status: 'integration_pending',
      provenance: 'unsupported',
    });
    expect(buildPackage.generatedFiles).toEqual([]);
    expect(buildPackage.stageRuns).toEqual([]);
    expect(buildPackage.runtimeRoutes).toEqual([
      expect.objectContaining({
        stageId: 'plan-builder',
        status: 'blocked',
        truth: { status: 'blocked', provenance: 'unsupported' },
        runtimeCommand: 'startCodexTaskRun',
      }),
    ]);
    expect(buildPackage.runtimeRoutes?.[0]).not.toHaveProperty('cwd');
    expect(buildPackage.runtimeRoutes?.[0]).not.toHaveProperty('taskId');
    expect(buildPackage.runtimeRoutes?.[0]).not.toHaveProperty('worktreeId');
  });

  it('does not synthesize completed backend stages when a build stage is requested', async () => {
    let id = 1;
    const client = createLocalOrchestrationClient({
      now: () => '2026-07-07T10:00:00.000Z',
      nextId: (prefix) => `${prefix}-${id++}`,
    });
    const buildPackage = await client.createDraft({
      title: 'Migration orchestration',
      folderPath: 'C:/orchestrations/migration',
      prompt: 'Stabilize the repo before live orchestration.',
      files: [],
    });

    const updatedBuild = await client.requestBuildStage({
      buildPackageId: buildPackage.id,
      stageId: 'plan-builder',
    });

    expect(updatedBuild.stages[0].state).toEqual({
      status: 'blocked',
      provenance: 'unsupported',
    });
    expect(updatedBuild.stages.some((stage) => stage.state.status === 'completed')).toBe(false);
    expect(
      updatedBuild.generatedFiles.some((file) => file.state.provenance === 'backend_response'),
    ).toBe(false);
    expect(updatedBuild.clientState.notices[0]).toMatchObject({
      kind: 'blocker',
      truth: { status: 'blocked', provenance: 'unsupported' },
    });
    expect(updatedBuild.stageRuns).toEqual([]);
    expect(updatedBuild.runtimeRoutes?.[0]).toMatchObject({
      stageId: 'plan-builder',
      status: 'blocked',
    });
  });

  it('preserves drafts and records unsupported evidence when Plan Builder start is requested', async () => {
    let id = 1;
    const client = createLocalOrchestrationClient({
      now: () => '2026-07-07T10:00:00.000Z',
      nextId: (prefix) => `${prefix}-${id++}`,
    });
    const buildPackage = await client.createDraft({
      title: 'Migration orchestration',
      folderPath: 'C:/orchestrations/migration',
      prompt: 'Stabilize the repo before live orchestration.',
      files: [{ id: 'file-1', name: 'handoff.md', size: 10 }],
    });

    const updatedBuild = await client.startPlanBuilderRun({
      buildPackageId: buildPackage.id,
    });

    expect(updatedBuild.sourcePrompt).toBe('Stabilize the repo before live orchestration.');
    expect(updatedBuild.files).toHaveLength(1);
    expect(updatedBuild.clientState.runtimeSupported).toBe(false);
    expect(updatedBuild.clientState.notices[0]).toMatchObject({
      kind: 'blocker',
      truth: { status: 'blocked', provenance: 'unsupported' },
    });
    expect(updatedBuild.stageRuns).toEqual([
      expect.objectContaining({
        buildPackageId: buildPackage.id,
        stageId: 'plan-builder',
        state: { status: 'blocked', provenance: 'unsupported' },
        evidence: expect.objectContaining({
          runtimeRoute: 'local',
          unsupported: true,
        }),
      }),
    ]);
    expect(updatedBuild.stageRuns?.[0]).not.toHaveProperty('promptArtifactId');
    expect(updatedBuild.stageRuns?.[0]).not.toHaveProperty('rawEventArtifactId');
    expect(updatedBuild.stageRuns?.[0]).not.toHaveProperty('outputArtifactId');
    expect(updatedBuild.messages.at(-1)?.body).toContain('no attached file contents were sent');
  });

  it('preserves feedback locally without claiming runtime continuation', async () => {
    let id = 1;
    const client = createLocalOrchestrationClient({
      now: () => '2026-07-07T10:00:00.000Z',
      nextId: (prefix) => `${prefix}-${id++}`,
    });
    const buildPackage = await client.createDraft({
      title: 'Migration orchestration',
      folderPath: 'C:/orchestrations/migration',
      prompt: 'Stabilize the repo before live orchestration.',
      files: [],
    });

    const updatedBuild = await client.addDraftNote({
      buildPackageId: buildPackage.id,
      body: 'Please narrow the plan.',
    });

    expect(updatedBuild.messages.at(-2)).toMatchObject({
      role: 'user',
      body: 'Please narrow the plan.',
      truth: { status: 'draft', provenance: 'local_draft' },
    });
    expect(updatedBuild.messages.at(-1)).toMatchObject({
      role: 'system',
      body: expect.stringContaining('not sent to the same Plan Builder runtime conversation'),
      truth: { status: 'integration_pending', provenance: 'unsupported' },
    });
  });
});
