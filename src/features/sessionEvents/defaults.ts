import type { ReferenceIdentity, SessionLogicalAddress } from './types';

export function emptyReference(kind = 'reference'): ReferenceIdentity {
  return { namespace: 'application', kind, id: '' };
}

export function emptyLogicalAddress(): SessionLogicalAddress {
  return {
    scope: emptyReference('scope'),
    subject: emptyReference('subject'),
  };
}
