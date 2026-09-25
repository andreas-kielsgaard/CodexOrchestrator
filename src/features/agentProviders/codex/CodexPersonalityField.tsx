import type { ProfileRoutePolicyDto } from '../../../application/executionConfiguration';

export function CodexPersonalityField({
  route,
  onChange,
}: {
  readonly route: ProfileRoutePolicyDto;
  readonly onChange: (route: ProfileRoutePolicyDto) => void;
}) {
  if (route.execution.provider !== 'codex') return null;
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
            value={route.codexPersonality ?? 'inherit'}
            onChange={(event) =>
              onChange({
                ...route,
                codexPersonality:
                  event.currentTarget.value === 'inherit'
                    ? null
                    : (event.currentTarget.value as 'none' | 'friendly' | 'pragmatic'),
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
