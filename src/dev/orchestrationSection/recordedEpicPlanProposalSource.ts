import type {
  EpicPlanProposalSnapshot,
  EpicPlanProposalSource,
} from '../../application/orchestrations';

export interface MutableRecordedEpicPlanProposalSource extends EpicPlanProposalSource {
  setSnapshot(snapshot: EpicPlanProposalSnapshot): void;
}

/** Development/test adapter only; product code receives the read-only source contract. */
export function createMutableRecordedEpicPlanProposalSource(
  initial: EpicPlanProposalSnapshot,
): MutableRecordedEpicPlanProposalSource {
  let snapshot = initial;
  const listeners = new Set<() => void>();
  return {
    getSnapshot: () => snapshot,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    refresh: async () => undefined,
    setSnapshot(next) {
      snapshot = next;
      for (const listener of listeners) listener();
    },
  };
}

/** Recorded/local evaluation input only. It is not derived from Agent Session prose or product reads. */
export const recordedLocalEpicPlanProposalSource = createMutableRecordedEpicPlanProposalSource({
  kind: 'available',
  sprints: [
    {
      title: 'Plan Builder foundation',
      intendedMovement:
        'Establish a reviewable planning conversation and compact proposal readout.',
      concernSummaries: [
        'Keep proposed planning separate from accepted orchestration facts.',
        'Make source availability explicit before a product proposal source exists.',
      ],
    },
    {
      title: 'State and integration proof',
      intendedMovement: 'Define the later boundary for durable state and product-tool integration.',
      concernSummaries: ['Do not infer provider acknowledgement from conversation text.'],
    },
  ],
});
