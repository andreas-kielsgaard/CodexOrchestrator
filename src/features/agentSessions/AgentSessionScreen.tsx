import type { AgentSessionImportClient } from '../../application/agentSessions/importContracts';
import { ImportCodexSessionDialog } from './ImportCodexSessionDialog';
import type { SessionWorkflowTarget } from '../../application/agentSessions/workflowNavigation';
import type { RepositoryBranchSource } from '../../application/branches';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type { ExecutionTargetClient } from '../../application/executionTargets/contracts';
import { useSessionTarget } from './useSessionTarget';
import { PerMessageRuntimeControls } from './PerMessageRuntimeControls';
import { legacyHarnessRoleLabel } from '../../application/identities/legacyAgentIdentityAdapter';
import type {
  SessionNavigationCommandRequest,
  SessionNavigationState,
} from '../../application/agentSessions/agentAccess';
import { useSessionNavigation } from './useSessionNavigation';
import { useSessionNavigationCommands } from './useSessionNavigationCommands';
import { AlertCircle, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  AgentIdentity,
  AgentSessionClient,
  AgentSessionProfileClient,
} from '../../application/agentSessions';
import type { ConversationHarnessManagementSource } from '../../application/conversationHarnesses';
import type { SessionEventQueryClient } from '../../application/sessionEvents';
import type { ProductDecisionEvidenceDestination } from '../../application/productDecisions';
import {
  buildSessionNavigation,
  selectionKey,
  sessionIdOf,
  type SessionNavigationSelection,
} from '../../application/agentSessions/navigation';
import type {
  SessionNavigationClient,
  SessionFolderTarget,
} from '../../application/agentSessions/organization';
import type { TranscriptAnchorRange } from './transcriptProjector';
import { AgentSessionHeaderActionsProvider, AgentSessionWorkspace } from './AgentSessionWorkspace';
import { AgentSessionExecutionSettings } from './AgentSessionExecutionSettings';
import { HarnessAwareAgentSessionPane } from '../conversationHarnesses/HarnessAwareAgentSessionPane';
import { SessionSelector } from './SessionSelector';
import { useAgentSessionCollection } from './useAgentSessionCollection';
import { useProfiledAgentSession } from './useProfiledAgentSession';
import { ResizableSplitSurface } from '../orchestrations/components/ResizableSplitSurface';
import './agentSession.css';
export interface AgentSessionScreenProps {
  executionTargetClient,
  executionConfigurationClient,
  branchSource,
