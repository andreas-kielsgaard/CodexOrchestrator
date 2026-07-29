export type FileReviewChangeKind = 'added' | 'modified' | 'deleted' | 'renamed';

export type FileReviewContent =
  | { readonly kind: 'markdown'; readonly text: string; readonly language?: string }
  | { readonly kind: 'text'; readonly text: string; readonly language?: string }
  | { readonly kind: 'binary'; readonly reason: string }
  | { readonly kind: 'unsupported'; readonly reason: string };

export interface FileReviewDiffLine {
  readonly kind: 'context' | 'addition' | 'deletion';
  readonly oldLineNumber?: number;
  readonly newLineNumber?: number;
  readonly text: string;
}

export interface FileReviewDiffHunk {
  readonly hunkId: string;
  readonly header: string;
  readonly lines: readonly FileReviewDiffLine[];
}

export interface FileReviewFile {
  readonly fileId: string;
  /** Display-safe relative path, never filesystem authority. */
  readonly displayPath: string;
  readonly previousDisplayPath?: string;
  readonly changeKind: FileReviewChangeKind;
  readonly additions: number;
  readonly deletions: number;
  readonly provenance?: readonly ('committed-divergence' | 'uncommitted')[];
  readonly content: FileReviewContent;
  readonly hunks: readonly FileReviewDiffHunk[];
}

export interface FileReviewSnapshot {
  readonly files: readonly FileReviewFile[];
}

/** One already-scoped application-owned review source. The viewer cannot choose repository scope. */
export interface FileReviewSource {
  load(): Promise<FileReviewSnapshot>;
}

/** Complete text facts must keep counts, line numbers, and full-file content coherent. */
export function assertCompleteFileReviewFile(file: FileReviewFile): void {
  if (file.content.kind !== 'text' && file.content.kind !== 'markdown') return;
  if (file.hunks.length !== 1) throw new Error('A complete text file requires one full-file hunk.');
  const hunk = file.hunks[0];
  const match = /^@@ -(\d+),(\d+) \+(\d+),(\d+) @@$/.exec(hunk.header);
  if (!match) throw new Error('Invalid complete hunk header.');
  const oldCount = Number(match[2]);
  const newCount = Number(match[4]);
  const additions = hunk.lines.filter(({ kind }) => kind === 'addition').length;
  const deletions = hunk.lines.filter(({ kind }) => kind === 'deletion').length;
  if (additions !== file.additions || deletions !== file.deletions)
    throw new Error('File counts do not match its diff lines.');
  const oldLines = hunk.lines.filter(({ oldLineNumber }) => oldLineNumber !== undefined);
  const newLines = hunk.lines.filter(({ newLineNumber }) => newLineNumber !== undefined);
  if (oldLines.length !== oldCount || newLines.length !== newCount)
    throw new Error('Hunk counts do not match complete side lines.');
  oldLines.forEach((line, index) => {
    if (line.oldLineNumber !== index + 1) throw new Error('Old line sequence is incomplete.');
  });
  newLines.forEach((line, index) => {
    if (line.newLineNumber !== index + 1) throw new Error('New line sequence is incomplete.');
  });
  const content = newLines.map(({ text }) => text).join('\n');
  const expected = file.content.text.endsWith('\n')
    ? file.content.text.slice(0, -1)
    : file.content.text;
  if (content !== expected) throw new Error('Full-file content does not match Changes.');
}
