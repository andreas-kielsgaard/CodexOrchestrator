declare const harnessIdBrand: unique symbol;
declare const harnessVersionNumberBrand: unique symbol;

/** Stable product identity for a Harness, independent of its display name and versions. */
export type HarnessId = string & { readonly [harnessIdBrand]: 'HarnessId' };

/** Monotonic, user-visible version number within one Harness. */
export type HarnessVersionNumber = number & {
  readonly [harnessVersionNumberBrand]: 'HarnessVersionNumber';
};

/** Exact reference to one immutable Harness configuration. */
export interface HarnessVersionRef {
  readonly harnessId: HarnessId;
  readonly version: HarnessVersionNumber;
}

export function createHarnessId(value: string): HarnessId {
  if (value.length === 0 || value.trim() !== value) {
    throw new Error('Harness ID must be a non-empty string without surrounding whitespace.');
  }
  return value as HarnessId;
}

export function createHarnessVersionNumber(value: number): HarnessVersionNumber {
  if (!Number.isSafeInteger(value) || value < 1) {
    throw new Error('Harness version must be a positive safe integer.');
  }
  return value as HarnessVersionNumber;
}

export function createHarnessVersionRef(
  harnessId: HarnessId,
  version: HarnessVersionNumber,
): HarnessVersionRef {
  return { harnessId, version };
}

export function harnessVersionRefsEqual(
  left: HarnessVersionRef,
  right: HarnessVersionRef,
): boolean {
  return left.harnessId === right.harnessId && left.version === right.version;
}
