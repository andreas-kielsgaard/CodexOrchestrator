import { useState } from 'react';
import { AgentSessionScreen } from '../../features/agentSessions/AgentSessionScreen';
import {
  createRecordedAgentSessionClient,
  createRecordedAgentSessionStore,
  recordedAgentSessionScenarios,
  type RecordedAgentSessionClient,
  type RecordedAgentSessionScenarioName,
  type RecordedAgentSessionStore,
} from './index';

const scenarios = Object.values(recordedAgentSessionScenarios);
const defaultScenarioKey: RecordedAgentSessionScenarioName = 'liveProcessing';

interface HarnessState {
  client: RecordedAgentSessionClient;
  scenarioKey: RecordedAgentSessionScenarioName;
  store: RecordedAgentSessionStore;
  revision: number;
}

export function AgentSessionHarness() {
  const [state, setState] = useState(() => createHarnessState(readScenarioKey()));
  const [, setStatusRevision] = useState(0);
  const scenario = recordedAgentSessionScenarios[state.scenarioKey];
  const remaining = state.client.stepCount - state.client.stepIndex;
  const next = state.client.peekNextStep();
  const status = `Step ${state.client.stepIndex} of ${state.client.stepCount} · ${remaining} remaining${
    next ? ` · next: ${next.kind}` : ' · complete'
  }`;

  const replaceState = (
    scenarioKey: RecordedAgentSessionScenarioName,
    store: RecordedAgentSessionStore,
  ) => {
    void state.client.disconnectUpdates();
    setState({
      client: createRecordedAgentSessionClient({
        store,
        scenario: recordedAgentSessionScenarios[scenarioKey],
      }),
      scenarioKey,
      store,
      revision: state.revision + 1,
    });
    setStatusRevision((value) => value + 1);
  };

  const advance = () => {
    state.client.advance();
    setStatusRevision((value) => value + 1);
  };

  const advanceAll = () => {
    state.client.advanceAll();
    setStatusRevision((value) => value + 1);
  };

  return (
    <div className="agent-session-harness">
      <header className="agent-session-harness-controls">
        <label>
          <span>Scenario</span>
          <select
            aria-label="Recorded scenario"
            value={scenario.name}
            onChange={(event) =>
              replaceState(scenarioKeyFor(event.target.value), createRecordedAgentSessionStore())
            }
          >
            {scenarios.map((item) => (
              <option key={item.name} value={item.name}>
                {item.name}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={advance} disabled={!state.client.peekNextStep()}>
          Advance one step
        </button>
        <button type="button" onClick={advanceAll} disabled={!state.client.peekNextStep()}>
          Advance all remaining
        </button>
        <button
          type="button"
          onClick={() => replaceState(state.scenarioKey, createRecordedAgentSessionStore())}
        >
          Reset
        </button>
        <button type="button" onClick={() => replaceState(state.scenarioKey, state.store)}>
          Restart screen
        </button>
        <output className="agent-session-harness-status" role="status" aria-live="polite">
          {status}
        </output>
        <small className="agent-session-harness-note">
          Development harness · StrictMode intentionally off for deterministic controls
        </small>
      </header>
      <AgentSessionScreen key={state.revision} client={state.client} />
    </div>
  );
}

function createHarnessState(scenarioKey: RecordedAgentSessionScenarioName): HarnessState {
  const store = createRecordedAgentSessionStore();
  return {
    client: createRecordedAgentSessionClient({
      store,
      scenario: recordedAgentSessionScenarios[scenarioKey],
    }),
    scenarioKey,
    store,
    revision: 0,
  };
}

function readScenarioKey(): RecordedAgentSessionScenarioName {
  const value = new URLSearchParams(window.location.search).get('scenario');
  return scenarioKeyFor(value ?? recordedAgentSessionScenarios[defaultScenarioKey].name);
}

function scenarioKeyFor(value: string): RecordedAgentSessionScenarioName {
  const match = Object.entries(recordedAgentSessionScenarios).find(
    ([, scenario]) => scenario.name === value,
  );
  return (match?.[0] ?? defaultScenarioKey) as RecordedAgentSessionScenarioName;
}
