import { recordedPlanWorkflow } from '../../dev/orchestrationSection/disposableRecordedOrchestrationView';
import { decodeRecordedPlanWorkflowV1 } from './recordedPlanWorkflow';

describe('Plan workflow projection', () => {
  it('validates the recorded Work Unit path with completion after settlement', () => {
    expect(decodeRecordedPlanWorkflowV1(recordedPlanWorkflow)).toBe(recordedPlanWorkflow);
    expect(recordedPlanWorkflow.fixtureKind).toBe('recorded_theoretical');
    expect(recordedPlanWorkflow.sprintPlannerActivityId).toBe('sprint-planner-activity-recorded-1');
    expect(recordedPlanWorkflow).not.toHaveProperty('planId');
    expect(recordedPlanWorkflow.workUnitLanes).toHaveLength(1);
    expect(recordedPlanWorkflow.workUnitLanes[0].steps.map(({ kind }) => kind)).toEqual(
      expect.arrayContaining(['worker_return', 'initiator_review', 'work_unit_settled']),
    );
    expect(recordedPlanWorkflow.interactions).toEqual([
      expect.objectContaining({ kind: 'return' }),
    ]);
    expect(recordedPlanWorkflow.sharedCompletion.map(({ kind }) => kind)).toEqual([
      'planner_completed',
      'sprint_outcome',
    ]);
  });

  it('rejects workflow interactions that reference an unknown step', () => {
    expect(() =>
      decodeRecordedPlanWorkflowV1({
        ...structuredClone(recordedPlanWorkflow),
        interactions: [{ ...recordedPlanWorkflow.interactions[0], toStepId: 'unknown-step' }],
      }),
    ).toThrow('Invalid Plan workflow data');
  });
});
