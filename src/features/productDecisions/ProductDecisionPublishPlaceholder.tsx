import type { ProductNavigationDestination } from '../../application/productNavigation';

export function ProductDecisionPublishPlaceholder({
  destination,
}: {
  readonly destination: Extract<
    ProductNavigationDestination,
    { readonly kind: 'product_decision_publish' }
  >;
}) {
  return (
    <main
      className="product-decision-publish-placeholder"
      aria-label="Product Decision Publish placeholder"
      data-viewport-contained="true"
      data-publish-effect="none"
    >
      <p className="eyebrow">Product Decision · Version {destination.version}</p>
      <h1>Publish is not available yet</h1>
      <p>
        This placeholder does not publish, apply, reconcile, invalidate, settle, audit, or change
        orchestration state.
      </p>
      <p>
        Future publishing will determine applicability and any required changes. No applicability
        scope is inferred here.
      </p>
      <dl>
        <div>
          <dt>Decision</dt>
          <dd>{destination.decisionId}</dd>
        </div>
        <div>
          <dt>Stored version</dt>
          <dd>{destination.versionId}</dd>
        </div>
        <div>
          <dt>Application state</dt>
          <dd>Not applied</dd>
        </div>
      </dl>
    </main>
  );
}
