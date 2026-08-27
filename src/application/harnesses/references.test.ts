import {
  createHarnessId,
  createHarnessVersionNumber,
  createHarnessVersionRef,
  harnessVersionRefsEqual,
} from './references';

describe('Harness references', () => {
  it('creates an exact Harness version reference', () => {
    const reference = createHarnessVersionRef(
      createHarnessId('harness-epic-plan-builder'),
      createHarnessVersionNumber(4),
    );

    expect(reference).toEqual({
      harnessId: 'harness-epic-plan-builder',
      version: 4,
    });
    expect(
      harnessVersionRefsEqual(
        reference,
        createHarnessVersionRef(
          createHarnessId('harness-epic-plan-builder'),
          createHarnessVersionNumber(4),
        ),
      ),
    ).toBe(true);
  });

  it('rejects ambiguous IDs and invalid versions', () => {
    expect(() => createHarnessId('')).toThrow(/non-empty/);
    expect(() => createHarnessId(' padded')).toThrow(/whitespace/);
    expect(() => createHarnessVersionNumber(0)).toThrow(/positive safe integer/);
    expect(() => createHarnessVersionNumber(1.5)).toThrow(/positive safe integer/);
  });

  it('distinguishes different Harnesses and versions', () => {
    const first = createHarnessVersionRef(
      createHarnessId('harness-one'),
      createHarnessVersionNumber(1),
    );

    expect(
      harnessVersionRefsEqual(
        first,
        createHarnessVersionRef(createHarnessId('harness-two'), createHarnessVersionNumber(1)),
      ),
    ).toBe(false);
    expect(
      harnessVersionRefsEqual(
        first,
        createHarnessVersionRef(createHarnessId('harness-one'), createHarnessVersionNumber(2)),
      ),
    ).toBe(false);
  });
});
