import { invoke } from '@tauri-apps/api/core';
import { createApplicationOwnedFileReviewSource } from '../../application/applicationOwnedFileReview';
import type {
  ContextualFileReviewClient,
  ContextualFileReviewFailureReason,
  ContextualFileReviewResult,
} from '../../application/contextualFileReview';
import {
  createTauriScopedFileReviewPorts,
  type ScopedFileReviewResponse,
} from './tauriScopedFileReview';

type ContextualResponse =
  | {
      readonly status: 'available';
      readonly opaqueReference: string;
      readonly idempotentReplay: boolean;
    }
  | {
      readonly status: 'unavailable';
      readonly reason: ContextualFileReviewFailureReason;
    };

export type ContextualFileReviewInvoke = (
  command: 'request_contextual_file_review' | 'load_scoped_file_review',
  args: { readonly input: Record<string, string> },
) => Promise<unknown>;

const tauriInvoke: ContextualFileReviewInvoke = (command, args) => invoke(command, args);

export function createTauriContextualFileReviewClient(
  invokeCommand: ContextualFileReviewInvoke = tauriInvoke,
): ContextualFileReviewClient {
  return {
    async requestForSprint(sprintId): Promise<ContextualFileReviewResult> {
      try {
        const response = decodeResponse(
          await invokeCommand('request_contextual_file_review', {
            input: { sprintId },
          }),
        );
        if (response.status !== 'available') return failure(response.reason);

        const ports = createTauriScopedFileReviewPorts(
          response.opaqueReference,
          (command, args) => invokeCommand(command, args) as Promise<ScopedFileReviewResponse>,
        );
        const source = createApplicationOwnedFileReviewSource(ports.documents, ports.artifacts);
        let initialSnapshot: Awaited<ReturnType<typeof source.load>> | undefined =
          await source.load();
        return {
          status: 'ready',
          idempotentReplay: response.idempotentReplay,
          source: {
            load() {
              if (initialSnapshot) {
                const loaded = initialSnapshot;
                initialSnapshot = undefined;
                return Promise.resolve(loaded);
              }
              return source.load();
            },
          },
        };
      } catch {
        return failure('unavailable');
      }
    },
  };
}

function decodeResponse(value: unknown): ContextualResponse {
  if (!value || typeof value !== 'object') throw new Error('invalid contextual response');
  const response = value as Record<string, unknown>;
  if (response.status === 'available') {
    if (
      typeof response.opaqueReference !== 'string' ||
      !response.opaqueReference.trim() ||
      typeof response.idempotentReplay !== 'boolean'
    )
      throw new Error('invalid contextual response');
    return {
      status: 'available',
      opaqueReference: response.opaqueReference,
      idempotentReplay: response.idempotentReplay,
    };
  }
  if (response.status === 'unavailable' && isFailureReason(response.reason))
    return { status: 'unavailable', reason: response.reason };
  throw new Error('invalid contextual response');
}

function isFailureReason(value: unknown): value is ContextualFileReviewFailureReason {
  return (
    value === 'not_ready' ||
    value === 'source_not_ready' ||
    value === 'conflict' ||
    value === 'source_unavailable' ||
    value === 'unavailable'
  );
}

function failure(reason: ContextualFileReviewFailureReason): ContextualFileReviewResult {
  const message =
    reason === 'not_ready'
      ? 'File Review is not ready for this Sprint.'
      : reason === 'source_not_ready'
        ? 'The Sprint source is not ready for File Review.'
        : reason === 'conflict'
          ? 'File Review could not confirm one current Sprint source.'
          : reason === 'source_unavailable'
            ? 'The produced File Review could not be loaded.'
            : 'File Review is unavailable right now.';
  return { status: 'failed', reason, message };
}
