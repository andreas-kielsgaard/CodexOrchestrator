import { composeProductOrchestrationReadModels } from '../../application/orchestrations';
import { presentProductOrchestrations } from '../../app/orchestrationPresentation';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { recordedPresentationAdjunct } from './recordedPresentationAdjunct';
import {
  createRecordedDevelopmentOrchestrationPresentation,
  recordedDevelopmentOrchestrationClient,
  recordedDevelopmentOrchestrationPresentation,
} from './recordedOrchestrationClient';
import { recordedProductReadCompositionInput } from './recordedProductReadCompositionInput';

describe('recorded orchestration composition', () => {
  it('returns the canonical composer result with complete recorded product inputs', async () => {
    const loaded = await recordedDevelopmentOrchestrationClient.load();
    expect(loaded).toEqual({
      kind: 'ready',
      readModels: composeProductOrchestrationReadModels(recordedProductReadCompositionInput),
    });
  });

  it('only enriches the same product presentation tree with narrow recorded adjuncts', () => {
    const reads = composeProductOrchestrationReadModels(recordedProductReadCompositionInput);
    expect(recordedDevelopmentOrchestrationPresentation.present(reads)).toEqual(
      presentProductOrchestrations(reads, recordedPresentationAdjunct),
    );
    const sprintAdjunct = recordedPresentationAdjunct.sprints?.['sprint-control-surface'];
    expect(sprintAdjunct).toBeDefined();
    expect(sprintAdjunct).not.toHaveProperty('workspace');
    expect(sprintAdjunct).not.toHaveProperty('lifecycle');
    expect(sprintAdjunct).not.toHaveProperty('continuation');
    expect(sprintAdjunct).not.toHaveProperty('revisionViews');
  });

  it('retains both ECS2E attempt outcomes with distinct Handler and Implementer Sessions', () => {
    const sprint = composeProductOrchestrationReadModels(
      recordedProductReadCompositionInput,
    ).epics[0].sprints.find(({ sprintId }) => sprintId === 'sprint-control-surface')!;
    const view = sprint.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === 'ECS-R4',
    )!;
    const unit = view.workUnits.find(({ workUnitId }) => workUnitId === 'WU-ECS2E')!;
    expect(unit.attempts).toEqual([
      expect.objectContaining({ attemptId: 'WU-ECS2E-attempt-1', returned: true }),
      expect.objectContaining({ attemptId: 'WU-ECS2E-attempt-2', returned: true }),
    ]);
    expect(unit.reviews).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ attemptId: 'WU-ECS2E-attempt-1', outcome: 'needs_correction' }),
        expect.objectContaining({ attemptId: 'WU-ECS2E-attempt-2', outcome: 'accepted' }),
      ]),
    );
    expect(
      sprint.agentSessionReferences.filter(
        ({ targetKind, targetId }) =>
          targetKind === 'work_unit_execution' && targetId === 'execution-WU-ECS2E',
      ),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          agentSessionId: 'recorded-session-WU-ECS2E',
          semanticRole: 'work_unit_handler',
        }),
        expect.objectContaining({
          agentSessionId: 'recorded-implementer-WU-ECS2E',
          semanticRole: 'work_unit_implementer',
        }),
      ]),
    );
    expect(
      recordedPresentationAdjunct.sprints?.[
        'sprint-control-surface'
      ]?.workspaceAdjunct?.workUnitSessions.filter(({ workUnitId }) => workUnitId === 'WU-ECS2E'),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          sessionId: 'recorded-session-WU-ECS2E',
          role: 'handler',
        }),
        expect.objectContaining({
          sessionId: 'recorded-implementer-WU-ECS2E',
          role: 'implementer',
        }),
      ]),
    );
  });

  it('keeps the representative Work Unit inspection explicitly presentation-only', () => {
    const reads = composeProductOrchestrationReadModels(recordedProductReadCompositionInput);
    const canonicalUnit = reads.epics[0]!.sprints
      .find(({ sprintId }) => sprintId === 'sprint-control-surface')!
      .revisionViews.find(({ sprintPlanRevisionId }) => sprintPlanRevisionId === 'ECS-R4')!
      .workUnits.find(({ workUnitId }) => workUnitId === 'WU-ECS2E')!;
    expect(canonicalUnit.inspection).toBeUndefined();

    const view = createRecordedDevelopmentOrchestrationPresentation({
      includeWorkUnitReview: true,
    }).present(reads);
    const workspace = view.epics[0]!.plan.items.find(
      ({ id }) => id === 'sprint-control-surface',
    )!.workspace!;
    const inspectedUnit = workspace.revisionViews
      .find(({ sprintPlanRevisionId }) => sprintPlanRevisionId === 'ECS-R4')!
      .workUnits.find(({ workUnitId }) => workUnitId === 'WU-ECS2E')!;

    expect(inspectedUnit.inspection).toMatchObject({
      workUnitId: 'WU-ECS2E',
      fileEvidence: { status: 'available', owner: 'application' },
      testEvidence: { owner: 'application' },
    });
    expect(inspectedUnit.inspection!.activities).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          role: 'handler',
          agentSessionId: 'recorded-session-WU-ECS2E',
          invocationId: 'recorded-handler-WU-ECS2E-first-review',
        }),
        expect.objectContaining({
          role: 'implementer',
          agentSessionId: 'recorded-implementer-WU-ECS2E',
          invocationId: 'recorded-implementer-WU-ECS2E-first-return',
        }),
      ]),
    );

    const activities = inspectedUnit.inspection!.activities;
    expect(activities.filter(({ applicationSummary }) => applicationSummary)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ primaryStage: 'implementer_reporting' }),
        expect.objectContaining({ primaryStage: 'handler_review' }),
      ]),
    );
    expect(activities.every(({ primaryStage, applicationSummary }) =>
      (primaryStage === 'implementer_reporting' || primaryStage === 'handler_review') ===
        (applicationSummary !== undefined),
    )).toBe(true);
    const reporting = activities.filter(({ primaryStage }) => primaryStage === 'implementer_reporting');
    expect(reporting.every(({ applicationSummary }) => applicationSummary !== undefined)).toBe(true);
    const handlerReview = activities.find(({ primaryStage }) => primaryStage === 'handler_review')!;
    const peer = activities.find(
      ({ activityId }) => activityId === handlerReview.applicationSummary!.peerEvidenceActivityIds[0],
    )!;
    expect(peer.primaryStage).toBe('implementer_reporting');
    expect(peer.attemptId).toBe(handlerReview.attemptId);
    expect(inspectedUnit.inspection!.fileEvidence).toMatchObject({
      status: 'available',
      sourceActivityId: peer.activityId,
    });
    expect(inspectedUnit.inspection!.activities.some(({ activityId }) =>
      activityId === 'recorded-wu-ecs2e-missing-activity',
    )).toBe(false);
  });

  it('keeps feature tests off the disposable compatibility fixture', () => {
    for (const file of [
      'src/features/orchestrations/OrchestrationSection.test.tsx',
      'src/features/orchestrations/components/SprintFlowMap.test.tsx',
      'src/features/orchestrations/components/SprintDocumentsPanel.test.tsx',
    ]) {
      expect(readFileSync(resolve(file), 'utf8')).not.toMatch(
        /from ['"].*disposableRecordedOrchestrationView/,
      );
    }
  });
});
