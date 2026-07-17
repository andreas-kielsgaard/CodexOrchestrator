export interface EpicPlanningDraftSummary {
  readonly epicPlanningDraftId: string;
  readonly agentSessionId: string;
  readonly title?: string;
  readonly status: 'active' | 'canceled' | 'initiated';
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface EpicPlanningDraftBinding {
  readonly draftId: string;
  readonly sessionId: string;
  readonly title?: string;
}

export interface EpicPlanningDraftLifecycleClient {
  /** Binds the durable planning draft that the managed send created for this Agent Session. */
  reconcile(sessionId: string, title?: string): Promise<EpicPlanningDraftBinding>;
  list(): Promise<readonly EpicPlanningDraftSummary[]>;
  updateTitle(binding: EpicPlanningDraftBinding, title: string): Promise<void>;
  cancel(binding: EpicPlanningDraftBinding): Promise<void>;
}
