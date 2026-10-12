import type { AgentInvocationDto, AgentInvocationStatusDto } from './contracts';
export type JevQuestion =
  | { type: 'choice'; instructions: unknown; criteria: Record<string, unknown> }
  | { type: 'score'; instructions: unknown; criteria: unknown[] }
  | { type: 'noul'; instructions: unknown; criteria?: { true?: unknown; false?: unknown } };
export type JevAnswer =
  | { type: 'choice'; choice: string; probabilities: Record<string, number>; confidence: number }
  | {
      type: 'score';
      score: number;
      legend: Record<string, unknown>;
      probabilities: Record<string, number>;
      confidence: number;
    }
  | { type: 'noul'; noul: number };
export interface JevQueryDetail {
  invocation: AgentInvocationDto;
  evaluation: { state: unknown; questions: Record<string, JevQuestion> };
  result: {
    model: string;
    answers: Record<string, JevAnswer>;
    usage: { input_tokens: number; output_tokens: number };
  } | null;
}
export interface JevQuerySummary {
  invocationId: string;
  status: AgentInvocationStatusDto;
  createdAt: string;
  startedAt: string | null;
  completedAt: string | null;
  inputPreview: string;
  questionCount: number;
  answers: string[];
}
export interface JevQueryClient {
  list(sessionId: string): Promise<JevQuerySummary[]>;
  load(sessionId: string, invocationId: string): Promise<JevQueryDetail>;
}
export interface AppliedJevQuery {
  invocationId: string;
  releaseId: string | null;
  decisions: {
    sentenceId: string;
    revision: number;
    currentRevision: number;
    applied: boolean;
    relevance: string | null;
    relevanceRetained: boolean;
    completion: string | null;
    reaction: string | null;
    buffered: boolean;
  }[];
}
