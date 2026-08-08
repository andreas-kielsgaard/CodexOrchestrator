import { AlertCircle, ArrowUpRight, ChevronDown, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import type { AgentIdentity, AgentSessionClient } from '../../application/agentSessions';
import type { ConversationHarnessManagementSource } from '../../application/conversationHarnesses';
import {
  buildAgentSessionNavigation,
  type AgentSessionNavigationIdentity,
  type AgentSessionProductLocation,
  type AgentSessionProductOrigin,
} from '../../application/agentSessionNavigation';
import type {
  EpicPlanningDraftSummary,
  ProductReadModelsV1,
} from '../../application/orchestrations';
import type { ProductDecisionEvidenceDestination } from '../../application/productDecisions';
import type { TranscriptAnchorRange } from './transcriptProjector';
import { AgentSessionWorkspace } from './AgentSessionWorkspace';
import { HarnessAwareAgentSessionPane } from '../conversationHarnesses/HarnessAwareAgentSessionPane';
import { SessionSelector } from './SessionSelector';
import { useAgentSession, useAgentSessionCollection } from './useAgentSessionController';
import { ResizableSplitSurface } from '../orchestrations/components/ResizableSplitSurface';
import './agentSession.css';

export interface AgentSessionScreenProps {
  readonly client: AgentSessionClient;
  readonly orchestrations?: ProductReadModelsV1;
  readonly planningDrafts?: readonly EpicPlanningDraftSummary[];
  readonly identities?: readonly AgentSessionNavigationIdentity[];
  readonly selectedSessionId?: string | null;
  readonly onSelectedSessionChange?: (sessionId: string | null) => void;
  readonly expandedNodeIds?: ReadonlySet<string>;
  readonly onExpandedNodeIdsChange?: (ids: ReadonlySet<string>) => void;
  readonly onNavigateToProduct?: (location: AgentSessionProductLocation) => void;
  readonly harnessManagementSource?: ConversationHarnessManagementSource;
  readonly agentIdentityForSession?: (sessionId: string) => AgentIdentity | undefined;
  readonly focusInvocationId?: string;
  readonly focusEvidence?: ProductDecisionEvidenceDestination;
  readonly returnOrigin?: AgentSessionProductOrigin | null;
}

export function StandaloneAgentSessionScreen({
  client,
  orchestrations,
  planningDrafts,
  identities,
  selectedSessionId,
  onSelectedSessionChange,
  expandedNodeIds,
  onExpandedNodeIdsChange,
  onNavigateToProduct,
  harnessManagementSource,
  agentIdentityForSession,
  focusInvocationId,
  focusEvidence,
  returnOrigin,
}: AgentSessionScreenProps) {
  const [localExpandedNodeIds, setLocalExpandedNodeIds] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const expansion = expandedNodeIds ?? localExpandedNodeIds;
  const updateExpansion = onExpandedNodeIdsChange ?? setLocalExpandedNodeIds;
  const collectionOptions = useMemo(
    () => ({ selectedSessionId, onSelectedSessionChange }),
    [onSelectedSessionChange, selectedSessionId],
  );
  const collection = useAgentSessionCollection(client, collectionOptions);
  const onCreated = useCallback(
    (id: string) => {
      onSelectedSessionChange?.(id);
      void collection.selectSession(id).then(() => collection.reload());
    },
    [collection, onSelectedSessionChange],
  );
  const session = useAgentSession(client, {
    selectedSessionId: collection.selectedSessionId,
    onSessionCreated: onCreated,
  });
  const sessionIdentities = useMemo(
    () =>
      identities ??
      collection.summaries.flatMap((summary) => {
        const identity = agentIdentityForSession?.(summary.id);
        return identity
          ? [
              {
                sessionId: summary.id,
                agentName: identity.name,
                harnessRole: identity.harnessRole,
              },
            ]
          : [];
      }),
    [agentIdentityForSession, collection.summaries, identities],
  );
  const navigation = useMemo(
    () =>
      buildAgentSessionNavigation({
        summaries: collection.summaries,
        ...(orchestrations ? { orchestrations } : {}),
        ...(planningDrafts ? { planningDrafts } : {}),
        ...(sessionIdentities.length > 0 ? { identities: sessionIdentities } : {}),
      }),
    [collection.summaries, orchestrations, planningDrafts, sessionIdentities],
  );
  const selectedNavigation = collection.selectedSessionId
    ? navigation.sessions.get(collection.selectedSessionId)
    : undefined;
  const selectedIdentity = collection.selectedSessionId
    ? agentIdentityForSession?.(collection.selectedSessionId)
    : undefined;
  const focusedInvocationId =
    collection.selectedSessionId === returnOrigin?.sessionId ? focusInvocationId : undefined;
  const evidenceRange =
    focusedInvocationId &&
    focusEvidence &&
    collection.selectedSessionId === focusEvidence.sessionId &&
    focusEvidence.invocationId === focusedInvocationId
      ? evidenceTranscriptRange(focusEvidence)
      : undefined;
  useEffect(() => {
    if (!focusedInvocationId || collection.selectedSessionId !== returnOrigin?.sessionId) return;
    const element = document.querySelector<HTMLElement>(
      `[data-invocation-id="${CSS.escape(focusedInvocationId)}"]`,
    );
    element?.focus();
    element?.scrollIntoView({ block: 'center' });
  }, [
    collection.selectedSessionId,
    focusedInvocationId,
    session.transcript,
    returnOrigin?.sessionId,
  ]);

  return (
    <main className="agent-session-screen">
      <ResizableSplitSurface
        axis="horizontal"
        primaryLabel="Agent Session navigation"
        secondaryLabel="Selected Agent Session"
        initialPrimaryPercent={25}
        minimumPrimaryPixels={240}
        minimumSecondaryPixels={480}
        compactBreakpoint={860}
        primary={
          <SessionSelector
            model={navigation}
            selectedSessionId={collection.selectedSessionId}
            expandedNodeIds={expansion}
            loading={collection.loading}
            onExpandedNodeIdsChange={updateExpansion}
            onSelect={(id) => void collection.selectSession(id)}
            onNew={collection.startNewSession}
            onReload={() => void collection.reload()}
          />
        }
        secondary={
          <div className="agent-session-content">
            {returnOrigin ? (
              <div
                className="agent-session-return-context"
                role="region"
                aria-label={returnContextLabel(returnOrigin.location)}
              >
                <span>{returnContextText(returnOrigin.location)}</span>
              </div>
            ) : null}
            {collection.error && (
              <section className="agent-session-error" role="alert">
                <AlertCircle size={17} aria-hidden="true" />
                <span>{collection.error}</span>
                <button type="button" onClick={collection.clearError} aria-label="Dismiss error">
                  <X size={15} aria-hidden="true" />
                </button>
              </section>
            )}
            {collection.selectedSessionId && harnessManagementSource ? (
              <HarnessAwareAgentSessionPane
                sessionId={collection.selectedSessionId}
                source={harnessManagementSource}
              >
                <AgentSessionWorkspace
                  controller={session}
                  transcriptRange={evidenceRange}
                  inspection={
                    focusEvidence
                      ? {
                          sessionId: focusEvidence.sessionId,
                          invocationId: focusEvidence.invocationId,
                        }
                      : undefined
                  }
                  presentation={
                    selectedIdentity
                      ? {
                          identityHeader: {
                            agentIdentity: selectedIdentity,
                            title: selectedIdentity.harnessRole
                              .split('_')
                              .filter(Boolean)
                              .map(
                                (part) => `${part.charAt(0).toLocaleUpperCase()}${part.slice(1)}`,
                              )
                              .join(' '),
                          },
                        }
                      : undefined
                  }
                />
              </HarnessAwareAgentSessionPane>
            ) : (
              <AgentSessionWorkspace
                controller={session}
                transcriptRange={evidenceRange}
                inspection={
                  focusEvidence
                    ? {
                        sessionId: focusEvidence.sessionId,
                        invocationId: focusEvidence.invocationId,
                      }
                    : undefined
                }
              />
            )}
            {selectedNavigation && onNavigateToProduct ? (
              <SessionProductNavigation
                locations={selectedNavigation.productLocations}
                onNavigate={onNavigateToProduct}
              />
            ) : null}
          </div>
        }
      />
    </main>
  );
}

function SessionProductNavigation({
  locations,
  onNavigate,
}: {
  readonly locations: readonly AgentSessionProductLocation[];
  readonly onNavigate: (location: AgentSessionProductLocation) => void;
}) {
  if (locations.length === 0) return null;
  if (locations.length === 1)
    return (
      <button
        className="agent-session-product-link"
        type="button"
        onClick={() => onNavigate(locations[0])}
      >
        <ArrowUpRight size={15} aria-hidden="true" />
        {directActionLabel(locations[0])}
      </button>
    );
  return (
    <details className="agent-session-product-menu">
      <summary>
        Related product views
        <ChevronDown size={14} aria-hidden="true" />
      </summary>
      <div>
        {locations.map((location) => (
          <button type="button" key={locationKey(location)} onClick={() => onNavigate(location)}>
            <span>{locationKindLabel(location)}</span>
            <strong>{location.label}</strong>
          </button>
        ))}
      </div>
    </details>
  );
}

function directActionLabel(location: AgentSessionProductLocation) {
  if (location.kind === 'epic_planning_draft') return 'Go to Epic planning draft';
  if (location.kind === 'work_slice_planning_point') return 'Go to planning view';
  if (location.kind === 'work_unit') return 'Go to Work Unit';
  return `Go to ${location.kind === 'epic' ? 'Epic' : 'Sprint'}`;
}

function returnContextLabel(location: AgentSessionProductLocation) {
  return `${locationKindLabel(location)} return context`;
}

function returnContextText(location: AgentSessionProductLocation) {
  return `Opened from ${locationKindLabel(location)}`;
}

function locationKindLabel(location: AgentSessionProductLocation) {
  return {
    epic: 'Epic',
    epic_product_decisions: 'Product decisions',
    sprint: 'Sprint',
    work_slice_planning_point: 'Planning',
    work_unit: 'Work Unit',
    epic_planning_draft: 'Epic planning draft',
  }[location.kind];
}

function evidenceTranscriptRange(
  destination: ProductDecisionEvidenceDestination,
): TranscriptAnchorRange {
  const anchor = {
    sessionId: destination.sessionId,
    invocationId: destination.invocationId,
    kind: destination.passage.kind,
    ...('runtimeEventId' in destination.passage
      ? { runtimeEventId: destination.passage.runtimeEventId }
      : {}),
  } as TranscriptAnchorRange['start'];
  return { start: anchor, end: anchor };
}

function locationKey(location: AgentSessionProductLocation) {
  return JSON.stringify(location);
}

export const AgentSessionScreen = StandaloneAgentSessionScreen;
