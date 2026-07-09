import {
  AlertCircle,
  Check,
  CheckCircle2,
  LoaderCircle,
  Send,
  ShieldCheck,
  Upload,
  X,
} from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import type { EntityId } from '../../../domain/model';
import type { AgentSessionRouter } from '../../../application/agentSessionRouter';
import type {
  AgentSessionDisplayItem,
  AgentSessionViewModel,
} from '../../../application/agentSessionOutputFormatter';
import {
  fallbackRuntimeInfo,
  type CodexReasoningEffort,
  type CodexRuntimeInfo,
} from '../../../application/codexRuntimeInfoProvider';
import { ErrorOutputIndicator } from '../../../ui';
import { errorMessage } from '../../../app/viewModels/formatting';

export interface AgentSessionPageProps {
  agentSessionRouter: AgentSessionRouter;
  loadCodexRuntimeInfo(): Promise<CodexRuntimeInfo>;
}
type AgentSandboxMode = 'read-only' | 'workspace-write' | 'danger-full-access';

export function AgentSessionPage({
  agentSessionRouter,
  loadCodexRuntimeInfo,
}: AgentSessionPageProps) {
  const [viewModel, setViewModel] = useState<AgentSessionViewModel>(() =>
    agentSessionRouter.emptyViewModel(),
  );
  const [prompt, setPrompt] = useState('');
  const [runtimeInfo, setRuntimeInfo] = useState<CodexRuntimeInfo>(fallbackRuntimeInfo);
  const [selectedModel, setSelectedModel] = useState(fallbackRuntimeInfo.recommendedModel);
  const [selectedSandbox, setSelectedSandbox] = useState<AgentSandboxMode>('danger-full-access');
  const [selectedReasoning, setSelectedReasoning] = useState<CodexReasoningEffort>('high');
  const [selectedFiles, setSelectedFiles] = useState<File[]>([]);
  const [isLaunching, setIsLaunching] = useState(false);
  const [sessionError, setSessionError] = useState<string | null>(null);
  const formRef = useRef<HTMLFormElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const canSend = prompt.trim().length > 0 && !isLaunching;
  const selectedModelRecord = runtimeInfo.models.find((model) => model.id === selectedModel);
  const modelReasoningOptions = selectedModelRecord?.reasoningEfforts ?? [];
  const reasoningOptions =
    modelReasoningOptions.length > 0 ? modelReasoningOptions : runtimeInfo.reasoningEfforts;
  const errorOutput = [sessionError, viewModel.errorOutput]
    .filter((output): output is string => Boolean(output?.trim()))
    .join('\n');

  useEffect(() => {
    setViewModel(agentSessionRouter.emptyViewModel());
  }, [agentSessionRouter]);

  useEffect(() => {
    let cancelled = false;

    void loadCodexRuntimeInfo()
      .then((info) => {
        if (cancelled) {
          return;
        }

        setRuntimeInfo(info);
        setSelectedModel(info.configuredModel ?? info.recommendedModel);
        const model = info.models.find(
          (candidate) => candidate.id === (info.configuredModel ?? info.recommendedModel),
        );
        setSelectedReasoning(
          model?.defaultReasoningEffort ??
            (info.reasoningEfforts.includes('high') ? 'high' : info.reasoningEfforts[0]) ??
            'high',
        );
      })
      .catch(() => {
        if (!cancelled) {
          setRuntimeInfo(fallbackRuntimeInfo);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [loadCodexRuntimeInfo]);

  const startSession = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmedPrompt = prompt.trim();

    if (!trimmedPrompt) {
      return;
    }

    void (async () => {
      setIsLaunching(true);
      setSessionError(null);

      try {
        const result = await agentSessionRouter.launch(
          {
            prompt: trimmedPrompt,
            additionalArgs: buildAgentSessionAdditionalArgs({
              model: selectedModel,
              sandbox: selectedSandbox,
              reasoningEffort: selectedReasoning,
            }),
            ...(viewModel.sessionId ? { sessionId: viewModel.sessionId as EntityId } : {}),
          },
          setViewModel,
        );

        setViewModel(result);
        setPrompt('');
      } catch (caught) {
        setSessionError(errorMessage(caught));
      } finally {
        setIsLaunching(false);
      }
    })();
  };

  const closeSession = () => {
    if (!viewModel.sessionId) {
      return;
    }

    void (async () => {
      try {
        setViewModel(await agentSessionRouter.close(viewModel.sessionId as EntityId));
      } catch (caught) {
        setSessionError(errorMessage(caught));
      }
    })();
  };

  const toggleTurn = (turnId: string) => {
    if (!viewModel.sessionId) {
      return;
    }

    try {
      setViewModel(agentSessionRouter.toggleTurn(viewModel.sessionId, turnId));
    } catch (caught) {
      setSessionError(errorMessage(caught));
    }
  };

  return (
    <section className="workspace agent-session-workspace" id="agent-session-view">
      <header className="topbar">
        <div>
          <p className="eyebrow">Tool Viewer</p>
          <h1>Agent Session View</h1>
        </div>
        <div className="status-strip" aria-label="Agent session status">
          <span>{viewModel.statusLabel}</span>
          <button
            className="icon-button"
            type="button"
            onClick={closeSession}
            disabled={!viewModel.sessionId || viewModel.status === 'closed'}
            title="Close agent session"
            aria-label="Close agent session"
          >
            <X size={16} aria-hidden="true" />
          </button>
        </div>
      </header>

      <section className="agent-session-display" aria-label="Agent session output">
        <header>
          <div>
            <p className="eyebrow">Session Display</p>
            <h2>{viewModel.promptText ? 'Conversation' : 'No session running'}</h2>
          </div>
          <div className="agent-session-display-actions">
            {errorOutput && <ErrorOutputIndicator errorOutput={errorOutput} />}
            {viewModel.exitCode !== undefined && (
              <span className={`state-pill ${viewModel.status}`}>exit {viewModel.exitCode}</span>
            )}
          </div>
        </header>
        <AgentSessionMetadataStrip metadata={viewModel.metadata} />
        <div className="agent-session-output conversation" role="log" aria-live="polite">
          {viewModel.items.length === 0 ? (
            <p className="detail-empty">No agent output yet.</p>
          ) : (
            viewModel.items.map((item) => (
              <AgentSessionDisplayItemView item={item} key={item.id} onToggleTurn={toggleTurn} />
            ))
          )}
        </div>
      </section>

      <form
        ref={formRef}
        className="agent-session-composer"
        onSubmit={startSession}
        aria-label="Agent prompt composer"
      >
        <div className="agent-session-prompt-row">
          <textarea
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            onKeyDown={(event) => {
              if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
                event.preventDefault();
                formRef.current?.requestSubmit();
              }
            }}
            disabled={isLaunching}
            placeholder="Agent prompt"
            aria-label="Agent prompt"
            rows={4}
          />
        </div>
        <div className="agent-session-toolbar" aria-label="Agent session settings">
          <button
            className="icon-button"
            type="button"
            onClick={() => fileInputRef.current?.click()}
            title="Upload files"
            aria-label="Upload files"
          >
            <Upload size={15} aria-hidden="true" />
          </button>
          <input
            ref={fileInputRef}
            type="file"
            multiple
            hidden
            onChange={(event) => setSelectedFiles(Array.from(event.target.files ?? []))}
          />
          <label>
            <ShieldCheck size={14} aria-hidden="true" />
            <span>Sandbox</span>
            <select
              value={selectedSandbox}
              onChange={(event) => setSelectedSandbox(event.target.value as AgentSandboxMode)}
              disabled={isLaunching}
            >
              <option value="danger-full-access">Full access</option>
              <option value="workspace-write">Workspace write</option>
              <option value="read-only">Read only</option>
            </select>
          </label>
          <label>
            <span>Model</span>
            <select
              value={selectedModel}
              onChange={(event) => {
                const modelId = event.target.value;
                const model = runtimeInfo.models.find((candidate) => candidate.id === modelId);
                setSelectedModel(modelId);
                setSelectedReasoning(
                  model?.defaultReasoningEffort ??
                    model?.reasoningEfforts?.[0] ??
                    selectedReasoning,
                );
              }}
              disabled={isLaunching}
            >
              {runtimeInfo.models.map((model) => (
                <option value={model.id} key={model.id}>
                  {model.name ?? model.id}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Reasoning</span>
            <select
              value={selectedReasoning}
              onChange={(event) => setSelectedReasoning(event.target.value as CodexReasoningEffort)}
              disabled={isLaunching}
            >
              {reasoningOptions.map((effort) => (
                <option value={effort} key={effort}>
                  {effort}
                </option>
              ))}
            </select>
          </label>
          <span className="agent-session-context-size" title="Context size">
            {viewModel.contextSize}
          </span>
          {selectedFiles.length > 0 && (
            <span className="agent-session-file-count">{selectedFiles.length} files</span>
          )}
          <button
            className="icon-button run-button"
            type="submit"
            disabled={!canSend}
            title="Send message"
            aria-label="Send message"
          >
            {isLaunching ? (
              <LoaderCircle size={16} aria-hidden="true" />
            ) : (
              <Send size={16} aria-hidden="true" />
            )}
          </button>
        </div>
      </form>
    </section>
  );
}

interface AgentSessionDisplayItemViewProps {
  item: AgentSessionDisplayItem;
  onToggleTurn(turnId: string): void;
}

function AgentSessionDisplayItemView({ item, onToggleTurn }: AgentSessionDisplayItemViewProps) {
  if (item.kind === 'finished-turn') {
    return (
      <article className="agent-session-turn-card">
        <button
          className={`agent-session-turn-toggle${item.expanded ? ' expanded' : ''}`}
          type="button"
          onClick={() => onToggleTurn(item.id)}
        >
          <CheckCircle2 size={15} aria-hidden="true" />
          <span>{item.text}</span>
          {item.hiddenItems.length > 0 && (
            <small>{item.expanded ? 'Hide details' : 'Show details'}</small>
          )}
        </button>
        {item.expanded && item.hiddenItems.length > 0 && (
          <div className="agent-session-turn-details">
            {item.hiddenItems.map((hiddenItem) => (
              <AgentSessionDisplayItemView
                item={hiddenItem}
                key={hiddenItem.id}
                onToggleTurn={onToggleTurn}
              />
            ))}
          </div>
        )}
        {item.finalText && (
          <MarkdownText className="agent-session-markdown" text={item.finalText} />
        )}
      </article>
    );
  }

  if (item.kind === 'processing') {
    return (
      <div className="agent-session-event processing">
        <LoaderCircle size={15} aria-hidden="true" />
        <span>{item.text}</span>
      </div>
    );
  }

  if (item.kind === 'agent-message') {
    return (
      <article className="agent-session-message">
        <MarkdownText text={item.text} />
      </article>
    );
  }

  if (item.kind === 'user-message') {
    return (
      <article className="agent-session-message user">
        <span>You</span>
        <pre>{item.text}</pre>
      </article>
    );
  }

  if (item.kind === 'item') {
    return (
      <div className={`agent-session-event item ${item.processing ? 'processing' : 'complete'}`}>
        {item.processing ? (
          <LoaderCircle size={15} aria-hidden="true" />
        ) : (
          <Check size={15} aria-hidden="true" />
        )}
        <span>{item.text}</span>
        <small>{item.itemType}</small>
      </div>
    );
  }

  if (item.kind === 'diagnostic') {
    return (
      <div className="agent-session-event diagnostic">
        <AlertCircle size={15} aria-hidden="true" />
        <span>{item.text}</span>
      </div>
    );
  }

  return null;
}

function MarkdownText({ text, className }: { text: string; className?: string }) {
  return (
    <div className={className ?? 'agent-session-markdown'}>
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{text}</ReactMarkdown>
    </div>
  );
}

interface AgentSessionMetadataStripProps {
  metadata: AgentSessionViewModel['metadata'];
}

function AgentSessionMetadataStrip({ metadata }: AgentSessionMetadataStripProps) {
  const fields = [
    ['Model', metadata.model],
    ['Approval', metadata.approval],
    ['Sandbox', metadata.sandbox],
    ['Reasoning', metadata.reasoningEffort],
    ['Summaries', metadata.reasoningSummaries],
  ].filter((field): field is [string, string] => Boolean(field[1]));

  if (fields.length === 0) {
    return null;
  }

  return (
    <dl className="agent-session-metadata" aria-label="Agent session metadata">
      {fields.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd title={value}>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function buildAgentSessionAdditionalArgs(settings: {
  model: string;
  sandbox: AgentSandboxMode;
  reasoningEffort: CodexReasoningEffort;
}): string[] {
  return [
    '--model',
    settings.model,
    '--sandbox',
    settings.sandbox,
    '--reasoning-effort',
    settings.reasoningEffort,
  ];
}
