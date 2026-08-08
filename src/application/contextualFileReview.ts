import type { FileReviewSource } from './fileReview';

export type ContextualFileReviewFailureReason =
  'not_ready' | 'source_not_ready' | 'conflict' | 'source_unavailable' | 'unavailable';

export type ContextualFileReviewResult =
  | {
      readonly status: 'ready';
      readonly source: FileReviewSource;
      readonly idempotentReplay: boolean;
    }
  | {
      readonly status: 'failed';
      readonly reason: ContextualFileReviewFailureReason;
      readonly message: string;
    };

export interface ContextualFileReviewClient {
  requestForSprint(sprintId: string): Promise<ContextualFileReviewResult>;
}
