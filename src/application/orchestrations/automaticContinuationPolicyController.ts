/** A policy update is distinct from eligibility evaluation and continuation commands. */
export type AutomaticContinuationPolicyUpdateIntent =
  | {
      readonly level: 'sprint';
      readonly sprintId: string;
      readonly policyId: string;
      readonly automaticEnabled: boolean;
    }
  | {
      readonly level: 'epic';
      readonly epicId: string;
      readonly policyId: string;
      readonly automaticEnabled: boolean;
    };

export type AutomaticContinuationPolicyUpdateOutcome =
  | { readonly status: 'unsupported' | 'failed'; readonly message: string }
  | { readonly status: 'policy_update_recorded'; readonly message: string };

export interface SprintAutomaticContinuationPolicyController {
  updatePolicy(
    intent: Extract<AutomaticContinuationPolicyUpdateIntent, { readonly level: 'sprint' }>,
  ): Promise<AutomaticContinuationPolicyUpdateOutcome>;
}

export interface EpicAutomaticContinuationPolicyController {
  updatePolicy(
    intent: Extract<AutomaticContinuationPolicyUpdateIntent, { readonly level: 'epic' }>,
  ): Promise<AutomaticContinuationPolicyUpdateOutcome>;
}

/** Honest product boundary until durable policy storage and refresh are connected. */
export const unsupportedProductSprintAutomaticContinuationPolicyController: SprintAutomaticContinuationPolicyController =
  {
    async updatePolicy() {
      return {
        status: 'unsupported',
        message:
          'Sprint automatic-continuation policy updates are not connected to durable storage.',
      };
    },
  };

/** Honest product boundary until durable policy storage and refresh are connected. */
export const unsupportedProductEpicAutomaticContinuationPolicyController: EpicAutomaticContinuationPolicyController =
  {
    async updatePolicy() {
      return {
        status: 'unsupported',
        message: 'Epic automatic-continuation policy updates are not connected to durable storage.',
      };
    },
  };
