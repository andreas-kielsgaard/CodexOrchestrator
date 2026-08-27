import type { HarnessConfiguration } from './configuration';
import type { HarnessVersionRef } from './references';

export interface SessionHarnessOverrideDraft {
  readonly baseHarnessRef: HarnessVersionRef;
  readonly configuration: HarnessConfiguration;
  readonly isDirty: true;
}

export interface SessionHarnessOverrideDraftCache {
  get(sessionId: string): SessionHarnessOverrideDraft | null;
  save(
    sessionId: string,
    baseHarnessRef: HarnessVersionRef,
    configuration: HarnessConfiguration,
  ): SessionHarnessOverrideDraft;
  discard(sessionId: string): boolean;
  clear(): void;
}

/**
 * Process-local edit state for Session-specific Harness customization.
 *
 * A composition root owns the instance. Saving an applied override belongs to the Harness and
 * Agent Session application services; this cache deliberately has no persistence behavior.
 */
export class InMemorySessionHarnessOverrideDraftCache implements SessionHarnessOverrideDraftCache {
  readonly #drafts = new Map<string, SessionHarnessOverrideDraft>();

  get(sessionId: string): SessionHarnessOverrideDraft | null {
    requireSessionId(sessionId);
    const draft = this.#drafts.get(sessionId);
    return draft === undefined ? null : copyDraft(draft);
  }

  save(
    sessionId: string,
    baseHarnessRef: HarnessVersionRef,
    configuration: HarnessConfiguration,
  ): SessionHarnessOverrideDraft {
    requireSessionId(sessionId);
    const stored = copyDraft({
      baseHarnessRef,
      configuration,
      isDirty: true,
    });
    this.#drafts.set(sessionId, stored);
    return copyDraft(stored);
  }

  discard(sessionId: string): boolean {
    requireSessionId(sessionId);
    return this.#drafts.delete(sessionId);
  }

  clear(): void {
    this.#drafts.clear();
  }
}

function requireSessionId(sessionId: string): void {
  if (sessionId.length === 0 || sessionId.trim() !== sessionId) {
    throw new Error('Agent Session ID must be a non-empty string without surrounding whitespace.');
  }
}

function copyDraft(draft: SessionHarnessOverrideDraft): SessionHarnessOverrideDraft {
  return structuredClone(draft);
}
