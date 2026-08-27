import { identityDefinitionFromCatalog } from './management';

describe('Identity management contracts', () => {
  it('projects persistence metadata out of the reusable domain definition', () => {
    expect(
      identityDefinitionFromCatalog({
        id: 'identity-avery',
        displayName: 'Avery',
        color: '#39745a',
        shape: 'circle',
        createdAt: '2026-08-27T09:00:00Z',
        updatedAt: '2026-08-27T10:00:00Z',
      }),
    ).toEqual({
      id: 'identity-avery',
      displayName: 'Avery',
      color: '#39745a',
      shape: 'circle',
    });
  });
});
