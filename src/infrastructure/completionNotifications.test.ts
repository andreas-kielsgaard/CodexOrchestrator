import { completedBuild } from '../features/worktreeReview/WorktreeReviewScreen.fixtures';
import type {
  AgentSessionUpdateDto,
  AgentSessionUpdateListener,
} from '../application/agentSessions';
import { InMemoryWorktreeReviewBuildActivitySource } from '../application/worktreeReview';
import { createCompletionNotificationSource } from './completionNotifications';

it('emits one completion per terminal invocation and review build', async () => {
  let update: AgentSessionUpdateListener = () => undefined;
  const stopSessions = vi.fn();
  const builds = new InMemoryWorktreeReviewBuildActivitySource();
  const source = createCompletionNotificationSource(
    {
      subscribeUpdates: async (listener) => {
        update = listener;
        return stopSessions;
      },
    },
    builds,
  );
  const received = vi.fn();
  const stop = await source.subscribe(received);
  const terminal = {
    kind: 'invocation_terminal',
    sessionId: 'session-one',
    invocationId: 'turn-one',
    invocation: { id: 'turn-one' },
  } as AgentSessionUpdateDto;

  update(terminal);
  update(terminal);
  builds.record(completedBuild);
  builds.record({ ...completedBuild, name: 'Same terminal build' });

  expect(received.mock.calls).toEqual([
    [{ kind: 'agent_turn', id: 'turn-one' }],
    [{ kind: 'worktree_review_build', id: completedBuild.buildId }],
  ]);
  stop();
  update({ ...terminal, invocationId: 'turn-two' } as AgentSessionUpdateDto);
  expect(received).toHaveBeenCalledTimes(2);
  expect(stopSessions).toHaveBeenCalledOnce();
});
