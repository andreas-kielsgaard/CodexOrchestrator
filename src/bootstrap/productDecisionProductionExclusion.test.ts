import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createProductApplicationComposition } from './productApplicationComposition';

describe('product decision production exclusion', () => {
  it('does not inject the recorded Epic product decision adapter into product boot', () => {
    expect(createProductApplicationComposition()).not.toHaveProperty('epicProductDecisionSource');
  });

  it('does not import the recorded development source from product boot', () => {
    const productBootSource = readFileSync(
      join(process.cwd(), 'src/bootstrap/productApplicationComposition.ts'),
      'utf8',
    );
    expect(productBootSource).not.toContain('recordedEpicProductDecisionSource');
    expect(productBootSource).not.toContain('../dev/productDecisions');
  });
});
