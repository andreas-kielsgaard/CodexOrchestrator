import { describe, expect, it } from 'vitest';
import {
  APPLICATION_TEST_FEEDBACK_V1,
  type ApplicationTestFeedbackEnvelope,
} from './applicationTestMode';

describe('application test-mode contracts', () => {
  it('keeps feedback explicitly non-authoritative', () => {
    const envelope: ApplicationTestFeedbackEnvelope = {
      version: APPLICATION_TEST_FEEDBACK_V1,
      authority: 'feedback_only',
      source: 'application_test_mode',
      buildRef: 'build-1',
      testSessionId: 'test-session-1',
      targetAgentSessionId: 'agent-session-1',
      annotations: [],
      screenshotEvidenceIds: [],
    };

    expect(envelope.authority).toBe('feedback_only');
    expect(envelope).not.toHaveProperty('orchestrationEvent');
    expect(envelope).not.toHaveProperty('agentControlCommand');
  });
});
