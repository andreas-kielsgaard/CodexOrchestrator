import { assertCompleteFileReviewFile, type FileReviewFile } from '../../application/fileReview';
import {
  createRecordedFileReviewSource,
  recordedFileReviewFixtures,
  type RecordedFileReviewFixtureId,
} from './recordedFileReviewClient';

const expectedTotals: Record<
  RecordedFileReviewFixtureId,
  { readonly files: number; readonly additions: number; readonly deletions: number }
> = {
  'working-tree': { files: 5, additions: 12, deletions: 4 },
  staged: { files: 2, additions: 5, deletions: 1 },
  'commit-range': { files: 2, additions: 2, deletions: 1 },
  generated: { files: 1, additions: 3, deletions: 0 },
  'application-owned': { files: 1, additions: 2, deletions: 1 },
};

describe('recorded file-review fixtures', () => {
  it('keeps every text and Markdown fixture complete and internally coherent', async () => {
    for (const { fixtureId } of recordedFileReviewFixtures) {
      const snapshot = await createRecordedFileReviewSource(fixtureId).load();
      const totals = snapshot.files.reduce(
        (sum, file) => ({
          additions: sum.additions + file.additions,
          deletions: sum.deletions + file.deletions,
        }),
        { additions: 0, deletions: 0 },
      );

      expect({ files: snapshot.files.length, ...totals }, fixtureId).toEqual(
        expectedTotals[fixtureId],
      );
      for (const file of snapshot.files) {
        if (file.content.kind === 'text' || file.content.kind === 'markdown')
          expect(
            () => assertCompleteFileReviewFile(file),
            `${fixtureId}: ${file.displayPath}`,
          ).not.toThrow();
      }
    }
  });

  it('detects mismatched counts, hunk headers, line numbers, and complete file content', async () => {
    const snapshot = await createRecordedFileReviewSource('working-tree').load();
    const file = snapshot.files.find(({ fileId }) => fileId === 'file-review-screen');
    expect(file?.content.kind).toBe('text');
    if (!file || file.content.kind !== 'text') throw new Error('Expected the added text fixture.');

    expectInvariantFailure({ ...file, additions: file.additions + 1 }, /addition count/i);
    expectInvariantFailure(
      {
        ...file,
        hunks: [{ ...file.hunks[0], header: '@@ -0,0 +1,8 @@' }],
      },
      /new header count/i,
    );
    expectInvariantFailure(
      {
        ...file,
        hunks: [
          {
            ...file.hunks[0],
            lines: [
              { ...file.hunks[0].lines[0], newLineNumber: 2 },
              ...file.hunks[0].lines.slice(1),
            ],
          },
        ],
      },
      /new line number/i,
    );
    expectInvariantFailure(
      { ...file, content: { ...file.content, text: `${file.content.text}// unrelated\n` } },
      /complete file content/i,
    );
  });
});

function expectInvariantFailure(file: FileReviewFile, message: RegExp) {
  expect(() => assertCompleteFileReviewFile(file)).toThrow(message);
}
