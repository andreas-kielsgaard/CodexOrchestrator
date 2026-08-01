export type FileReviewChangeKind = 'added' | 'modified' | 'deleted' | 'renamed';

export type FileReviewContent =
  | {
      readonly kind: 'markdown';
      readonly text: string;
      readonly language?: string;
    }
  | {
      readonly kind: 'text';
      readonly text: string;
      readonly language?: string;
    }
  | {
      readonly kind: 'binary';
      readonly reason: string;
    }
  | {
      readonly kind: 'unsupported';
      readonly reason: string;
    };

export type FileReviewDiffLineKind = 'context' | 'addition' | 'deletion';

export interface FileReviewDiffLine {
  readonly kind: FileReviewDiffLineKind;
  readonly oldLineNumber?: number;
  readonly newLineNumber?: number;
  readonly text: string;
}

export interface FileReviewDiffHunk {
  readonly hunkId: string;
  readonly header: string;
  readonly collapsedBefore?: readonly FileReviewDiffLine[];
  readonly lines: readonly FileReviewDiffLine[];
  readonly collapsedAfter?: readonly FileReviewDiffLine[];
}

export interface FileReviewFile {
  readonly fileId: string;
  /** Display-safe relative path supplied by the adapter, never a filesystem authority. */
  readonly displayPath: string;
  readonly previousDisplayPath?: string;
  readonly changeKind: FileReviewChangeKind;
  readonly additions: number;
  readonly deletions: number;
  readonly content: FileReviewContent;
  readonly hunks: readonly FileReviewDiffHunk[];
}

export interface FileReviewSnapshot {
  readonly files: readonly FileReviewFile[];
}

/** Text and Markdown review facts are complete; the contract has no truncated-hunk state. */
export function assertCompleteFileReviewFile(file: FileReviewFile): void {
  if (file.content.kind !== 'text' && file.content.kind !== 'markdown') return;
  if (file.hunks.length === 0) throw new Error('A complete text file requires diff hunks.');

  const allLines: FileReviewDiffLine[] = [];
  let additions = 0;
  let deletions = 0;
  for (const hunk of file.hunks) {
    const collapsedBefore = [...(hunk.collapsedBefore ?? [])];
    const collapsedAfter = [...(hunk.collapsedAfter ?? [])];
    if ([...collapsedBefore, ...collapsedAfter].some(({ kind }) => kind !== 'context'))
      throw new Error('Collapsed lines must be unchanged context.');
    const lines = [...collapsedBefore, ...hunk.lines, ...collapsedAfter];
    lines.forEach(assertLineNumbers);
    const header = parseHunkHeader(hunk.header);
    assertHunkSide(lines, 'old', header.oldStart, header.oldCount);
    assertHunkSide(lines, 'new', header.newStart, header.newCount);
    additions += lines.filter(({ kind }) => kind === 'addition').length;
    deletions += lines.filter(({ kind }) => kind === 'deletion').length;
    allLines.push(...lines);
  }

  if (additions !== file.additions)
    throw new Error(`Addition count ${file.additions} does not match ${additions}.`);
  if (deletions !== file.deletions)
    throw new Error(`Deletion count ${file.deletions} does not match ${deletions}.`);

  const oldNumbers = allLines.flatMap((line) =>
    line.oldLineNumber === undefined ? [] : [line.oldLineNumber],
  );
  const newLines = allLines.flatMap((line) =>
    line.newLineNumber === undefined ? [] : [{ number: line.newLineNumber, text: line.text }],
  );
  assertCompleteSequence(oldNumbers, 'old');
  assertCompleteSequence(
    newLines.map(({ number }) => number),
    'new',
  );
  if (newLines.map(({ text }) => text).join('\n') !== contentLines(file.content.text).join('\n'))
    throw new Error('Complete file content does not equal the added side of Changes.');
}

function parseHunkHeader(header: string) {
  const match = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@(?: .*)?$/.exec(header);
  if (!match) throw new Error(`Invalid hunk header: ${header}`);
  return {
    oldStart: Number(match[1]),
    oldCount: match[2] === undefined ? 1 : Number(match[2]),
    newStart: Number(match[3]),
    newCount: match[4] === undefined ? 1 : Number(match[4]),
  };
}

function assertHunkSide(
  lines: readonly FileReviewDiffLine[],
  side: 'old' | 'new',
  start: number,
  count: number,
) {
  if ((count === 0 && start !== 0) || (count > 0 && start < 1))
    throw new Error(`${side} hunk range starts at ${start} for count ${count}.`);
  const numbers = lines.flatMap((line) => {
    const number = side === 'old' ? line.oldLineNumber : line.newLineNumber;
    return number === undefined ? [] : [number];
  });
  if (numbers.length !== count)
    throw new Error(`${side} header count ${count} does not match ${numbers.length}.`);
  numbers.forEach((number, index) => {
    if (number !== start + index)
      throw new Error(`${side} line number ${number} does not follow ${start + index}.`);
  });
}

function assertLineNumbers(line: FileReviewDiffLine) {
  const hasOld = line.oldLineNumber !== undefined;
  const hasNew = line.newLineNumber !== undefined;
  if (
    (line.kind === 'context' && (!hasOld || !hasNew)) ||
    (line.kind === 'addition' && (hasOld || !hasNew)) ||
    (line.kind === 'deletion' && (!hasOld || hasNew))
  )
    throw new Error(`${line.kind} line carries inconsistent side line numbers.`);
}

function assertCompleteSequence(numbers: readonly number[], side: 'old' | 'new') {
  numbers.forEach((number, index) => {
    if (number !== index + 1)
      throw new Error(`Complete ${side} line number ${number} does not follow ${index + 1}.`);
  });
}

function contentLines(text: string) {
  if (!text) return [];
  return (text.endsWith('\n') ? text.slice(0, -1) : text).split('\n');
}

/**
 * One read-only, application-scoped review source. Selection, authorization, retrieval, and
 * normalization stay behind this port; the viewer receives display-ready facts only.
 */
export interface FileReviewSource {
  load(): Promise<FileReviewSnapshot>;
}
