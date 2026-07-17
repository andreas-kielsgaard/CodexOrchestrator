import type { EpicPlanningDraftLifecycleClient } from '../../application/orchestrations';
import type { OrchestrationNativeQueryClient } from './tauriOrchestrationNativeQuery';

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createTauriEpicPlanningDraftLifecycleClient(
  invoke: Invoke,
  nativeQuery: OrchestrationNativeQueryClient,
): EpicPlanningDraftLifecycleClient {
  return {
    async reconcile(sessionId, title) {
      return invoke('reconcile_managed_plan_builder_session', { input: { sessionId, title } });
    },
    async list() {
      const query = await nativeQuery.load();
      return query.planningDrafts.map((draft) => {
        const association = query.agentSessionAssociations.find(
          (item) => item.epicPlanningDraftId === draft.epicPlanningDraftId,
        );
        if (!association)
          throw new Error('Planning draft is missing its Agent Session association');
        return {
          epicPlanningDraftId: draft.epicPlanningDraftId,
          agentSessionId: association.agentSessionId,
          ...(draft.title ? { title: draft.title } : {}),
          status: draft.status,
          createdAt: draft.createdAt,
          updatedAt: draft.updatedAt,
        };
      });
    },
    async updateTitle(binding, title) {
      await invoke('update_epic_planning_draft_title', {
        input: {
          epicPlanningDraftId: binding.draftId,
          agentSessionId: binding.sessionId,
          title,
          idempotencyKey: `title:${binding.draftId}:${crypto.randomUUID()}`,
        },
      });
    },
    async cancel(binding) {
      await invoke('cancel_epic_planning_draft', {
        input: {
          epicPlanningDraftId: binding.draftId,
          agentSessionId: binding.sessionId,
          idempotencyKey: `cancel:${binding.draftId}`,
        },
      });
    },
  };
}
