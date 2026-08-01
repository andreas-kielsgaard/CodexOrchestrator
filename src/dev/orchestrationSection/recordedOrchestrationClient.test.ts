import { composeProductOrchestrationReadModels } from '../../application/orchestrations';
import { presentProductOrchestrations } from '../../app/orchestrationPresentation';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { recordedPresentationAdjunct } from './recordedPresentationAdjunct';
import {
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
