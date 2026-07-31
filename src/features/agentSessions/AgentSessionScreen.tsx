import { AlertCircle, ArrowUpRight, ChevronDown, X } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import type { AgentSessionClient } from '../../application/agentSessions';
import {
  buildAgentSessionNavigation,
  type AgentSessionNavigationIdentity,
  type AgentSessionProductLocation,
} from '../../application/agentSessionNavigation';
import type {
  EpicPlanningDraftSummary,
  ProductReadModelsV1,
} from '../../application/orchestrations';
import { AgentSessionWorkspace } from './AgentSessionWorkspace';
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
  const navigation = useMemo(
    () =>
      buildAgentSessionNavigation({
        summaries: collection.summaries,
        ...(orchestrations ? { orchestrations } : {}),
        ...(planningDrafts ? { planningDrafts } : {}),
        ...(identities ? { identities } : {}),
      }),
    [collection.summaries, identities, orchestrations, planningDrafts],
  );
  const selectedNavigation = collection.selectedSessionId
    ? navigation.sessions.get(collection.selectedSessionId)
    : undefined;

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
            {collection.error && (
              <section className="agent-session-error" role="alert">
                <AlertCircle size={17} aria-hidden="true" />
                <span>{collection.error}</span>
                <button type="button" onClick={collection.clearError} aria-label="Dismiss error">
                  <X size={15} aria-hidden="true" />
                </button>
              </section>
            )}
            <AgentSessionWorkspace
              controller={session}
              headerActions={
                selectedNavigation && onNavigateToProduct ? (
                  <SessionProductNavigation
                    locations={selectedNavigation.productLocations}
                    onNavigate={onNavigateToProduct}
                  />
                ) : undefined
              }
            />
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

function locationKindLabel(location: AgentSessionProductLocation) {
  return {
    epic: 'Epic',
    sprint: 'Sprint',
    work_slice_planning_point: 'Planning',
    work_unit: 'Work Unit',
    epic_planning_draft: 'Epic planning draft',
  }[location.kind];
}

function locationKey(location: AgentSessionProductLocation) {
  return JSON.stringify(location);
}

export const AgentSessionScreen = StandaloneAgentSessionScreen;
