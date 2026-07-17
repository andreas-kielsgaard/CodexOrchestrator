import type { AgentSessionDetailsDto } from '../../application/agentSessions';
import type { ProjectedTranscript } from './transcriptProjector';

export interface AgentSessionClipboard {
  writeText(text: string): Promise<void>;
}

export const browserAgentSessionClipboard: AgentSessionClipboard = {
  async writeText(text) {
    if (!navigator.clipboard?.writeText) throw new Error('Clipboard access is unavailable.');
    await navigator.clipboard.writeText(text);
  },
};

/** A readable session export. Raw payloads and environment data are intentionally excluded. */
export function formatAgentSessionContext(
  details: AgentSessionDetailsDto,
  transcript: ProjectedTranscript,
): string {
  const lines = [
    'Agent Session',
    `Title: ${details.session.title}`,
    `Session ID: ${details.session.id}`,
    `Availability: ${details.session.availability}`,
    `Created: ${details.session.createdAt}`,
    `Updated: ${details.session.updatedAt}`,
  ];

  transcript.invocations.forEach((projected, index) => {
    const source = details.invocations.find(
      ({ invocation }) => invocation.id === projected.id,
    )?.invocation;
    lines.push(
      '',
      `Turn ${index + 1}`,
      `Invocation ID: ${projected.id}`,
      `Status: ${projected.status}`,
      `Created: ${projected.createdAt}`,
    );
    if (source?.startedAt) lines.push(`Started: ${source.startedAt}`);
    if (source?.completedAt) lines.push(`Completed: ${source.completedAt}`);
    lines.push(
      '',
      projected.inputProvenance === 'application' ? 'Plan Builder / Application' : 'User',
      redactSensitiveText(projected.submittedText),
    );

    const tools = projected.processing.filter((activity) => activity.kind === 'tool');
    if (tools.length) {
      lines.push('', 'Tool activity');
      tools.forEach((activity) => {
        lines.push(`- ${activity.recordedAt}: ${redactSensitiveText(activity.text)}`);
      });
    }

    const agentActivity = projected.processing.filter(
      (activity) => activity.kind === 'agent_intermediate',
    );
    if (agentActivity.length) {
      lines.push('', 'Agent intermediate messages');
      agentActivity.forEach((activity) => {
        lines.push(`- ${activity.recordedAt}: ${redactSensitiveText(activity.text)}`);
      });
    }

    if (projected.finalResponse) {
      lines.push('', 'Agent', redactSensitiveText(projected.finalResponse.text));
    }

    if (projected.diagnostics.length) {
      lines.push('', 'Diagnostics');
      projected.diagnostics.forEach((diagnostic) => {
        lines.push(
          `- ${diagnostic.recordedAt} [${diagnostic.severity}/${diagnostic.source}/${diagnostic.code}] ${redactSensitiveText(diagnostic.message)}`,
        );
      });
    }

    if (projected.outcome.message) {
      lines.push('', `Outcome: ${redactSensitiveText(projected.outcome.message)}`);
    }
  });

  return `${lines.join('\n').trimEnd()}\n`;
}

function redactSensitiveText(value: string): string {
  return value
    .replace(/\bBearer\s+[A-Za-z0-9._~+/-]+=*/gi, 'Bearer [REDACTED]')
    .replace(
      /\b(api[_-]?key|access[_-]?token|authorization|password)\s*[:=]\s*([^\s,;]+)/gi,
      '$1=[REDACTED]',
    );
}
