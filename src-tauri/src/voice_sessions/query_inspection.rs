//! Voice decisions link to provider-owned immutable evaluations; no duplicate query store.
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppliedJevQuery {
    pub(crate) invocation_id: String,
    pub(crate) decisions: Vec<SentenceDecision>,
    pub(crate) release_id: Option<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SentenceDecision {
    pub(crate) sentence_id: String,
    pub(crate) revision: u64,
    pub(crate) current_revision: u64,
    pub(crate) applied: bool,
    pub(crate) relevance: Option<String>,
    pub(crate) relevance_retained: bool,
    pub(crate) completion: Option<String>,
    pub(crate) reaction: Option<String>,
    pub(crate) buffered: bool,
}

pub(super) fn sentence_decision(
    input: &super::domain::SpokenInput,
    evaluated_revision: u64,
    returned_relevance: Option<&str>,
    buffered: bool,
) -> SentenceDecision {
    let applied = input.revision == evaluated_revision;
    SentenceDecision {
        sentence_id: input.id.clone(),
        revision: evaluated_revision,
        current_revision: input.revision,
        applied,
        relevance: if applied {
            input.relevance.clone()
        } else {
            None
        },
        relevance_retained: applied && input.relevance.as_deref() != returned_relevance,
        completion: if applied {
            input.completion.clone()
        } else {
            None
        },
        reaction: if applied {
            input.reaction.clone()
        } else {
            None
        },
        buffered: applied && buffered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice_sessions::domain::SpokenInput;
    #[test]
    fn applied_evidence_keeps_effective_relevance_separate_from_provider_choice() {
        let input = SpokenInput {
            id: "sentence".into(),
            revision: 2,
            relevance: Some("task_context".into()),
            completion: Some("continue".into()),
            ..Default::default()
        };
        let decision = sentence_decision(&input, 2, Some("unaddressed"), true);
        assert!(decision.applied && decision.relevance_retained && decision.buffered);
        assert_eq!(decision.relevance.as_deref(), Some("task_context"));
        let stored: SentenceDecision =
            serde_json::from_value(serde_json::to_value(&decision).unwrap()).unwrap();
        assert_eq!(stored.revision, 2);
        assert!(stored.relevance_retained);
    }
    #[test]
    fn superseded_evaluation_does_not_claim_to_apply_a_newer_sentence() {
        let input = SpokenInput {
            revision: 3,
            relevance: Some("direct_request".into()),
            ..Default::default()
        };
        let decision = sentence_decision(&input, 2, Some("unaddressed"), true);
        assert!(!decision.applied && !decision.buffered && !decision.relevance_retained);
        assert!(decision.relevance.is_none());
        assert_eq!((decision.revision, decision.current_revision), (2, 3));
    }
}
