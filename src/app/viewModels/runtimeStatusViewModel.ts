import type { RuntimeStatusSnapshot } from '../../capabilities/runtimeHealth';
import { capitalize } from './formatting';

export function formatStaleTargets(targets: RuntimeStatusSnapshot['staleTargets']): string {
  if (targets.length === 0 || targets.includes('app')) {
    return 'App';
  }

  if (targets.length === 1) {
    return capitalize(targets[0]);
  }

  return targets.map(capitalize).join(' and ');
}
