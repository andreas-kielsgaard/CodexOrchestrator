import { describe, expect, it } from 'vitest';
import {
  createProductNavigation,
  isProductNavigationDestination,
  productNavigationReducer,
  sameProductNavigationDestination,
} from '../productNavigation';

describe('Coordinator navigation', () => {
  it('keeps list, setup, and device destinations in Back/Forward history', () => {
    const list = { kind: 'runtime_coordinator', location: { view: 'list' } } as const;
    const setup = { kind: 'runtime_coordinator', location: { view: 'setup' } } as const;
    const detail = {
      kind: 'runtime_coordinator',
      location: { view: 'device', deviceId: 'device-1' },
    } as const;
    let state = createProductNavigation(list);
    state = productNavigationReducer(state, {
      type: 'navigate',
      intent: 'push',
      destination: setup,
    });
    state = productNavigationReducer(state, {
      type: 'navigate',
      intent: 'push',
      destination: detail,
    });
    state = productNavigationReducer(state, { type: 'back' });
    expect(state.current.destination).toEqual(setup);
    state = productNavigationReducer(state, { type: 'forward' });
    expect(state.current.destination).toEqual(detail);
    expect(sameProductNavigationDestination(list, setup)).toBe(false);
    expect(isProductNavigationDestination(detail)).toBe(true);
    expect(
      isProductNavigationDestination({ ...setup, location: { view: 'setup', token: 'secret' } }),
    ).toBe(false);
  });
});
