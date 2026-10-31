use super::{domain::*, VoiceSessionApplication};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn recent_inputs(c: &Conversation, seconds: u64) -> Vec<SpokenInput> {
    let cutoff = chrono::Utc::now() - chrono::Duration::seconds(seconds as i64);
    c.inputs
        .iter()
        .filter(|i| {
            !i.text.trim().is_empty()
                && (i.evaluation_failed
                    || chrono::DateTime::parse_from_rfc3339(&i.created_at)
                        .is_ok_and(|at| at >= cutoff))
        })
        .cloned()
        .collect()
}
pub(super) fn recent_release(c: &Conversation, seconds: u64) -> Option<&InputRelease> {
    c.last_input_release.as_ref().filter(|release| {
        chrono::DateTime::parse_from_rfc3339(&release.created_at)
            .is_ok_and(|at| at >= chrono::Utc::now() - chrono::Duration::seconds(seconds as i64))
    })
}
fn sticky_relevance(previous: Option<&str>, next: &str) -> String {
    match (previous, next) {
        (Some("direct_request"), _) | (_, "direct_request") => "direct_request",
        (Some("task_context"), _) | (_, "task_context") => "task_context",
        _ => "unaddressed",
    }
    .into()
}
pub(super) fn prepare_window(c: &mut Conversation, seconds: u64) {
    let recent = recent_inputs(c, seconds);
    for input in &mut c.inputs {
        input.filter_pending = recent.iter().any(|r| r.id == input.id);
    }
    if !recent.is_empty() {
        c.input_buffer.complete = false;
    }
}

fn may_accumulate(input: &SpokenInput, continuation: bool) -> bool {
    input.delivered_revision == 0 || (continuation && input.revision > input.delivered_revision)
}
pub(super) fn ready(state: &VoiceState, c: &Conversation) -> bool {
    state.enabled
        && !c.inputs.iter().any(|i| i.filter_pending)
        && c.input_buffer.complete
        && !c.input_buffer.blocks.is_empty()
}
impl VoiceSessionApplication {
    pub(super) fn accept_evaluation(
        self: &Arc<Self>,
        payload: Value,
        direct_reactions: bool,
    ) -> Result<Value, String> {
        let evaluated: Conversation =
            serde_json::from_value(payload["conversation"].clone()).map_err(|e| e.to_string())?;
        let recent: Vec<SpokenInput> =
            serde_json::from_value(payload["recent"].clone()).map_err(|e| e.to_string())?;
        let evaluation_id = payload["evaluationId"]
            .as_str()
            .ok_or("Missing evaluation identity")?
            .to_string();
        let answers: BTreeMap<String, String> =
            serde_json::from_value(payload["answers"].clone()).map_err(|e| e.to_string())?;
        let policy = self.snapshot()?.input_policy;
        let (opened, relevant, classifications, continued) = self.update(|state| {
            if !state.enabled {
                return Err("Voice connection stopped during filtering".into());
            }
            let c = state.conversation.as_mut().ok_or("No conversation")?;
            if c.id != evaluated.id {
                return Err("Conversation changed during filtering".into());
            }
            state.filter_error = None;
            let was_empty = c.input_buffer.blocks.is_empty();
            let mut relevant = vec![];
            let mut classifications = vec![];
            let mut continued = false;
            let mut decisions = vec![];
            for block in &recent {
                let input = c
                    .inputs
                    .iter_mut()
                    .find(|i| i.id == block.id)
                    .ok_or("Sentence missing")?;
                if input.revision != block.revision {
                    input.filter_pending = true;
                    decisions.push(super::query_inspection::sentence_decision(input, block.revision, None, false));
                    continue;
                }
                input.evaluation_failed = false;
                let relevance = answers
                    .get(&format!("relevance:{}", block.id))
                    .ok_or("Relevance answer missing")?;
                let effective = sticky_relevance(input.relevance.as_deref(), relevance);
                let accepted = effective != "unaddressed";
                let revised = input.revision > input.evaluated_revision;
                let continuation = answers
                    .get(&format!("continuation:{}", block.id))
                    .is_some_and(|v| v == "continued");
                let eligible = accepted && may_accumulate(input, continuation);
                if eligible
                    && (revised
                        || input.relevance.as_deref() == Some("unaddressed")
                        || input.relevance.is_none())
                {
                    relevant.push(input.id.clone());
                }
                input.relevance = Some(effective.clone());
                input.reaction = Some(answers.get(&format!("reaction:{}", block.id)).ok_or("Reaction answer missing")?.clone());
                input.evaluated_revision = block.revision;
                input.filter_pending = input.revision != block.revision;
                input.completion = if input.filter_pending {
                    None
                } else {
                    Some(
                        answers
                            .get(&format!("completion:{}", block.id))
                            .ok_or("Completion answer missing")?
                            .clone(),
                    )
                };
                decisions.push(super::query_inspection::sentence_decision(input, block.revision, Some(relevance), eligible));
                classifications
                    .push(json!({"id":input.id,"relevance":effective,"revision":input.revision,
                        "addressed":relevance != "unaddressed",
                        "sentenceComplete":super::query_schedule::sentence_complete(&input.text),"reaction":input.reaction}));
                if eligible {
                    continued |= continuation && revised;
                    if let Some(existing) =
                        c.input_buffer.blocks.iter_mut().find(|b| b.id == input.id)
                    {
                        *existing = input.clone();
                    } else {
                        c.input_buffer.blocks.push(input.clone());
                    }
                }
            }
            if continued {
                c.input_buffer.continuation_of =
                    recent_release(&evaluated, policy.followup_seconds).map(|r| r.id.clone());
            }
            c.input_buffer
                .blocks
                .sort_by(|a, b| a.created_at.cmp(&b.created_at));
            c.input_buffer.complete = c
                .input_buffer
                .blocks
                .last()
                .is_some_and(|b| b.completion.as_deref() == Some("complete"));
            c.jev_applications.push(super::query_inspection::AppliedJevQuery { invocation_id: evaluation_id.clone(), decisions, release_id: None });
            Ok((
                was_empty && !c.input_buffer.blocks.is_empty(),
                relevant,
                classifications,
                continued,
            ))
        })?;
        for value in classifications {
            let mut event = value;
            event["type"] = "input_classification".into();
            let _ = self.events.send(event);
        }
        if opened {
            let _ = self.events.send(json!({"type":"cue","cue":"listening"}));
        }
        for id in relevant {
            let _ = self.events.send(json!({"type":"relevant_input","id":id}));
        }
        if continued {
            let _ = self.events.send(json!({"type":"input_continuation"}));
        }
        self.release_input(direct_reactions)
    }
    pub(crate) fn speech_activity(self: &Arc<Self>, speaking: bool) -> Result<(), String> {
        self.transient(|s| s.speaking = speaking)
    }
    pub(crate) fn audio_pending(&self, delta: isize) -> Result<(), String> {
        self.transient(|s| s.pending_audio = s.pending_audio.saturating_add_signed(delta))
    }
    pub(crate) fn release_input(self: &Arc<Self>, direct_reactions: bool) -> Result<Value, String> {
        let captured = self.update(|state| {
            let Some(c) = &state.conversation else {
                return Ok(None);
            };
            if !ready(state, c) {
                return Ok(None);
            }
            let policy = state.input_policy.clone();
            let c = state.conversation.as_mut().unwrap();
            let buffer = std::mem::take(&mut c.input_buffer);
            let previous = recent_release(c, policy.followup_seconds).cloned();
            for block in &buffer.blocks {
                if let Some(input) = c.inputs.iter_mut().find(|i| i.id == block.id) {
                    input.delivered_revision = input.revision;
                }
            }
            let release = InputRelease {
                id: uuid::Uuid::new_v4().to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                blocks: buffer.blocks.clone(),
            };
            c.last_input_release = Some(release.clone());
            for block in &buffer.blocks {
                if let Some(application) = c.jev_applications.iter_mut().rev().find(|a| {
                    a.decisions.iter().any(|d| {
                        d.applied && d.sentence_id == block.id && d.revision == block.revision
                    })
                }) {
                    application.release_id = Some(release.id.clone());
                }
            }
            Ok(Some((
                c.clone(),
                release,
                previous,
                policy,
                buffer.continuation_of,
            )))
        })?;
        let Some((_c, release, previous, policy, continuation_of)) = captured else {
            return Ok(Value::Null);
        };
        let reaction = direct_reactions
            && release
                .blocks
                .iter()
                .all(|b| b.reaction.as_deref() == Some("reaction_request"));
        let payload = json!({"requestId":release.id,"blocks":release.blocks,"previousRequest":previous,"continuationOf":continuation_of,
            "responseRoute":if reaction {"direct_reaction"} else {"work_routing"}});
        self.record_activity("user", "accepted_input", payload.clone(), &[])?;
        let _ = self.events.send(json!({"type":"cue","cue":"processing"}));
        let _ = self.events.send(json!({"type":"input_released","requestId":release.id,"followupSeconds":policy.followup_seconds}));
        Ok(payload)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_evaluation_retains_the_opening_after_the_rolling_window_expires() {
        let mut c = Conversation::default();
        c.inputs.push(SpokenInput {
            id: "opening".into(),
            text: "Orchid, investigate".into(),
            created_at: (chrono::Utc::now() - chrono::Duration::seconds(82)).to_rfc3339(),
            evaluation_failed: true,
            ..Default::default()
        });
        c.inputs.push(SpokenInput {
            id: "continuation".into(),
            text: "the routing, that's all".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            ..Default::default()
        });
        assert_eq!(recent_inputs(&c, 60).len(), 2);
        c.inputs[0].evaluation_failed = false;
        assert_eq!(recent_inputs(&c, 60).len(), 1);
    }
    #[test]
    fn rolling_queries_keep_the_beginning_of_an_unfinished_request() {
        let first = SpokenInput {
            id: "first".into(),
            text: "Orchid, investigate this long request.".into(),
            created_at: (chrono::Utc::now() - chrono::Duration::seconds(90)).to_rfc3339(),
            relevance: Some("direct_request".into()),
            completion: Some("continue".into()),
            ..Default::default()
        };
        let latest = SpokenInput {
            id: "latest".into(),
            text: "Here is the final detail.".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            ..Default::default()
        };
        let mut c = Conversation {
            inputs: vec![first.clone(), latest],
            input_buffer: InputBuffer {
                blocks: vec![first],
                ..Default::default()
            },
            ..Default::default()
        };
        prepare_window(&mut c, 60);
        assert_eq!(recent_inputs(&c, 60).len(), 1);
        assert_eq!(c.input_buffer.blocks[0].id, "first");
        assert!(!c.inputs[0].filter_pending);
        assert!(c.inputs[1].filter_pending);
    }
    #[test]
    fn delivered_revision_requires_a_confirmed_continuation() {
        let mut input = SpokenInput {
            revision: 2,
            delivered_revision: 1,
            ..Default::default()
        };
        assert!(!may_accumulate(&input, false));
        assert!(may_accumulate(&input, true));
        input.delivered_revision = 2;
        assert!(!may_accumulate(&input, true));
        input.delivered_revision = 0;
        assert!(may_accumulate(&input, false));
    }
    #[test]
    fn relevance_is_sticky_and_can_upgrade() {
        assert_eq!(
            sticky_relevance(Some("direct_request"), "unaddressed"),
            "direct_request"
        );
        assert_eq!(
            sticky_relevance(Some("task_context"), "unaddressed"),
            "task_context"
        );
        assert_eq!(
            sticky_relevance(Some("task_context"), "direct_request"),
            "direct_request"
        );
    }
    #[test]
    fn completion_releases_during_capture_but_not_against_unevaluated_revisions() {
        let state = VoiceState {
            enabled: true,
            speaking: true,
            pending_audio: 1,
            ..Default::default()
        };
        let block = SpokenInput {
            id: "sentence".into(),
            text: "Orchid, what time is it?".into(),
            ..Default::default()
        };
        let mut c = Conversation {
            input_buffer: InputBuffer {
                blocks: vec![block.clone()],
                complete: true,
                ..Default::default()
            },
            inputs: vec![block],
            ..Default::default()
        };
        assert!(ready(&state, &c));
        c.inputs[0].filter_pending = true;
        assert!(!ready(&state, &c));
    }
    #[test]
    fn query_window_uses_time_not_a_fixed_number_of_blocks() {
        let mut c = Conversation::default();
        for index in 0..12 {
            c.inputs.push(SpokenInput {
                id: index.to_string(),
                text: "Sentence.".into(),
                created_at: (chrono::Utc::now() - chrono::Duration::seconds(index * 5))
                    .to_rfc3339(),
                ..Default::default()
            });
        }
        assert_eq!(recent_inputs(&c, 60).len(), 12);
        assert_eq!(recent_inputs(&c, 20).len(), 4);
    }
}
