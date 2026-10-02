import type { AgentSessionDetailsDto } from '../../application/agentSessions';
import type { ProjectedTranscript, TranscriptActivity } from './transcriptProjector';
import { presentToolActivity } from './toolActivityPresentation';
import { redactSensitiveText } from './sessionClipboard';

export type SessionMarkdownMode = 'messages' | 'reasoning' | 'actions' | 'actionsPlus';
export function formatSessionMarkdown(
  details: AgentSessionDetailsDto,
  transcript: ProjectedTranscript,
  mode: SessionMarkdownMode,
): string {
  const blocks = [`# ${details.session.title}`];
  const reasoning = mode !== 'messages';
  const actions = mode === 'actions' || mode === 'actionsPlus';
  const message = (role: string, text: string) => {
    if (text.trim()) blocks.push(`## ${role}\n\n${text}`);
  };
  const activity = (item: TranscriptActivity) => {
    if (item.kind === 'agent_intermediate') message('Agent', item.text);
    else if (item.kind === 'processing' && reasoning) message('Reasoning', item.text);
    else if (item.kind === 'tool' && actions) {
      const shown = presentToolActivity(item, false);
      const files = shown.details.find((row) => row.label === 'Files')?.value.split('\n');
      const label = files?.length
        ? `Edited ${files.length === 1 ? 'file' : 'files'}: ${files.join(', ')}`
        : shown.label;
      blocks.push(
        `### ${label}${shown.failed ? ' · Failed' : ''}${shown.batchLabel ? ` · ${shown.batchLabel}` : ''}`,
      );
      if (mode === 'actionsPlus') {
        for (const row of shown.details) blocks.push(`**${row.label}**\n\n${fenced(row.value)}`);
        // Preserve the actual provider command rather than only the shortened display label.
        if (item.rawPayload)
          blocks.push(
            `**Provider action data**\n\n${fenced(JSON.stringify(item.rawPayload, null, 2), 'json')}`,
          );
      }
    }
  };
  for (const turn of transcript.invocations) {
    if (turn.imported) {
      for (const item of turn.imported.items) {
        if (item.kind === 'user') message('You', item.text);
        else if (item.kind === 'assistant') message('Agent', item.text);
        else if (actions) message('Historical activity', item.text);
      }
      continue;
    }
    message('You', turn.submittedText);
    if (turn.attachments.length)
      blocks.push(turn.attachments.map((file) => `Attached file: ${file.name}`).join('\n'));
    const content: Array<{ sequence: number; item?: TranscriptActivity; text?: string }> =
      turn.processing.map((item) => ({ sequence: item.sequence, item }));
    for (const steering of turn.interactions ?? [])
      if (steering.kind === 'steering' && steering.state === 'accepted' && steering.content.text)
        content.push({ sequence: steering.sequence, text: steering.content.text });
    for (const entry of content.sort((a, b) => a.sequence - b.sequence)) {
      if (entry.item) activity(entry.item);
      else message('You', entry.text!);
    }
    if (turn.finalResponse) message('Agent', turn.finalResponse.text);
  }
  return redactSensitiveText(`${blocks.join('\n\n')}\n`);
}
function fenced(value: string, language = '') {
  const ticks = '`'.repeat(
    Math.max(3, ...[...value.matchAll(/`+/g)].map((match) => match[0].length + 1)),
  );
  return `${ticks}${language}\n${value}\n${ticks}`;
}
