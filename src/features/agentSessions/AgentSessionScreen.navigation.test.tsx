import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { composeProductOrchestrationReadModels } from '../../application/orchestrations';
import type { ProductReadModelsV1 } from '../../application/orchestrations';
import { recordedProductReadCompositionInput } from '../../dev/orchestrationSection/recordedProductReadCompositionInput';
import { recordedDevelopmentAgentSessionClient } from '../../dev/orchestrationSection/recordedOrchestrationClient';
import { StandaloneAgentSessionScreen } from './AgentSessionScreen';

describe('standalone Agent Session product navigation', () => {
  it('offers an explicit chooser when durable references lead to multiple views', async () => {
    const read = structuredClone(
      composeProductOrchestrationReadModels(recordedProductReadCompositionInput),
    ) as Mutable<ProductReadModelsV1>;
    const sessionId = 'recorded-epic-runner-manual-continuation-ready';
    read.epics[0].agentSessionReferences.push({
      agentSessionRefId: 'session-ref-additional-sprint-view',
      agentSessionId: sessionId,
      title: 'Orientation discovery handler',
      source: read.epics[0].source,
      targetKind: 'sprint',
      targetId: 'sprint-control-surface',
      semanticRole: 'sprint',
    });
    const navigate = vi.fn();
    render(
      <StandaloneAgentSessionScreen
        client={recordedDevelopmentAgentSessionClient}
        orchestrations={read}
        selectedSessionId={sessionId}
        onNavigateToProduct={navigate}
      />,
    );

    const chooser = await screen.findByText('Related product views');
    fireEvent.click(chooser);
    const sprint = screen.getByRole('button', {
      name: /Sprint Sprint Control Surface Discovery/i,
    });
    fireEvent.click(sprint);
    expect(navigate).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'sprint', sprintId: 'sprint-control-surface' }),
    );
  });

  it('omits product navigation when no typed relationship exists', async () => {
    render(
      <StandaloneAgentSessionScreen
        client={recordedDevelopmentAgentSessionClient}
        orchestrations={composeProductOrchestrationReadModels(recordedProductReadCompositionInput)}
        selectedSessionId="recorded-independent-research"
        onNavigateToProduct={vi.fn()}
      />,
    );

    expect(
      await screen.findByRole('heading', { name: 'Independent product research' }),
    ).toBeVisible();
    expect(screen.queryByRole('button', { name: /Go to/ })).toBeNull();
    expect(screen.queryByText('Related product views')).toBeNull();
  });

  it('fails closed for invocation focus when a different Session is selected', async () => {
    const origin = {
      sessionId: 'recorded-session-WU-ECS2E',
      invocationId: 'recorded-handler-WU-ECS2E-first-review',
      location: {
        kind: 'work_unit' as const,
        epicId: 'epic-runner',
        sprintId: 'sprint-control-surface',
        revisionId: 'ECS-R4',
        workSlicePlanningPointId: 'planning-point-ECS2E',
        workUnitId: 'WU-ECS2E',
        label: 'Plan and Work Unit detail surfaces',
        inspectionState: {
          tab: 'activity' as const,
          activityId:
            'work-unit-inspection:WU-ECS2E:WU-ECS2E-attempt-1:handler-action:recorded-handler-WU-ECS2E-first-review',
          sessionId: 'recorded-session-WU-ECS2E',
          invocationId: 'recorded-handler-WU-ECS2E-first-review',
        },
      },
    };
    const foreignInvocation = document.createElement('button');
    foreignInvocation.dataset.invocationId = origin.invocationId;
    const focus = vi.spyOn(foreignInvocation, 'focus');
    document.body.append(foreignInvocation);

    render(
      <StandaloneAgentSessionScreen
        client={recordedDevelopmentAgentSessionClient}
        selectedSessionId="recorded-implementer-WU-ECS2E"
        focusInvocationId={origin.invocationId}
        returnOrigin={origin}
        onReturnToProduct={vi.fn()}
      />,
    );

    expect(
      await screen.findByRole('heading', { name: 'Recorded WU-ECS2E Work Unit Implementer' }),
    ).toBeVisible();
    await waitFor(() => expect(focus).not.toHaveBeenCalled());
    expect(screen.getByRole('region', { name: 'Work Unit return context' })).toBeVisible();
    foreignInvocation.remove();
  });
});

type Mutable<T> = {
  -readonly [K in keyof T]: T[K] extends readonly (infer U)[]
    ? Mutable<U>[]
    : T[K] extends object
      ? Mutable<T[K]>
      : T[K];
};
