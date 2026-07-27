import type { AgentReviewInstanceRef, AgentReviewTestSourceRef } from './contracts';
import {
  evaluateAgentReviewInstanceStatus,
  type AgentReviewInstanceRequest,
  type AgentReviewWorktreeRuntime,
} from './worktreeRuntime';

const request = {
  source: 'repository/review' as AgentReviewTestSourceRef,
  purpose: 'agent review',
} satisfies AgentReviewInstanceRequest;

describe('agent review worktree-runtime handoff', () => {
  it('matches the opaque, action-separated runtime facade', async () => {
    const handle = 'instance-1' as AgentReviewInstanceRef;
    const runtime = {
      request: vi.fn().mockResolvedValue({
        handle,
        status: { phase: 'prepared', health: 'not-observed', stale: false },
      }),
      build: vi.fn(),
      test: vi.fn(),
      start: vi.fn(),
      status: vi.fn(),
      stop: vi.fn(),
      recover: vi.fn(),
    } satisfies AgentReviewWorktreeRuntime;

    await expect(runtime.request(request)).resolves.toMatchObject({ handle });
    expect(request).not.toHaveProperty('worktreePath');
    expect(request).not.toHaveProperty('gitCommit');
    expect(runtime).not.toHaveProperty('buildAndLaunch');
  });

  it('requires current running health without claiming resource cleanup or attachment details', () => {
    expect(
      evaluateAgentReviewInstanceStatus({ phase: 'running', health: 'healthy', stale: false }),
    ).toEqual({ ready: true, reasons: [] });

    expect(
      evaluateAgentReviewInstanceStatus({ phase: 'stopped', health: 'closed', stale: true }),
    ).toEqual({
      ready: false,
      reasons: ['instance is not running', 'instance is not healthy', 'instance status is stale'],
    });
  });
});
