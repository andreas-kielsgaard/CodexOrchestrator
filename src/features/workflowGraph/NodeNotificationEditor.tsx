import type { NodeNotifications } from '../../application/completionNotifications';
import './workflowGraph.css';

export function NodeNotificationEditor({
  value,
  disabled = false,
  onChange,
}: {
  value: NodeNotifications;
  disabled?: boolean;
  onChange: (value: NodeNotifications) => void;
}) {
  return (
    <fieldset disabled={disabled}>
      <legend>Notifications</legend>
      <label className="node-notification-toggle">
        <input
          type="checkbox"
          checked={value.turnCompletionSound}
          onChange={(event) => onChange({ turnCompletionSound: event.target.checked })}
        />
        Sound when a turn finishes
      </label>
    </fieldset>
  );
}
