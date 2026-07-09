import { AlertCircle, Check, Copy } from 'lucide-react';
import { useState } from 'react';
import { browserClipboardHelper, type ClipboardWriter } from '../infrastructure/clipboardHelper';

export interface ErrorOutputIndicatorProps {
  errorOutput: string;
  label?: string;
  clipboardWriter?: ClipboardWriter;
}

export function ErrorOutputIndicator({
  errorOutput,
  label = 'Error output',
  clipboardWriter = browserClipboardHelper,
}: ErrorOutputIndicatorProps) {
  const [copyState, setCopyState] = useState<'idle' | 'copied' | 'failed'>('idle');

  if (!errorOutput.trim()) {
    return null;
  }

  const copyErrorOutput = () => {
    void (async () => {
      try {
        await clipboardWriter.writeText(errorOutput);
        setCopyState('copied');
        window.setTimeout(() => setCopyState('idle'), 1600);
      } catch {
        setCopyState('failed');
      }
    })();
  };

  const statusLabel =
    copyState === 'copied'
      ? 'Copied error output'
      : copyState === 'failed'
        ? 'Could not copy error output'
        : 'Copy error output';

  return (
    <div className="error-output-indicator">
      <button
        className="error-output-indicator-button"
        type="button"
        onClick={copyErrorOutput}
        aria-label={statusLabel}
      >
        <AlertCircle size={16} aria-hidden="true" />
        {copyState === 'copied' ? (
          <Check size={13} aria-hidden="true" />
        ) : (
          <Copy size={13} aria-hidden="true" />
        )}
      </button>
      <div className="error-output-tooltip" role="tooltip">
        <strong>{label}</strong>
        <pre>{errorOutput}</pre>
        <span>{statusLabel}</span>
      </div>
    </div>
  );
}
