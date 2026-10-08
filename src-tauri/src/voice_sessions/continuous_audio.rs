use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone)]
pub(super) struct AudioWindow {
    pub(super) stream_id: String,
    pub(super) epoch_ms: i64,
    pub(super) start_ms: u64,
    pub(super) end_ms: u64,
    pub(super) audio: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct Sentence {
    pub(super) text: String,
    pub(super) start_ms: u64,
    pub(super) end_ms: u64,
}
#[derive(Deserialize)]
pub(super) struct Transcript {
    pub(super) text: String,
    pub(super) sentences: Vec<Sentence>,
}
#[derive(Default)]
pub(super) struct ContinuousAudio {
    stream_id: String,
    epoch_ms: i64,
    next_sample: u64,
    pcm: VecDeque<u8>,
}
impl ContinuousAudio {
    pub(super) fn push(
        &mut self,
        stream_id: &str,
        epoch_ms: i64,
        start_sample: u64,
        encoded: &str,
        context_seconds: u64,
    ) -> Result<AudioWindow, String> {
        let bytes = STANDARD.decode(encoded).map_err(|_| "Invalid PCM audio")?;
        if bytes.is_empty() || bytes.len() > 64_000 || bytes.len() % 2 != 0 {
            return Err("Expected at most two seconds of mono 16-bit PCM at 16 kHz".into());
        }
        if stream_id != self.stream_id || self.pcm.is_empty() {
            self.stream_id = stream_id.into();
            self.epoch_ms = epoch_ms;
            self.next_sample = start_sample;
            self.pcm.clear();
        }
        if start_sample != self.next_sample {
            return Err(
                "Audio stream discontinuity; reactivate listening to start a new stream".into(),
            );
        }
        self.next_sample += (bytes.len() / 2) as u64;
        self.pcm.extend(bytes);
        let capacity = context_seconds.clamp(10, 120) as usize * 32_000;
        while self.pcm.len() > capacity {
            self.pcm.pop_front();
        }
        let end_ms = self.next_sample / 16;
        let start_ms = (self.next_sample - (self.pcm.len() / 2) as u64) / 16;
        let body: Vec<u8> = self.pcm.iter().copied().collect();
        let mut wav = Vec::with_capacity(44 + body.len());
        wav.extend(b"RIFF");
        wav.extend((36 + body.len() as u32).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(16_000u32.to_le_bytes());
        wav.extend(32_000u32.to_le_bytes());
        wav.extend(2u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend((body.len() as u32).to_le_bytes());
        wav.extend(body);
        Ok(AudioWindow {
            stream_id: self.stream_id.clone(),
            epoch_ms: self.epoch_ms,
            start_ms,
            end_ms,
            audio: STANDARD.encode(wav),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_keep_every_packet_and_evict_only_the_oldest_audio() {
        let mut stream = ContinuousAudio::default();
        for second in 0..12 {
            let pcm = vec![second as u8; 32_000];
            let window = stream
                .push("stream", 123, second * 16_000, &STANDARD.encode(pcm), 10)
                .unwrap();
            let decoded = STANDARD.decode(window.audio).unwrap();
            assert_eq!(window.end_ms, (second + 1) * 1000);
            assert_eq!(
                window.start_ms,
                ((second + 1) * 1000).saturating_sub(10_000)
            );
            assert_eq!(decoded.last(), Some(&(second as u8)));
            if second >= 10 {
                assert_eq!(decoded[44], (second - 9) as u8);
            }
        }
        assert!(stream
            .push(
                "stream",
                123,
                14 * 16_000,
                &STANDARD.encode(vec![0; 32_000]),
                10
            )
            .is_err());
        let restarted = stream
            .push("new-stream", 456, 0, &STANDARD.encode(vec![1; 32_000]), 10)
            .unwrap();
        assert_eq!(restarted.start_ms, 0);
        assert_eq!(restarted.epoch_ms, 456);
    }
}
