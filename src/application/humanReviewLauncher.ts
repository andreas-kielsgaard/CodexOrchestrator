export interface HumanReviewSource {
  readonly sourceRef: string;
  readonly label: string;
  readonly revision: string;
}

export interface HumanReviewInstance {
  readonly instanceRef: string;
  readonly name: string;
  readonly sourceLabel: string;
  readonly phase: string;
  readonly health: string;
  readonly stale: boolean;
  readonly build: 'not-built' | 'passed' | 'failed';
  readonly canFocus: boolean;
}

export interface HumanReviewLauncherClient {
  listSources(): Promise<readonly HumanReviewSource[]>;
  listInstances(): Promise<readonly HumanReviewInstance[]>;
  prepare(sourceRef: string, name: string): Promise<HumanReviewInstance>;
  build(instanceRef: string): Promise<HumanReviewInstance>;
  start(instanceRef: string): Promise<HumanReviewInstance>;
  status(instanceRef: string): Promise<HumanReviewInstance>;
  focus(instanceRef: string): Promise<HumanReviewInstance>;
  stop(instanceRef: string): Promise<HumanReviewInstance>;
  recover(instanceRef: string): Promise<HumanReviewInstance>;
}
