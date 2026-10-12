import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { JevQueryInspector } from './JevQueryInspector';
import type {
  JevQueryDetail,
  JevQuerySummary,
} from '../../../application/agentSessions/jevQueries';
function detail(id: string): JevQueryDetail {
  return {
    invocation: {
      id,
      createdAt: '2026-10-12T12:00:00Z',
      startedAt: '2026-10-12T12:00:00Z',
      completedAt: '2026-10-12T12:00:01Z',
      status: 'completed',
      requestedOptions: { model: 'jev-latest', sandbox: null },
      effectiveOptions: null,
      runtimeError: null,
    } as JevQueryDetail['invocation'],
    evaluation: {
      state: { recentSpeech: [{ id: 'sentence', revision: 2, text: 'Actually, vegetarian.' }] },
      questions: {
        'relevance:sentence': {
          type: 'choice',
          instructions: 'Addressed to Orchid?',
          criteria: { unaddressed: 'Ambient', task_context: 'Relevant' },
        },
        score: { type: 'score', instructions: 'Effort', criteria: ['Low', 'High'] },
        urgent: { type: 'noul', instructions: 'Urgent?' },
      },
    },
    result: {
      model: 'jev-1.13.0',
      usage: { input_tokens: 200, output_tokens: 10 },
      answers: {
        'relevance:sentence': {
          type: 'choice',
          choice: id === 'old' ? 'task_context' : 'unaddressed',
          confidence: 0.35,
          probabilities: { unaddressed: 0.57, task_context: 0.43 },
        },
        score: {
          type: 'score',
          score: 0.9,
          legend: { 0: 'Low', 1: 'High' },
          probabilities: { 0: 0.1, 1: 0.9 },
          confidence: 0.8,
        },
        urgent: { type: 'noul', noul: 0.1 },
      },
    },
  };
}
const rows: JevQuerySummary[] = ['new', 'old'].map((id) => ({
  invocationId: id,
  status: 'completed',
  createdAt: '2026-10-12T12:00:00Z',
  startedAt: null,
  completedAt: null,
  inputPreview: id === 'new' ? 'Actually, vegetarian.' : 'Original question',
  questionCount: 3,
  answers: [],
}));
it('shows typed questions, separate probability/confidence and the actual retained decision', async () => {
  const client = {
    list: vi.fn().mockResolvedValue(rows),
    load: vi.fn(async (_: string, id: string) => detail(id)),
  };
  render(
    <JevQueryInspector
      client={client}
      sessionId="filter"
      applications={[
        {
          invocationId: 'new',
          releaseId: null,
          decisions: [
            {
              sentenceId: 'sentence',
              revision: 2,
              currentRevision: 2,
              applied: true,
              relevance: 'task_context',
              relevanceRetained: true,
              completion: 'continue',
              reaction: 'work_or_context',
              buffered: true,
            },
          ],
        },
      ]}
    />,
  );
  const question = await screen.findByRole('region', { name: 'Question relevance:sentence' });
  expect(within(question).getByText('Confidence: 35.0%')).toBeVisible();
  expect(within(question).getByText('57.0%')).toBeVisible();
  expect(screen.getByText(/retained earlier relevance/)).toBeVisible();
  expect(screen.getByText('Noul 0.100')).toBeVisible();
  fireEvent.click(screen.getByRole('checkbox', { name: 'Compare previous evaluation' }));
  expect(await screen.findByText('Previous: task_context → changed')).toBeVisible();
});
it('keeps an older query selected when new evaluations arrive', async () => {
  const client = {
    list: vi.fn().mockResolvedValue(rows),
    load: vi.fn(async (_: string, id: string) => detail(id)),
  };
  render(<JevQueryInspector client={client} sessionId="filter" />);
  fireEvent.click(await screen.findByRole('button', { name: /Original question/ }));
  await waitFor(() => expect(client.load).toHaveBeenLastCalledWith('filter', 'old'));
  client.list.mockResolvedValue([
    { ...rows[0], invocationId: 'latest', inputPreview: 'New input' },
    ...rows,
  ]);
  fireEvent.click(screen.getByRole('button', { name: 'Refresh queries' }));
  await screen.findByRole('button', { name: /New input/ });
  expect(screen.getByRole('button', { name: /Original question/ })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
});
