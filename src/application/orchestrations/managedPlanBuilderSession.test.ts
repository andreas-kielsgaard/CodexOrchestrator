import { managedPlanBuilderSessionConfiguration } from './managedPlanBuilderSession';

describe('managed Plan Builder Agent Session configuration', () => {
  it('declares the two durable pre-initiation planning capabilities and remaining unsupported work', () => {
    expect(managedPlanBuilderSessionConfiguration.identity).toBe('epic_plan_builder');
    expect(managedPlanBuilderSessionConfiguration.role).toBe('epic_planning');
    expect(managedPlanBuilderSessionConfiguration.implementedCapabilities).toEqual({
      mcpTools: ['get_epic_planning_context', 'save_epic_plan_proposal'],
      durableProposal: 'pre_initiation',
    });
    expect(managedPlanBuilderSessionConfiguration.unsupportedCapabilities).toEqual([
      { capability: 'orchestration_folder_routing', status: 'unsupported' },
    ]);
  });

  it('derives the first-session title from the Epic name with a fallback', () => {
    expect(managedPlanBuilderSessionConfiguration.titleForEpicName(' Suggested Epic ')).toBe(
      'Epic builder session for Suggested Epic',
    );
    expect(managedPlanBuilderSessionConfiguration.titleForEpicName('')).toBe(
      'Epic builder session',
    );
  });
});
