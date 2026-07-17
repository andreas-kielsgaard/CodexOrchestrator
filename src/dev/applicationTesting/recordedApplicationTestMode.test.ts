import { describe, expect, it } from 'vitest';
import {
  createRecordedApplicationTestModeComposition,
  RECORDED_TEST_VIEW_ID,
} from './recordedApplicationTestMode';

describe('recorded application test mode', () => {
  it('navigates, acts, observes, and waits through semantic identifiers', async () => {
    const { controller } = createRecordedApplicationTestModeComposition();

    expect((await controller.navigate(RECORDED_TEST_VIEW_ID)).state.active).toBe(true);
    const acted = await controller.performAction({
      viewId: RECORDED_TEST_VIEW_ID,
      elementId: 'agent_sessions.runtime_fixture',
      actionId: 'advance_recorded_runtime',
    });
    expect(acted.state.eventCount).toBe(1);
    expect(
      (
        await controller.waitFor({
          viewId: RECORDED_TEST_VIEW_ID,
          condition: { kind: 'state_at_least', key: 'eventCount', value: 1 },
          timeoutMs: 25,
        })
      ).state.latestEventKind,
    ).toBe('processing_started');
  });

  it('fails closed for pixels and describes a bounded demo capture envelope', async () => {
    const { controller } = createRecordedApplicationTestModeComposition();

    await expect(
      controller.captureScreenshot({ viewId: RECORDED_TEST_VIEW_ID }),
    ).resolves.toMatchObject({
      status: 'unavailable',
    });
    await expect(controller.describeDemoCapture()).resolves.toMatchObject({
      status: 'adapter_required',
      captureRootElementId: 'agent_test_mode.preview_root',
      excludedData: expect.arrayContaining(['product Agent Sessions']),
    });
    await expect(
      controller.captureScreenshot({
        viewId: RECORDED_TEST_VIEW_ID,
        elementId: 'body',
      }),
    ).rejects.toThrow('Unknown semantic element');
  });

  it('delivers anchored feedback to the recorded sink without product authority', async () => {
    const { controller, deliveredFeedback } = createRecordedApplicationTestModeComposition();
    await controller.addAnnotation({
      message: 'Keep the processing status visible.',
      anchor: {
        viewId: RECORDED_TEST_VIEW_ID,
        elementId: 'agent_sessions.status',
        stateKey: 'invocationStatus',
      },
    });

    const delivery = await controller.deliverFeedback('agent-session-worker');

    expect(delivery).toMatchObject({
      status: 'delivered',
      channel: 'recorded_test_sink',
      envelope: {
        authority: 'feedback_only',
        targetAgentSessionId: 'agent-session-worker',
      },
    });
    expect(deliveredFeedback).toHaveLength(1);
    expect(deliveredFeedback[0]).not.toHaveProperty('orchestrationEvent');
  });

  it('rejects views and actions outside the declared scope', async () => {
    const { controller } = createRecordedApplicationTestModeComposition();

    await expect(controller.observe('orchestration.overview')).rejects.toThrow(
      'View is outside this test session',
    );
    await expect(
      controller.performAction({
        viewId: RECORDED_TEST_VIEW_ID,
        elementId: 'agent_sessions.transcript',
        actionId: 'click_at_coordinates',
      }),
    ).rejects.toThrow('Semantic action is not available');
  });
});
