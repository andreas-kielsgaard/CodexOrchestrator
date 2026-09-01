import { useState } from 'react';
import type {
  SessionEventDefinitionDto,
  SessionEventResultDto,
} from '../../application/sessionEvents';
import type { WorkflowAuthoringClient } from '../../application/workflowAuthoring';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import { EventGroupInspector } from '../sessionEvents';
import { errorMessage } from './workflowAuthoringPresentation';

export function WorkflowRunPanel({
  client,
  recipeId,
  compiled,
  eventResult,
  onCompiled,
  onEventResult,
  onError,
}: {
  readonly client: WorkflowAuthoringClient;
  readonly recipeId: string;
  readonly compiled: readonly SessionEventDefinitionDto[];
  readonly eventResult: SessionEventResultDto | null;
  readonly onCompiled: (value: readonly SessionEventDefinitionDto[]) => void;
  readonly onEventResult: (value: SessionEventResultDto | null) => void;
  readonly onError: (value: string | null) => void;
}) {
  const [instanceId, setInstanceId] = useState(() => `workflow-instance-${crypto.randomUUID()}`);
  const [message, setMessage] = useState('');
  const [busy, setBusy] = useState(false);
  const act = async (action: () => Promise<void>) => {
    setBusy(true);
    onError(null);
    try {
      await action();
    } catch (caught) {
      onError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="workflow-run-panel">
      <header>
        <p>Active recipe</p>
        <h1>Compile and run</h1>
        <span>The saved active revision is compiled into Session Event definitions.</span>
      </header>
      <CollapsibleSection
        title="Workflow instance"
        description="One logical instance scopes the Sessions addressed by this run."
        className="execution-configuration__section"
      >
        <label>
          <span>Instance ID</span>
          <input
            value={instanceId}
            onChange={(event) => setInstanceId(event.currentTarget.value)}
          />
        </label>
        <button
          type="button"
          disabled={busy || !instanceId.trim()}
          onClick={() =>
            void act(async () =>
              onCompiled(await client.compileRecipeInstance(recipeId, instanceId.trim())),
            )
          }
        >
          Compile active recipe
        </button>
        {compiled.length ? (
          <p>
            {compiled.length} Session Event definition{compiled.length === 1 ? '' : 's'} compiled.
          </p>
        ) : null}
      </CollapsibleSection>
      <CollapsibleSection
        title="User-triggered Session Event"
        description="Dispatches the starting node event for this Workflow instance."
        className="execution-configuration__section"
      >
        <label>
          <span>User message</span>
          <textarea
            rows={5}
            value={message}
            onChange={(event) => setMessage(event.currentTarget.value)}
          />
        </label>
        <button
          type="button"
          disabled={busy || !instanceId.trim() || !message.trim()}
          onClick={() =>
            void act(async () =>
              onEventResult(
                await client.dispatchUserRequest({
                  recipeId,
                  instanceId: instanceId.trim(),
                  text: message.trim(),
                }),
              ),
            )
          }
        >
          Dispatch user request
        </button>
      </CollapsibleSection>
      {eventResult ? (
        <EventGroupInspector group={eventResult.group} deliveries={eventResult.deliveries} />
      ) : null}
    </div>
  );
}
