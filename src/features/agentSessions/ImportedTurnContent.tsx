import type { ImportedTranscript } from './importedTranscript';
import { AgentMarkdown } from './AgentMarkdown';
export function ImportedTurnContent({
  history,
  status,
  safeOnly = false,
  showOutcome = true,
}: {
  history: ImportedTranscript;
  status: string;
  safeOnly?: boolean;
  showOutcome?: boolean;
}) {
  return (
    <>
      <p className="imported-turn-label">
        Imported from Codex
        {history.sourceStartedAt !== null
          ? ' · ' + new Date(history.sourceStartedAt * 1000).toLocaleString()
          : ''}
      </p>
      {history.items.map((item) =>
        item.kind === 'activity' ? (
          <details key={item.eventId} className="imported-history-activity">
            <summary>Historical activity</summary>
            <pre>
              {safeOnly ? 'Open this session to view historical activity details.' : item.text}
            </pre>
          </details>
        ) : (
          <article
            key={item.eventId}
            className={
              'transcript-message ' + (item.kind === 'user' ? 'user-message' : 'agent-message')
            }
          >
            <header>{item.kind === 'user' ? 'User (imported)' : 'Agent'}</header>
            {item.kind === 'assistant' ? (
              <AgentMarkdown>{item.text}</AgentMarkdown>
            ) : (
              <p>{item.text}</p>
            )}
          </article>
        ),
      )}
      {showOutcome && status !== 'completed' && (
        <p className="imported-turn-label">Historical outcome: {status}</p>
      )}
    </>
  );
}
