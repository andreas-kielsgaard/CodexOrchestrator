export interface EpicOriginBranch {
  readonly name: string;
  readonly revision: string;
  readonly parentName?: string;
  readonly relationship: 'related' | 'unrelated';
  readonly ahead: number;
  readonly behind: number;
  readonly forkRevision: string;
  readonly isCurrent: boolean;
  readonly isBaseline: boolean;
}

export interface EpicOriginProject {
  readonly name: string;
  readonly path: string;
  readonly gitDetected: boolean;
  readonly repositoryRoot?: string;
  readonly branches: readonly EpicOriginBranch[];
}

export interface EpicOriginProjectClient {
  chooseProject(): Promise<EpicOriginProject | null>;
  inspectProject(path: string): Promise<EpicOriginProject>;
}
