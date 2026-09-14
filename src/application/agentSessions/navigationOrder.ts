export type NavigationOrderScope =
  | { readonly kind: 'repositories' }
  | { readonly kind: 'sections'; readonly repositoryId: string }
  | { readonly kind: 'workflows'; readonly repositoryId: string };
export interface NavigationOrder {
  readonly scope: NavigationOrderScope;
  readonly orderedIds: readonly string[];
}
export interface NavigationOrderItem {
  readonly scope: NavigationOrderScope;
  readonly id: string;
}
export const orderScopeKey = (scope: NavigationOrderScope): string =>
  scope.kind === 'repositories' ? scope.kind : `${scope.kind}:${scope.repositoryId}`;

/** Keep saved siblings first; append new siblings in their supplied default order. */
export function applyNavigationOrder<T>(
  items: readonly T[],
  idOf: (item: T) => string,
  scope: NavigationOrderScope,
  orders: readonly NavigationOrder[],
): T[] {
  const saved = orders.find((order) => orderScopeKey(order.scope) === orderScopeKey(scope));
  const remaining = new Map(items.map((item) => [idOf(item), item]));
  const result: T[] = [];
  for (const id of saved?.orderedIds ?? []) {
    const item = remaining.get(id);
    if (item) {
      result.push(item);
      remaining.delete(id);
    }
  }
  return [...result, ...remaining.values()];
}

export function insertNavigationSibling(
  siblings: readonly string[],
  source: string,
  target: string,
  side: 'before' | 'after',
): string[] {
  if (source === target) return [...siblings];
  const result = siblings.filter((id) => id !== source);
  result.splice(result.indexOf(target) + (side === 'after' ? 1 : 0), 0, source);
  return result;
}
