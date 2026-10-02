import { formatSessionMarkdown } from './sessionMarkdownExport';
import { projectAgentSessionTranscript } from './transcriptProjector';
import { runtimeEvent, sessionDetails } from './testFixtures';
function fixture() {
  const command = runtimeEvent(3, 'tool_activity', 'command');
  command.normalized!.toolActivity = {
    kind: 'command',
    phase: 'completed',
    itemId: 'command',
    server: null,
    tool: null,
    status: 'completed',
    resultClassification: 'succeeded',
  };
  command.rawPayload = {
    item: {
      id: 'command',
      type: 'command_execution',
      command: 'echo exact-input',
      aggregated_output: 'exact-output',
      exit_code: 0,
    },
  };
  const edit = runtimeEvent(4, 'tool_activity', 'edit', {
    fileChanges: [
      { path: 'src/a.ts', operation: 'update' },
      { path: 'src/b.ts', operation: 'add' },
    ],
  });
  edit.normalized!.toolActivity = {
    ...command.normalized!.toolActivity,
    kind: 'file_change',
    itemId: 'edit',
  };
  const details = sessionDetails('completed', [
    runtimeEvent(1, 'processing_update', 'Think carefully'),
    runtimeEvent(2, 'agent_message', 'Progress update', { role: 'commentary' }),
    command,
    edit,
    runtimeEvent(5, 'agent_message', 'Final output', { role: 'final' }),
  ]);
  return { details, transcript: projectAgentSessionTranscript(details) };
}
it('exports cumulative detail levels without leaking action details into summaries', () => {
  const { details, transcript } = fixture();
  const messages = formatSessionMarkdown(details, transcript, 'messages');
  expect(messages).toContain('Do the work');
  expect(messages).toContain('Final output');
  expect(messages).toContain('Progress update');
  expect(messages).not.toContain('Think carefully');
  expect(messages).not.toContain('echo exact-input');
  const reasoning = formatSessionMarkdown(details, transcript, 'reasoning');
  expect(reasoning).toContain('Think carefully');
  expect(reasoning).not.toContain('echo exact-input');
  const actions = formatSessionMarkdown(details, transcript, 'actions');
  expect(actions).toContain('src/a.ts');
  expect(actions).toContain('src/b.ts');
  expect(actions).toContain('Ran echo exact-input');
  expect(actions).not.toContain('exact-output');
  const full = formatSessionMarkdown(details, transcript, 'actionsPlus');
  expect(full).toContain('exact-output');
  expect(full).toContain('Exit code');
});
it('keeps accepted steering in chronological order and does not export pending steering', () => {
  const { details, transcript } = fixture();
  transcript.invocations[0].interactions = [
    {
      id: 'steer',
      invocationId: 'invocation-1',
      sequence: 2.5,
      kind: 'steering',
      state: 'accepted',
      content: { text: 'Accepted correction' },
      result: null,
    },
    {
      id: 'pending',
      invocationId: 'invocation-1',
      sequence: 6,
      kind: 'steering',
      state: 'pending',
      content: { text: 'Not delivered' },
      result: null,
    },
  ];
  const result = formatSessionMarkdown(details, transcript, 'actions');
  expect(result.indexOf('Accepted correction')).toBeLessThan(result.indexOf('echo exact-input'));
  expect(result).not.toContain('Not delivered');
});
