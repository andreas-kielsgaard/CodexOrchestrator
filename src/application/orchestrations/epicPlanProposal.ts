/**
 * A disposable Plan Builder presentation source. It deliberately has no Epic, Sprint Plan, or
 * Work Unit identity because it represents a proposal before any durable orchestration fact exists.
 */
export type EpicPlanProposalSnapshot =
  | {
      readonly kind: 'available';
      readonly suggestedEpicName?: string;
      /** Durable revision evidence when the backing source can provide it. */
      readonly revision?: Readonly<{ id: string; recordedAt: string }>;
      readonly sprints: readonly {
        readonly title: string;
        readonly intendedMovement: string;
        readonly concernSummaries: readonly string[];
      }[];
    }
  | { readonly kind: 'unavailable'; readonly reason?: string };

/** An injected source may update the proposal; Agent Session transcript prose never does. */
export interface EpicPlanProposalSource {
  getSnapshot(): EpicPlanProposalSnapshot;
  subscribe(listener: () => void): () => void;
  /** Re-reads durable state; callers use this after mount/restart rather than trusting notifications. */
  refresh(): Promise<void>;
}

const unavailableEpicPlanProposalSnapshot: EpicPlanProposalSnapshot = { kind: 'unavailable' };

export const unavailableEpicPlanProposalSource: EpicPlanProposalSource = {
  getSnapshot: () => unavailableEpicPlanProposalSnapshot,
  subscribe: () => () => undefined,
  refresh: async () => undefined,
};
