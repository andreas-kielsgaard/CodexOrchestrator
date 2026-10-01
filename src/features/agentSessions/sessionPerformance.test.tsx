import { Profiler, type ProfilerOnRenderCallback } from 'react';
import { render } from '@testing-library/react';
import { ConversationViewport } from './ConversationViewport';
import { projectAgentSessionTranscript } from './transcriptProjector';
import { runtimeEvent, sessionDetails } from './testFixtures';
import * as invocations from './AgentSessionInvocation';

it('keeps draft edits independent of a long conversation', () => {
  const invocationRender = vi.spyOn(invocations, 'AgentSessionInvocation');
  const details = sessionDetails('completed');
  details.invocations = Array.from({ length: 20 }, (_, index) => {
    const id = `invocation-${index}`;
    return {
      ...details.invocations[0],
      invocation: { ...details.invocations[0].invocation, id },
      events: Array.from({ length: 40 }, (_, sequence) => ({
        ...runtimeEvent(sequence + 1, 'agent_message', `Update **${sequence}** with some text`, {
          role: sequence === 39 ? 'final' : 'commentary',
        }),
        id: `${id}-event-${sequence}`,
        invocationId: id,
      })),
    };
  });
  const segments = [{ id: 'history', transcript: projectAgentSessionTranscript(details) }];
  const expanded = new Set<string>();
  const noop = () => undefined;
  const durations: number[] = [];
  const onRender: ProfilerOnRenderCallback = (_, phase, duration) => {
    if (phase === 'update') durations.push(duration);
  };
  const view = (draft: string) => (
    <Profiler id="conversation" onRender={onRender}>
      <ConversationViewport
        segments={segments}
        loading={false}
        expandedProcessing={expanded}
        onToggleProcessing={noop}
        composerTarget={{
          sessionId: 'session-1',
          draft,
          workingDirectory: '',
          sending: false,
          active: false,
          canceling: false,
          setDraft: noop,
          setWorkingDirectory: noop,
          send: async () => undefined,
          cancel: async () => undefined,
        }}
      />
    </Profiler>
  );
  const { rerender, container } = render(view(''));
  const renderedTurns = invocationRender.mock.calls.length;
  expect(renderedTurns).toBe(20);
  const history = container.querySelector('.agent-transcript')!;
  expect(container.querySelectorAll('.turn-activity li')).toHaveLength(0);
  let mutations = 0;
  const observer = new MutationObserver(() => mutations++);
  observer.observe(history, { childList: true, subtree: true, characterData: true });
  for (let i = 1; i <= 5; i++) rerender(view('a'.repeat(i)));
  mutations += observer.takeRecords().length;
  observer.disconnect();
  console.info(
    `Long-session draft render average: ${(durations.reduce((a, b) => a + b, 0) / durations.length).toFixed(1)}ms; hidden activity rows: ${container.querySelectorAll('.turn-activity li').length}`,
  );
  expect(mutations).toBe(0);
  expect(invocationRender).toHaveBeenCalledTimes(renderedTurns);
  invocationRender.mockRestore();
});
