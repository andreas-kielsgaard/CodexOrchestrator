import { formatSessionMarkdown } from './sessionMarkdownExport';
import { projectAgentSessionTranscript } from './transcriptProjector';
self.onmessage = (event) => {
  try {
    self.postMessage({
      text: formatSessionMarkdown(
        event.data.details,
        projectAgentSessionTranscript(event.data.details),
        event.data.mode,
      ),
    });
  } catch (cause) {
    self.postMessage({ error: String(cause) });
  }
};
