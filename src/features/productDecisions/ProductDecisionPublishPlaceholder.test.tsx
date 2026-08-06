import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { ProductNavigationDestination } from '../../application/productNavigation';
import { ProductDecisionPublishPlaceholder } from './ProductDecisionPublishPlaceholder';

describe('ProductDecisionPublishPlaceholder', () => {
  it('states the no-effect boundary for direct typed entry', () => {
    const destination: Extract<
      ProductNavigationDestination,
      { readonly kind: 'product_decision_publish' }
    > = {
      kind: 'product_decision_publish',
      epicId: 'epic-1',
      decisionId: 'decision-1',
      versionId: 'version-2',
      version: 2,
    };
    render(<ProductDecisionPublishPlaceholder destination={destination} />);

    expect(
      screen.getByRole('main', { name: 'Product Decision Publish placeholder' }),
    ).toHaveAttribute('data-publish-effect', 'none');
    expect(screen.getByRole('heading', { name: 'Publish is not available yet' })).toBeVisible();
    expect(screen.getByText(/Future publishing will determine applicability/)).toBeVisible();
    expect(
      screen.getByText(/does not publish, apply, reconcile, invalidate, settle, audit/),
    ).toBeVisible();
  });
});
