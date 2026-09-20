import type { CapabilityProfileDraft, HarnessInferenceRouteOption } from './types';

function sameExecution(
  left: CapabilityProfileDraft['execution'],
  right: HarnessInferenceRouteOption,
): boolean {
  if (!left) return right.selected;
  return (
    left?.deviceId === right.execution.deviceId &&
    (left.configurationRef === right.execution.configurationRef ||
      (left.configurationRef === 'selected' && right.selected)) &&
    left.connection.kind === right.execution.connection.kind
  );
}

export function HarnessInferenceRouteFields({
  profile,
  routes,
  onChange,
}: {
  readonly profile: CapabilityProfileDraft;
  readonly routes: readonly HarnessInferenceRouteOption[];
  onChange(profile: CapabilityProfileDraft): void;
}) {
  const selected = routes.find((route) => sameExecution(profile.execution, route));
  return (
    <section className="execution-configuration__route" aria-labelledby="profile-route-title">
      <div>
        <h3 id="profile-route-title">Harness &amp; inference source</h3>
        <p>
          This profile uses one configured harness-to-source connection. Credentials stay with the
          harness; Orchid stores only this route selection.
        </p>
      </div>
      {routes.length ? (
        <label className="execution-configuration__field">
          <span>Route</span>
          <select
            aria-label="Capability profile harness and inference source"
            value={selected?.id ?? ''}
            onChange={(event) => {
              const route = routes.find((candidate) => candidate.id === event.currentTarget.value);
              if (route) onChange({ ...profile, execution: route.execution });
            }}
          >
            {!selected ? <option value="">Choose a configured route</option> : null}
            {routes.map((route) => (
              <option key={route.id} value={route.id}>
                {route.label} — {route.sourceLabel}
              </option>
            ))}
          </select>
          {selected ? <small>{selected.detail}</small> : null}
        </label>
      ) : (
        <p className="execution-configuration__route-empty" role="status">
          No local Codex harness is available. Add or register a Codex home in Technical Settings
          before assigning this profile a route.
        </p>
      )}
    </section>
  );
}
