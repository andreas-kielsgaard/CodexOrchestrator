import {
  projectSprintWorkspacePresentation,
  type ProductReadModelsV1,
} from '../application/orchestrations';
import type {
  SprintPlanItemPresentation,
  EpicMovementPresentation,
  OrchestrationSectionView,
  EpicStatePresentation,
} from '../features/orchestrations';

/**
 * Product facts are the authority for the shared overview. Recorded-only adjuncts may add the
 * accepted discovery details until their dedicated controllers are integrated in later units.
 */
export interface OrchestrationPresentationAdapter {
  present(readModels: ProductReadModelsV1): OrchestrationSectionView;
}

/** Only deferred recorded presentation details may enrich canonical product structure. */
export interface RecordedPresentationAdjunct {
  readonly epic?: Readonly<{
    readonly epicRunnerSession?: OrchestrationSectionView['epics'][number]['epicRunnerSession'];
  }>;
  readonly sprints?: Readonly<
    Record<string, Readonly<Pick<SprintPlanItemPresentation, 'agentSession' | 'workspaceAdjunct'>>>
  >;
}

export const productOrchestrationPresentationAdapter: OrchestrationPresentationAdapter = {
  present(readModels) {
    return presentProductOrchestrations(readModels);
  },
};

export function presentProductOrchestrations(
  readModels: ProductReadModelsV1,
  adjunct: RecordedPresentationAdjunct = {},
): OrchestrationSectionView {
  return {
    epics: readModels.epics.map((epic) => ({
      id: epic.epicId,
      name: epic.title,
      goal: epic.goal,
      movement: presentationMovement(epic.overview.currentMovement),
      state: presentationState(epic.overview.state),
      readyWork: presentationReadyWork(epic.overview.readyWork),
      humanInput: presentationHumanInput(epic.overview.humanInput),
      plan: {
        items: epic.sprints.map((sprint) => ({
          id: sprint.sprintId,
          name: sprint.title,
          purpose: sprint.summary,
          status: sprint.lifecycle
            ? presentationSprintStatus(sprint.lifecycle)
            : unavailableSprintStatus(),
          detail: { summary: sprint.summary, outcome: sprint.details },
          workspace: projectSprintWorkspacePresentation(sprint),
          ...adjunct.sprints?.[sprint.sprintId],
        })),
      },
      ...(adjunct.epic?.epicRunnerSession
        ? { epicRunnerSession: adjunct.epic.epicRunnerSession }
        : {}),
      ...presentationContinuation(epic.continuation, epic.epicId),
      ...(epic.bootstrapTransition ? { bootstrapTransition: epic.bootstrapTransition } : {}),
    })),
  };
}

function presentationMovement(
  value: ProductReadModelsV1['epics'][number]['overview']['currentMovement'],
): EpicMovementPresentation {
  return value.source.status === 'available'
    ? { kind: 'available', items: value.value!.items }
    : { kind: value.source.status, reason: value.source.reason };
}

function presentationState(
  value: ProductReadModelsV1['epics'][number]['overview']['state'],
): EpicStatePresentation {
  return value.source.status === 'available'
    ? value.value!
    : { kind: value.source.status, reason: value.source.reason };
}

function presentationReadyWork(
  value: ProductReadModelsV1['epics'][number]['overview']['readyWork'],
): OrchestrationSectionView['epics'][number]['readyWork'] {
  return value.source.status === 'available'
    ? value.value!
    : { kind: value.source.status, reason: value.source.reason };
}

function presentationHumanInput(
  value: ProductReadModelsV1['epics'][number]['overview']['humanInput'],
): OrchestrationSectionView['epics'][number]['humanInput'] {
  return value.source.status === 'available'
    ? value.value!
    : { kind: value.source.status, reason: value.source.reason };
}

function presentationSprintStatus(
  value: NonNullable<ProductReadModelsV1['epics'][number]['sprints'][number]['lifecycle']>,
): SprintPlanItemPresentation['status'] {
  return value.source.status === 'available'
    ? value.value!
    : { kind: value.source.status, reason: value.source.reason };
}

function unavailableSprintStatus(): SprintPlanItemPresentation['status'] {
  return {
    kind: 'unavailable',
    reason: 'Sprint lifecycle status is not available from the product read.',
  };
}

function presentationContinuation(
  continuation: ProductReadModelsV1['epics'][number]['continuation'],
  epicId?: string,
) {
  if (!continuation.policy) return {};
  return {
    continuation: {
      automaticEnabled: continuation.policy.automaticEnabled,
      eligible: continuation.eligibility?.status === 'eligible',
      status: continuation.initiationObserved
        ? ('continuation_requested' as const)
        : continuation.eligibility?.status === 'eligible'
          ? ('ready_for_manual' as const)
          : ('not_ready' as const),
      ...(epicId
        ? {
            policyUpdateIntent: {
              level: 'epic' as const,
              epicId,
              policyId: continuation.policy.policyId,
              automaticEnabled: continuation.policy.automaticEnabled,
            },
          }
        : {}),
    },
  };
}
