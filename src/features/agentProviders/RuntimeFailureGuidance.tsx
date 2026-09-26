import type { ComponentType } from 'react';
import type { AgentRuntimeFailureDto } from '../../application/agentSessions';
import { CodexHomeGuidance } from './codex/CodexHomeGuidance';

interface FailureGuidanceProps {
  readonly failure: AgentRuntimeFailureDto | null;
}

/** Explicit provider composition. Each provider's guidance renders only for its own failures. */
const FAILURE_GUIDANCE: readonly ComponentType<FailureGuidanceProps>[] = [CodexHomeGuidance];

export function RuntimeFailureGuidance({ failure }: FailureGuidanceProps) {
  return (
    <>
      {FAILURE_GUIDANCE.map((Guidance, index) => (
        <Guidance key={index} failure={failure} />
      ))}
    </>
  );
}
