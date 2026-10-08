use super::{
    continuous_audio::{AudioWindow, Sentence, Transcript},
    domain::*,
    VoiceSessionApplication,
};
use serde_json::json;
use std::{collections::HashSet, sync::Arc};

pub(super) fn reconcile(
    conversation: &mut Conversation,
    window: &AudioWindow,
    sentences: &[Sentence],
) -> bool {
    let evaluating = conversation.inputs.iter().any(|i| i.filter_pending);
    let mut matched = HashSet::new();
    let mut changed = false;
    for sentence in sentences.iter().filter(|s| !s.text.trim().is_empty()) {
        let candidate = conversation
            .inputs
            .iter()
            .enumerate()
            .filter(|(_, input)| {
                input.stream_id.as_deref() == Some(&window.stream_id)
                    && !matched.contains(&input.id)
                    && (input.start_ms.abs_diff(sentence.start_ms) < 1000
                        || (input.start_ms < sentence.end_ms && sentence.start_ms < input.end_ms))
            })
            .min_by_key(|(_, input)| input.start_ms.abs_diff(sentence.start_ms))
            .map(|(index, _)| index);
        let index = candidate.unwrap_or_else(|| {
            let created_at =
                chrono::DateTime::from_timestamp_millis(window.epoch_ms + sentence.start_ms as i64)
                    .unwrap_or_else(chrono::Utc::now)
                    .to_rfc3339();
            conversation.inputs.push(SpokenInput {
                id: uuid::Uuid::new_v4().to_string(),
                stream_id: Some(window.stream_id.clone()),
                created_at,
                ..Default::default()
            });
            conversation.inputs.len() - 1
        });
        let input = &mut conversation.inputs[index];
        matched.insert(input.id.clone());
        if input.text != sentence.text {
            input.text = sentence.text.clone();
            input.revision += 1;
            input.completion = None;
            input.filter_pending = evaluating;
            conversation.input_buffer.complete = false;
            changed = true;
        }
        input.start_ms = sentence.start_ms;
        input.end_ms = sentence.end_ms;
        if let Some(accepted) = conversation
            .input_buffer
            .blocks
            .iter_mut()
            .find(|b| b.id == input.id)
        {
            *accepted = input.clone();
        }
    }
    for input in &mut conversation.inputs {
        if input.stream_id.as_deref() == Some(&window.stream_id)
            && input.start_ms >= window.start_ms
            && input.end_ms <= window.end_ms
            && !matched.contains(&input.id)
            && !input.text.is_empty()
        {
            conversation.input_buffer.complete = false;
            input.text.clear();
            input.revision += 1;
            input.completion = None;
            input.filter_pending = false;
            changed = true;
        }
    }
    conversation.input_buffer.blocks.retain(|b| {
        conversation
            .inputs
            .iter()
            .any(|i| i.id == b.id && !i.text.is_empty())
    });
    conversation
        .inputs
        .sort_by(|a, b| a.created_at.cmp(&b.created_at));
    changed
}
impl VoiceSessionApplication {
    pub(super) fn transcript_window(
        self: &Arc<Self>,
        window: &AudioWindow,
        transcript: Transcript,
    ) -> Result<(), String> {
        let blocks = self.update(|state| {
            let c = state.conversation.as_mut().ok_or("No conversation")?;
            reconcile(c, window, &transcript.sentences);
            Ok(c.inputs
                .iter()
                .filter(|i| {
                    i.stream_id.as_deref() == Some(&window.stream_id)
                        && i.end_ms >= window.start_ms
                        && !i.text.is_empty()
                })
                .cloned()
                .collect::<Vec<_>>())
        })?;
        let _ = self.events.send(json!({"type":"transcript_window","streamId":window.stream_id,"windowStartMs":window.start_ms,"blocks":blocks}));
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn later_audio_revises_a_sentence_without_losing_its_identity_or_acceptance() {
        let window = AudioWindow {
            stream_id: "stream".into(),
            epoch_ms: 1000,
            start_ms: 0,
            end_ms: 10_000,
            audio: String::new(),
        };
        let mut c = Conversation::default();
        reconcile(
            &mut c,
            &window,
            &[Sentence {
                text: "Find the file.".into(),
                start_ms: 1000,
                end_ms: 3000,
            }],
        );
        c.inputs[0].relevance = Some("direct_request".into());
        let id = c.inputs[0].id.clone();
        reconcile(
            &mut c,
            &window,
            &[Sentence {
                text: "Find the folder named voices.".into(),
                start_ms: 1100,
                end_ms: 5000,
            }],
        );
        assert_eq!(c.inputs.len(), 1);
        assert_eq!(c.inputs[0].id, id);
        assert_eq!(c.inputs[0].relevance.as_deref(), Some("direct_request"));
        assert_eq!(c.inputs[0].revision, 2);
        reconcile(&mut c, &window, &[]);
        assert!(c.inputs[0].text.is_empty());
    }
}
