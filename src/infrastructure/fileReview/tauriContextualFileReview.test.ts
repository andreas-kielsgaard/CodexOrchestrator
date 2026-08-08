import { createTauriContextualFileReviewClient } from './tauriContextualFileReview';

describe('contextual File Review client', () => {
  it('produces, reauthorizes, and preloads one scoped source before reporting ready', async () => {
    const calls: string[] = [];
    const client = createTauriContextualFileReviewClient(async (command) => {
      calls.push(command);
      if (command === 'request_contextual_file_review')
        return { status: 'available', opaqueReference: 'opaque', idempotentReplay: false };
      return {
        status: 'available',
        document: {
          documentRefId: 'document',
          title: 'Changed files',
          artifactId: 'artifact',
          payload: [...new TextEncoder().encode(JSON.stringify(payload))],
          changedFiles: [
            {
              changedFileReferenceId: 'file',
              displayName: 'src/a.ts',
              changeKind: 'modified',
            },
          ],
        },
      };
    });

    const result = await client.requestForSprint('sprint-1');

    expect(result.status).toBe('ready');
    expect(calls).toEqual([
      'request_contextual_file_review',
      'load_scoped_file_review',
      'load_scoped_file_review',
    ]);
    if (result.status === 'ready') {
      await expect(result.source.load()).resolves.toMatchObject({
        files: [{ displayPath: 'src/a.ts' }],
      });
      expect(calls).toHaveLength(3);
    }
  });

  it('keeps bounded pending-source and transport failures distinct', async () => {
    const notReady = createTauriContextualFileReviewClient(async () => ({
      status: 'unavailable',
      reason: 'source_not_ready',
    }));
    await expect(notReady.requestForSprint('sprint-1')).resolves.toEqual({
      status: 'failed',
      reason: 'source_not_ready',
      message: 'The Sprint source is not ready for File Review.',
    });

    const failed = createTauriContextualFileReviewClient(async () => {
      throw new Error('private detail');
    });
    await expect(failed.requestForSprint('sprint-1')).resolves.toEqual({
      status: 'failed',
      reason: 'unavailable',
      message: 'File Review is unavailable right now.',
    });

    const malformed = createTauriContextualFileReviewClient(async () => ({
      status: 'available',
      opaqueReference: 'opaque',
      idempotentReplay: 'yes',
    }));
    await expect(malformed.requestForSprint('sprint-1')).resolves.toMatchObject({
      status: 'failed',
      reason: 'unavailable',
    });
  });
});

const payload = {
  contractVersion: 'stored-file-review-artifact/v1',
  documentRefId: 'document',
  artifactId: 'artifact',
  files: [
    {
      changedFileReferenceId: 'file',
      content: {
        encoding: 'utf-8',
        bytesBase64: btoa('after\n'),
      },
      hunks: [
        {
          header: '@@ -1 +1 @@',
          lines: [
            { kind: 'deletion', oldLineNumber: 1, text: 'before' },
            { kind: 'addition', newLineNumber: 1, text: 'after' },
          ],
        },
      ],
    },
  ],
};
