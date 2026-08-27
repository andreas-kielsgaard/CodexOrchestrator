import type { HarnessConfiguration } from './configuration';
import type { HarnessId, HarnessVersionRef } from './references';

/** One Harness independent of its immutable versions and single editable draft. */
export interface HarnessCatalogItem {
  readonly harnessId: HarnessId;
  readonly name: string;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export type HarnessVersionScope =
  { readonly kind: 'reusable' } | { readonly kind: 'session_specific'; readonly sessionId: string };

export interface PublishedHarnessVersion {
  readonly reference: HarnessVersionRef;
  readonly scope: HarnessVersionScope;
  readonly configuration: HarnessConfiguration;
  readonly createdAt: string;
}

/** The one persistent draft owned by a Harness. Storage concurrency evidence is not product state. */
export interface HarnessDraft {
  readonly harnessId: HarnessId;
  readonly basedOn: HarnessVersionRef | null;
  readonly configuration: HarnessConfiguration;
  readonly savedAt: string;
}

export interface HarnessDetails {
  readonly harness: HarnessCatalogItem;
  readonly draft: HarnessDraft | null;
  readonly versions: readonly PublishedHarnessVersion[];
  readonly replacements: readonly HarnessVersionReplacement[];
}

export interface HarnessVersionReplacement {
  readonly source: HarnessVersionRef;
  readonly target: HarnessVersionRef;
}

export interface ResolvedHarnessVersion {
  readonly requested: HarnessVersionRef;
  readonly version: PublishedHarnessVersion;
  /** Requested reference followed by each replacement target, including the resolved reference. */
  readonly replacementPath: readonly HarnessVersionRef[];
}

export interface LoadHarnessInput {
  readonly harnessId: HarnessId;
}

export interface CreateHarnessInput {
  readonly name: string;
  readonly initialConfiguration: HarnessConfiguration;
}

export interface RenameHarnessInput {
  readonly harnessId: HarnessId;
  readonly name: string;
}

export interface SaveHarnessDraftInput {
  readonly harnessId: HarnessId;
  readonly basedOn: HarnessVersionRef | null;
  readonly configuration: HarnessConfiguration;
}

export interface PublishHarnessDraftInput {
  readonly harnessId: HarnessId;
}

export interface PublishSessionHarnessOverrideInput {
  readonly harnessId: HarnessId;
  readonly sessionId: string;
  readonly baseHarnessRef: HarnessVersionRef;
  readonly configuration: HarnessConfiguration;
}

export interface OrderHarnessVersionReplacementInput {
  readonly source: HarnessVersionRef;
  readonly target: HarnessVersionRef;
}

export interface ResolveHarnessVersionInput {
  readonly requested: HarnessVersionRef;
}

/** Product-facing Harness catalog and command boundary. */
export interface HarnessManagementClient {
  list(): Promise<readonly HarnessCatalogItem[]>;
  load(input: LoadHarnessInput): Promise<HarnessDetails>;
  create(input: CreateHarnessInput): Promise<HarnessCatalogItem>;
  rename(input: RenameHarnessInput): Promise<HarnessCatalogItem>;
  saveDraft(input: SaveHarnessDraftInput): Promise<HarnessDraft>;
  publishDraft(input: PublishHarnessDraftInput): Promise<PublishedHarnessVersion>;
  publishSessionOverride(
    input: PublishSessionHarnessOverrideInput,
  ): Promise<PublishedHarnessVersion>;
  orderReplacement(input: OrderHarnessVersionReplacementInput): Promise<HarnessVersionReplacement>;
  resolveVersion(input: ResolveHarnessVersionInput): Promise<ResolvedHarnessVersion>;
}
