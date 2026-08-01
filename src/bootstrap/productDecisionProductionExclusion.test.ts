import { createProductApplicationComposition } from './productApplicationComposition';

describe('product decision production exclusion', () => {
  it('does not inject the recorded Epic product decision adapter into product boot', () => {
    expect(createProductApplicationComposition()).not.toHaveProperty('epicProductDecisionSource');
  });
});
