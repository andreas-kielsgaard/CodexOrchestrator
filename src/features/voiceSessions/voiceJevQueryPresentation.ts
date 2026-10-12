import type { JevQueryDetail } from '../../application/agentSessions/jevQueries';
/** Only Voice knows the sentence identities in its submitted context. */
export function voiceQuestionSubject(
  detail: JevQueryDetail,
  questionId: string,
): string | undefined {
  const instructions = detail.evaluation.questions[questionId]?.instructions;
  const state = detail.evaluation.state;
  if (
    !instructions ||
    typeof instructions !== 'object' ||
    !('sentenceId' in instructions) ||
    !state ||
    typeof state !== 'object' ||
    !('recentSpeech' in state) ||
    !Array.isArray(state.recentSpeech)
  )
    return;
  const sentence = state.recentSpeech.find(
    (value) => value && typeof value === 'object' && value.id === instructions.sentenceId,
  );
  return sentence && typeof sentence.text === 'string'
    ? `${sentence.text} · r${sentence.revision}`
    : undefined;
}
