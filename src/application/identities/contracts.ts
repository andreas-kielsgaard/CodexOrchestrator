/** Stable application identity for a reusable identity definition. */
export type IdentityId = string;

export type IdentityShape = 'circle' | 'square' | 'hexagon';

/** Reusable identity offered to Session creation and identity pickers. */
export interface IdentityDefinition {
  readonly id: IdentityId;
  readonly displayName: string;
  readonly color: string;
  readonly shape: IdentityShape;
}

/** Session-owned copy of an identity. It does not follow later definition changes. */
export interface AssignedAgentIdentity {
  readonly originIdentityId: IdentityId | null;
  readonly displayName: string;
  readonly color: string;
  readonly shape: IdentityShape;
}
