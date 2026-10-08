use super::domain::{SpokenInput, VoiceInputPolicy};
use std::collections::BTreeMap;

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
    ) -> bool {
        if blocks.is_empty() || evaluating {
            return false;
        }
        let new_sentences = blocks
            .iter()
            .filter(|b| {
                self.queried.get(&b.id).copied().unwrap_or(0) < b.revision
                    && b.text
                        .trim_end_matches(['"', '\u{201d}', '\u{2019}', ')'])
                        .ends_with(['.', '?', '!'])
            })
            .count();
        let revised = blocks
            .iter()
            .any(|b| self.queried.get(&b.id).copied().unwrap_or(0) < b.revision);
        let pause = quiet_ms >= policy.pause_seconds * 1000
            && (last_voice_ms > self.last_pause_voice_ms || revised);
        if new_sentences < policy.sentences_per_query && !pause {
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
    fn sentence_threshold_and_pause_are_independent_and_do_not_loop_while_idle() {
        let policy = VoiceInputPolicy::default();
        let mut schedule = QuerySchedule::default();
        let mut blocks = vec![SpokenInput {
            id: "one".into(),
            text: "First sentence.".into(),
            revision: 1,
            ..Default::default()
        }];
        assert!(!schedule.due(&blocks, &policy, 0, 1000, false));
        assert!(schedule.due(&blocks, &policy, 2000, 1000, false));
        assert!(!schedule.due(&blocks, &policy, 3000, 1000, false));
        blocks[0].revision = 2;
        assert!(schedule.due(&blocks, &policy, 3000, 1000, false));
        assert!(!schedule.due(&blocks, &policy, 4000, 1000, false));
        blocks[0].revision = 3;
        blocks.push(SpokenInput {
            id: "two".into(),
            text: "Second sentence.".into(),
            revision: 1,
            ..Default::default()
        });
        assert!(!schedule.due(&blocks, &policy, 0, 4000, true));
        assert!(schedule.due(&blocks, &policy, 0, 4000, false));
    }
}
