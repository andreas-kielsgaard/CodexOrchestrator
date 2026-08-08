import { render, screen } from '@testing-library/react';
import {
  composeProductOrchestrationReadModels,
  projectSprintWorkspacePresentation,
  type SprintWorkspacePresentationV1,
} from '../../../application/orchestrations';
import { recordedProductReadCompositionInput } from '../../../dev/orchestrationSection/recordedProductReadCompositionInput';
import { SprintWorkspace } from './SprintWorkspace';

describe('Sprint reassessment presentation', () => {
  it('retains the unresolved concern and labels downstream movement as a request only', () => {
    const readModels = composeProductOrchestrationReadModels(recordedProductReadCompositionInput);
    const sourceSprint = readModels.epics[0]!.sprints[0]!;
    const workspace = projectSprintWorkspacePresentation(sourceSprint) as SprintWorkspacePresentationV1 & {
      epicEscalationReceivers: NonNullable<SprintWorkspacePresentationV1['epicEscalationReceivers']>;
    };
    workspace.epicEscalationReceivers = [
      {
        epicId: sourceSprint.epicId,
        sprintId: sourceSprint.sprintId,
        deliveryRequestedAt: '2026-08-05T00:00:00Z',
        semanticReassessmentRecordedAt: '2026-08-05T00:00:01Z',
        disposition: {
          movementKind: 'return_context_to_sprint_runner',
          rationale: 'The concern remains unresolved.',
          downstreamRequest: {
            target: 'sprint_runner',
            request: 'Reconsider the same Sprint-local concern.',
            resumptionPath: 'Resume from the unchanged concern.',
          },
        },
      },
    ];

    render(
      <SprintWorkspace
        workspace={workspace}
        artifactAccessController={undefined as never}
        selectedRevisionId={workspace.selectedSprintPlanRevisionId}
        onSelectedRevisionChange={vi.fn()}
        detailLocation={{ kind: 'sprint' }}
        onDetailLocationChange={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    const region = screen.getByRole('region', { name: 'Unresolved Epic reassessment' });
    expect(region).toHaveTextContent('The concern remains unresolved');
    expect(region).toHaveTextContent('Downstream request recorded only');
    expect(region).toHaveTextContent('not delivery or activation');
    expect(region).toHaveTextContent('has not cleared this Sprint concern');
  });
});
