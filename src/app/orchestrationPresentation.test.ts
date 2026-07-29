import { composeProductOrchestrationReadModels } from '../application/orchestrations';
import { recordedProductReadCompositionInput } from '../dev/orchestrationSection/recordedProductReadCompositionInput';
import { presentProductOrchestrations } from './orchestrationPresentation';

describe('orchestration overview presentation', () => {
  it('preserves typed movement, ready work, and exact navigation from the application read', () => {
    const read = composeProductOrchestrationReadModels(recordedProductReadCompositionInput);
    const epic = presentProductOrchestrations(read).epics[0];

    expect(epic.movement).toEqual({ kind: 'available', items: [] });
    expect(epic.readyWork).toEqual([
      {
        actionId: 'continue-planner-work-unit-sprint',
        label: 'Continue with Planner and Work Unit Interaction Discovery',
        target: {
          kind: 'sprint',
          epicId: 'epic-codex-runner-workspace',
          sprintId: 'sprint-planner-work-unit',
          revisionId: 'sprint-planner-work-unit-r1',
        },
      },
    ]);
    expect(epic.humanInput).toBeNull();
    expect(epic.state).toBe('paused');
  });
});
