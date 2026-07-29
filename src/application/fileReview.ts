export type FileReviewSourceKind =
  'working_tree' | 'staged' | 'commit_range' | 'generated_material' | 'application_owned';

export interface FileReviewSourceSummary {
  /** Opaque adapter-owned identifier. Presentation must not derive repository access from it. */
  readonly sourceId: string;
  readonly kind: FileReviewSourceKind;
  readonly label: string;
  readonly detail: string;
  /** Optional comparison meaning supplied by the source adapter. */
  readonly comparisonLabel?: string;
}

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
  readonly source: FileReviewSourceSummary;
  readonly files: readonly FileReviewFile[];
}

/**
 * Read-only application boundary. Repository/worktree lookup, path authorization, and source
 * collection stay behind this port; the viewer receives display-ready facts only.
 */
export interface FileReviewClient {
  listSources(): Promise<readonly FileReviewSourceSummary[]>;
  loadSource(sourceId: string): Promise<FileReviewSnapshot>;
}
