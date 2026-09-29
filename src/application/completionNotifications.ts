export type CompletionNotification =
  | { readonly kind: 'agent_turn'; readonly id: string }
  | { readonly kind: 'worktree_review_build'; readonly id: string };

export interface CompletionNotificationSource {
  subscribe(listener: (notification: CompletionNotification) => void): Promise<() => void>;
}

export interface CompletionSoundPlayer {
  unlock(): void;
  play(): void;
}
