import { composeProductOrchestrationReadModels, type ProductReadModelsV1 } from '../application/orchestrations';
import { recordedProductReadCompositionInput } from '../dev/orchestrationSection/recordedProductReadCompositionInput';
import { presentProductOrchestrations } from './orchestrationPresentation';

describe('Epic/Sprint reassessment presentation', () => {
  it('carries the same unresolved receiver fact to Epic Detail and Sprint Workspace', () => {
    const readModels = structuredClone(
      composeProductOrchestrationReadModels(recordedProductReadCompositionInput),
    ) as ProductReadModelsV1 & { epics: Array<ProductReadModelsV1['epics'][number] & { epicEscalationReceivers: unknown[] }> };
    const receiver = {
      epicId: readModels.epics[0]!.epicId,
      sprintId: readModels.epics[0]!.sprints[0]!.sprintId,
      deliveryRequestedAt: '2026-08-05T00:00:00Z',
      semanticReassessmentRecordedAt: '2026-08-05T00:00:01Z',
      disposition: {
        movementKind: 'consider_other_epic_work',
        rationale: 'Intent only.',
        consideredIntent: 'Consider another bounded Epic responsibility.',
      },
    };
    readModels.epics[0]!.epicEscalationReceivers = [receiver];
    const sprint = readModels.epics[0]!.sprints[0]! as ProductReadModelsV1['epics'][number]['sprints'][number] & { epicEscalationReceivers: unknown[] };
    sprint.epicEscalationReceivers = [receiver];

    const view = presentProductOrchestrations(readModels);
    expect(view.epics[0]!.epicEscalationReceivers).toMatchObject([
      { disposition: { movementKind: 'consider_other_epic_work' } },
    ]);
    expect(view.epics[0]!.plan.items[0]!.workspace?.epicEscalationReceivers).toMatchObject([
      { disposition: { consideredIntent: 'Consider another bounded Epic responsibility.' } },
    ]);
  });
});
