import type {
  FileReviewClient,
  FileReviewDiffLine,
  FileReviewFile,
  FileReviewSnapshot,
  FileReviewSourceSummary,
} from '../../application/fileReview';
import type { AppProps } from '../../app/App';
import { createRecordedDevelopmentApplicationComposition } from '../orchestrationSection/recordedOrchestrationClient';

interface RecordedSource {
  /** Adapter-private locator facts are intentionally excluded from the returned snapshot. */
  readonly locator: {
    readonly repository: string;
    readonly worktree?: string;
    readonly revision?: string;
  };
  readonly snapshot: FileReviewSnapshot;
}

const workingTreeSource = source(
  'source-working-tree',
  'working_tree',
  'Working tree changes',
  'Uncommitted changes · review worktree',
);
const stagedSource = source(
  'source-staged',
  'staged',
  'Staged changes',
  'Index snapshot · ready for commit review',
);
const commitRangeSource = source(
  'source-commit-range',
  'commit_range',
  'Commit range',
  'main…exploration · three commits',
);
const generatedSource = source(
  'source-generated',
  'generated_material',
  'Generated material',
  'Bootstrap preview · not persisted',
);
const applicationOwnedSource = source(
  'source-application-owned',
  'application_owned',
  'Application-owned record',
  'Accepted review note · durable product record',
);

const recordedSources: readonly RecordedSource[] = [
  {
    locator: {
      repository: 'repository:codex-orchestrator',
      worktree: 'worktree:exploration-review',
    },
    snapshot: {
      source: workingTreeSource,
      files: [
        {
          fileId: 'file-review-notes',
          displayPath: 'docs/orchestration/file-diff-viewer-exploration.md',
          changeKind: 'modified',
          additions: 18,
          deletions: 6,
          content: {
            kind: 'markdown',
            text: `# A review surface inside the product

The viewer presents **display-ready facts** from an injected client. It does not resolve a repository, authorize a path, or edit content.

## Review questions

- Is the changed-file list dense enough?
- Should unified or split view be the default?
- Is generated material clearly distinct from persisted material?

| Boundary | Owner |
| --- | --- |
| Repository and worktree selection | Application adapter |
| Markdown and diff presentation | Viewer |
| Writes and edits | Outside this exploration |

> Source labels describe provenance; they do not grant access.

\`\`\`ts
interface FileReviewClient {
  loadSource(sourceId: string): Promise<FileReviewSnapshot>;
}
\`\`\`

<button>Untrusted HTML action</button>
`,
          },
          hunks: [
            {
              hunkId: 'review-notes-contract',
              header: '@@ -8,8 +8,12 @@ Neutral contracts',
              collapsedBefore: [
                contextLine(5, 'The application chooses an authorized source.'),
                contextLine(6, 'Repository identity remains inside the adapter.'),
                contextLine(7, 'The viewer receives display-safe relative paths.'),
              ],
              lines: [
                contextLine(8, '## Contracts'),
                deletedLine(9, 'The screen reads files from the active worktree.'),
                addedLine(9, 'The screen requests an opaque review source.'),
                addedLine(10, 'The adapter returns display-ready file facts.'),
                contextLine(11, ''),
                contextLine(12, '## Review points'),
                deletedLine(13, '- Confirm the basic diff layout.'),
                addedLine(13, '- Confirm the changed-file information density.'),
                addedLine(14, '- Choose the default unified or split layout.'),
              ],
              collapsedAfter: [
                contextLine(15, '- Confirm how generated material is labeled.'),
                contextLine(16, ''),
              ],
            },
          ],
        },
        {
          fileId: 'file-review-screen',
          displayPath: 'src/features/fileReview/FileReviewScreen.tsx',
          changeKind: 'added',
          additions: 42,
          deletions: 0,
          content: {
            kind: 'text',
            language: 'tsx',
            text: `export function FileReviewScreen({ client }: FileReviewScreenProps) {
  const [sourceId, setSourceId] = useState('');
  const [layout, setLayout] = useState<'unified' | 'split'>('unified');

  return <main aria-label="Files and diffs">{/* read-only presentation */}</main>;
}
`,
          },
          hunks: [
            {
              hunkId: 'new-screen',
              header: '@@ -0,0 +1,8 @@',
              lines: [
                addedLine(1, "import { useState } from 'react';"),
                addedLine(2, ''),
                addedLine(3, 'export function FileReviewScreen() {'),
                addedLine(4, "  const [layout, setLayout] = useState('unified');"),
                addedLine(5, ''),
                addedLine(6, '  return <main>Read-only review</main>;'),
                addedLine(7, '}'),
              ],
            },
          ],
        },
        {
          fileId: 'file-review-renamed',
          displayPath: 'src/application/fileReview.ts',
          previousDisplayPath: 'src/application/diffViewer.ts',
          changeKind: 'renamed',
          additions: 24,
          deletions: 9,
          content: {
            kind: 'text',
            language: 'ts',
            text: `export interface FileReviewClient {
  listSources(): Promise<readonly FileReviewSourceSummary[]>;
  loadSource(sourceId: string): Promise<FileReviewSnapshot>;
}
`,
          },
          hunks: [
            {
              hunkId: 'renamed-contract',
              header: '@@ -1,5 +1,8 @@',
              lines: [
                deletedLine(1, 'export interface DiffViewer {'),
                deletedLine(2, '  read(path: string): Promise<string>;'),
                addedLine(1, 'export interface FileReviewClient {'),
                addedLine(2, '  listSources(): Promise<readonly FileReviewSourceSummary[]>;'),
                addedLine(3, '  loadSource(sourceId: string): Promise<FileReviewSnapshot>;'),
                contextLine(4, '}'),
              ],
            },
          ],
        },
        {
          fileId: 'file-review-binary',
          displayPath: 'docs/review/file-review-walkthrough.mp4',
          changeKind: 'modified',
          additions: 0,
          deletions: 0,
          content: {
            kind: 'binary',
            reason:
              'The adapter identified this file as binary. Playback and byte-level comparison are outside the viewer.',
          },
          hunks: [],
        },
        {
          fileId: 'file-review-unsupported',
          displayPath: 'docs/review/layout-study.sketch',
          changeKind: 'deleted',
          additions: 0,
          deletions: 1,
          content: {
            kind: 'unsupported',
            reason:
              'This file type has no safe text or Markdown renderer. The deletion remains visible in the changed-file list.',
          },
          hunks: [],
        },
      ],
    },
  },
  {
    locator: {
      repository: 'repository:codex-orchestrator',
      worktree: 'worktree:exploration-review',
      revision: 'index',
    },
    snapshot: {
      source: stagedSource,
      files: [
        textFile(
          'staged-contract',
          'src/application/fileReview.ts',
          'modified',
          12,
          2,
          'The staged snapshot is collected independently from working-tree changes.',
        ),
        markdownFile(
          'staged-note',
          'docs/orchestration/review-checklist.md',
          'added',
          9,
          0,
          '# Staged review checklist\n\n- Contracts are neutral\n- No write controls are present\n',
        ),
      ],
    },
  },
  {
    locator: {
      repository: 'repository:codex-orchestrator',
      revision: 'range:main...codex/explore-file-diff-viewer',
    },
    snapshot: {
      source: commitRangeSource,
      files: [
        markdownFile(
          'range-summary',
          'docs/orchestration/exploration-summary.md',
          'modified',
          31,
          8,
          '# Commit range review\n\nThree commits contribute to this bounded exploration.\n',
        ),
        textFile(
          'range-test',
          'src/features/fileReview/FileReviewScreen.test.tsx',
          'added',
          48,
          0,
          'Commit-range facts use the same presentation contract.',
        ),
      ],
    },
  },
  {
    locator: {
      repository: 'repository:codex-orchestrator',
      revision: 'generated:epic-bootstrap-preview',
    },
    snapshot: {
      source: generatedSource,
      files: [
        markdownFile(
          'generated-bootstrap',
          'preview/epic-bootstrap.md',
          'added',
          22,
          0,
          '# Generated Epic bootstrap\n\nThis material is a preview and has not been persisted.\n',
        ),
      ],
    },
  },
  {
    locator: {
      repository: 'product-records:orchestration',
      revision: 'record:review-note-017',
    },
    snapshot: {
      source: applicationOwnedSource,
      files: [
        markdownFile(
          'application-review-note',
          'review-notes/file-viewer.md',
          'modified',
          6,
          1,
          '# Accepted review note\n\nThis content is application-owned, not a repository file.\n',
        ),
      ],
    },
  },
];

export const recordedFileReviewClient: FileReviewClient = {
  async listSources() {
    return recordedSources.map(({ snapshot }) => structuredClone(snapshot.source));
  },
  async loadSource(sourceId) {
    const recorded = recordedSources.find(({ snapshot }) => snapshot.source.sourceId === sourceId);
    if (!recorded) throw new Error(`Unknown recorded file-review source: ${sourceId}`);
    return structuredClone(recorded.snapshot);
  },
};

/** Development-only tab composition. Product boot does not receive this client or surface. */
export function createRecordedFileReviewApplicationComposition(): AppProps {
  return {
    ...createRecordedDevelopmentApplicationComposition(),
    fileReviewClient: recordedFileReviewClient,
    initialSurface: 'file-review',
  };
}

function source(
  sourceId: string,
  kind: FileReviewSourceSummary['kind'],
  label: string,
  detail: string,
): FileReviewSourceSummary {
  return { sourceId, kind, label, detail };
}

function contextLine(lineNumber: number, text: string): FileReviewDiffLine {
  return {
    kind: 'context',
    oldLineNumber: lineNumber,
    newLineNumber: lineNumber,
    text,
  };
}

function addedLine(lineNumber: number, text: string): FileReviewDiffLine {
  return { kind: 'addition', newLineNumber: lineNumber, text };
}

function deletedLine(lineNumber: number, text: string): FileReviewDiffLine {
  return { kind: 'deletion', oldLineNumber: lineNumber, text };
}

function textFile(
  fileId: string,
  displayPath: string,
  changeKind: FileReviewFile['changeKind'],
  additions: number,
  deletions: number,
  text: string,
): FileReviewFile {
  return {
    fileId,
    displayPath,
    changeKind,
    additions,
    deletions,
    content: { kind: 'text', text },
    hunks: [
      {
        hunkId: `${fileId}-hunk`,
        header: '@@ -1 +1 @@',
        lines: [deletedLine(1, 'Previous review material.'), addedLine(1, text)],
      },
    ],
  };
}

function markdownFile(
  fileId: string,
  displayPath: string,
  changeKind: FileReviewFile['changeKind'],
  additions: number,
  deletions: number,
  text: string,
): FileReviewFile {
  return {
    ...textFile(fileId, displayPath, changeKind, additions, deletions, text),
    content: { kind: 'markdown', text },
  };
}
