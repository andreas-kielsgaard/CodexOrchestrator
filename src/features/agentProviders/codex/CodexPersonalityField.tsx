import type { ProviderRouteSettingsProps } from '../ProviderRouteSettings';
import { codexOptions, codexPersonality, type CodexPersonality } from './codexOptions';

export function CodexPersonalityField({ route, onChange }: ProviderRouteSettingsProps) {
  return (
    <section aria-labelledby={`${route.routeId}-personality`}>
      <div className="capability-route__section-heading">
        <div>
          <h4 id={`${route.routeId}-personality`}>Codex personality</h4>
          <p>Override the selected Codex profile default for sessions launched on this route.</p>
        </div>
        <label>
          <span className="sr-only">Codex personality</span>
          <select
            value={codexPersonality(route.providerOptions) ?? 'inherit'}
            onChange={(event) =>
              onChange({
                ...route,
                providerOptions: codexOptions(
                  event.currentTarget.value === 'inherit'
                    ? null
                    : (event.currentTarget.value as CodexPersonality),
                ),
              })
            }
          >
            <option value="inherit">Use Codex profile default</option>
            <option value="none">None</option>
            <option value="friendly">Friendly</option>
            <option value="pragmatic">Pragmatic</option>
          </select>
        </label>
      </div>
    </section>
  );
}
