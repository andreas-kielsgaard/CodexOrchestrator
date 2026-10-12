use super::domain::{SpokenInput, VoiceInputPolicy};
use std::collections::BTreeMap;

pub(super) fn sentence_complete(text: &str) -> bool {
    text.trim_end().trim_end_matches(['"', '\u{201d}', '\u{2019}', ')']).ends_with(['.', '?', '!'])
}

#[derive(Default)]
pub(super) struct QuerySchedule {
    queried: BTreeMap<String, u64>,
    last_pause_voice_ms: u64,
}
impl QuerySchedule {
    pub(super) fn due(
        &mut self,
        blocks: &[SpokenInput],
        policy: &VoiceInputPolicy,
        quiet_ms: u64,
        last_voice_ms: u64,
        evaluating: bool,
        playing: bool,
    ) -> bool {
        if blocks.is_empty() || evaluating {
            return false;
        }
        let new_sentences = blocks
            .iter()
            .filter(|b| {
                self.queried.get(&b.id).copied().unwrap_or(0) < b.revision
                    && sentence_complete(&b.text)
            })
            .count();
        let revised = blocks
            .iter()
            .any(|b| self.queried.get(&b.id).copied().unwrap_or(0) < b.revision);
        let pause = quiet_ms >= policy.pause_seconds * 1000
            && (last_voice_ms > self.last_pause_voice_ms || revised);
        if new_sentences < if playing {1} else {policy.sentences_per_query} && !pause {
            return false;
        }
        self.queried = blocks.iter().map(|b| (b.id.clone(), b.revision)).collect();
        if pause {
            self.last_pause_voice_ms = last_voice_ms;
        }
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn playback_queries_the_first_complete_sentence_but_not_partial_speech() {
        let mut schedule = QuerySchedule::default();
        let mut blocks = vec![SpokenInput { id:"one".into(), text:"Orchid can you".into(), revision:1, ..Default::default() }];
        let policy = VoiceInputPolicy::default();
        assert!(!schedule.due(&blocks, &policy, 0, 1000, false, true));
        blocks[0].text="Orchid, can you explain?".into(); blocks[0].revision=2;
        assert!(schedule.due(&blocks, &policy, 0, 1000, false, true));
        assert!(!schedule.due(&blocks, &policy, 0, 1000, false, true));
    }
    #[test]
    fn sentence_threshold_and_pause_are_independent_and_do_not_loop_while_idle() {
        let policy = VoiceInputPolicy::default();
        let mut schedule = QuerySchedule::default();
        let mut blocks = vec![SpokenInput {
            id: "one".into(),
            text: "First sentence.".into(),
            revision: 1,
            ..Default::default()
        }];
        assert!(!schedule.due(&blocks, &policy, 0, 1000, false, false));
        assert!(schedule.due(&blocks, &policy, 2000, 1000, false, false));
        assert!(!schedule.due(&blocks, &policy, 3000, 1000, false, false));
        blocks[0].revision = 2;
        assert!(schedule.due(&blocks, &policy, 3000, 1000, false, false));
        assert!(!schedule.due(&blocks, &policy, 4000, 1000, false, false));
        blocks[0].revision = 3;
        blocks.push(SpokenInput {
            id: "two".into(),
            text: "Second sentence.".into(),
            revision: 1,
            ..Default::default()
        });
        assert!(!schedule.due(&blocks, &policy, 0, 4000, true, false));
        assert!(schedule.due(&blocks, &policy, 0, 4000, false, false));
    }
}
