import type { AgentSessionDetailsDto } from '../../application/agentSessions';
import { formatSessionMarkdown, type SessionMarkdownMode } from './sessionMarkdownExport';
import { projectAgentSessionTranscript } from './transcriptProjector';

/** Large full-detail exports format away from the interaction thread. */
export function exportSessionMarkdown(
  details: AgentSessionDetailsDto,
  mode: SessionMarkdownMode,
): Promise<string> {
  if (typeof Worker === 'undefined')
    return Promise.resolve(
      formatSessionMarkdown(details, projectAgentSessionTranscript(details), mode),
    );
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./sessionMarkdownWorker.ts', import.meta.url), {
      type: 'module',
    });
    worker.onmessage = (event) => {
      worker.terminate();
      if (event.data.error) reject(new Error(event.data.error));
      else resolve(event.data.text);
    };
    worker.onerror = (event) => {
      worker.terminate();
      reject(new Error(event.message));
    };
    worker.postMessage({ details, mode });
  });
}
