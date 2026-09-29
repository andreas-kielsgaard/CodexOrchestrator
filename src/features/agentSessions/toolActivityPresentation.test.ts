import { describe, expect, it } from 'vitest';
import type { TranscriptActivity } from './transcriptProjector';
import { presentToolActivity } from './toolActivityPresentation';

describe('presentToolActivity', () => {
  it('shows provider inputs, results, runtime, and parallel call count when available', () => {
    const activity: TranscriptActivity = {
      id: 'event-1',
      sequence: 1,
      kind: 'tool',
      text: 'MCP tool call',
      source: 'stdout',
      recordedAt: '2026-09-29T10:00:00.000Z',
      completedAt: '2026-09-29T10:00:01.250Z',
      normalizedDetails: null,
      safeDetail: {
        kind: 'tool',
        activity: 'mcp_tool',
        phase: 'completed',
        itemId: 'tool-1',
        server: 'functions',
        tool: 'exec',
        status: 'completed',
        resultClassification: 'succeeded',
      },
      rawPayload: {
        lifecycleEvents: [
          {
            message: {
              content: [
                {
                  type: 'tool_use',
                  id: 'tool-1',
                  name: 'exec',
                  input: { command: 'npm test' },
                },
                {
                  type: 'tool_use',
                  id: 'tool-2',
                  name: 'read_file',
                  input: { path: 'package.json' },
                },
              ],
            },
          },
          {
            tool_use_result: { stdout: '10 tests passed', stderr: '' },
            status: 'completed',
          },
        ],
      },
    };

    const presentation = presentToolActivity(activity, false);

    expect(presentation.label).toBe('Used functions / exec');
    expect(presentation.batchLabel).toBe('Batch · 2 calls');
    expect(presentation.details).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ label: 'Input', value: expect.stringContaining('npm test') }),
        expect.objectContaining({
          label: 'Result',
          value: expect.stringContaining('10 tests passed'),
        }),
        expect.objectContaining({ label: 'Runtime', value: '1.3 s' }),
      ]),
    );
  });
});
