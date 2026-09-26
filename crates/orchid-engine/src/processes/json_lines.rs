//! Newline-delimited JSON from a child's stdout. Output arrives in arbitrary chunks; a value is
//! complete at its newline.
use serde_json::Value;

const MAX_BUFFERED_BYTES: usize = 32 * 1024 * 1024;

#[derive(Default)]
pub struct JsonLines {
    buffer: Vec<u8>,
}

impl JsonLines {
    /// Appends a chunk and returns every complete value. Blank lines are skipped.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Value>, String> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > MAX_BUFFERED_BYTES {
            return Err("A JSON line exceeds 32 MiB".into());
        }
        let mut values = Vec::new();
        while let Some(end) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<_> = self.buffer.drain(..=end).collect();
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            values.push(
                serde_json::from_slice(&line).map_err(|_| "Malformed JSON line".to_string())?,
            );
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_values_only_when_their_line_is_complete() {
        let mut lines = JsonLines::default();
        assert!(lines.push(b"{\"a\":").unwrap().is_empty());
        let values = lines.push(b"1}\n\n{\"b\":2}\n{\"c\"").unwrap();
        assert_eq!(
            values,
            vec![serde_json::json!({"a":1}), serde_json::json!({"b":2})]
        );
        assert!(lines.push(b"not json\n").is_err());
    }
}
