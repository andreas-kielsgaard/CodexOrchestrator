use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::Mutex,
};

struct Worker {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub(crate) struct SpeechSession {
    root: PathBuf,
    worker: Mutex<Option<Worker>>,
    preview_worker: Mutex<Option<Worker>>,
}
impl SpeechSession {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            worker: Mutex::new(None),
            preview_worker: Mutex::new(None),
        }
    }
    pub(crate) fn configured(&self) -> bool {
        self.python().is_file() && self.root.join("model/model.bin").is_file()
    }
    fn python(&self) -> PathBuf {
        self.root.join("venv/Scripts/python.exe")
    }
    pub(crate) fn transcribe(&self, audio: &str) -> Result<String, String> {
        self.transcribe_with(&self.worker, "transcribe.py", audio)
    }
    pub(crate) fn transcribe_preview(&self, audio: &str) -> Result<String, String> {
        self.transcribe_with(&self.preview_worker, "transcribe-preview.py", audio)
    }
    fn transcribe_with(&self, worker: &Mutex<Option<Worker>>, script_name: &str, audio: &str) -> Result<String, String> {
        if audio.len() > 5_400_000 {
            return Err("Speech input exceeds 4 MB".into());
        }
        if !self.configured() {
            return Err("Run npm run setup:voice to install local transcription".into());
        }
        let mut slot = worker
            .lock()
            .map_err(|_| "Transcription unavailable")?;
        if slot.is_none() {
            let script = self.root.join(script_name);
            std::fs::write(
                &script,
                include_str!("../../../resources/voice-sessions/transcribe.py"),
            )
            .map_err(|e| e.to_string())?;
            let mut command = Command::new(self.python());
            command
                .arg("-u")
                .arg(script)
                .arg(self.root.join("model"))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x08000000);
            }
            let mut child = command.spawn().map_err(|e| e.to_string())?;
            let input = child.stdin.take().ok_or("Transcriber stdin unavailable")?;
            let output = BufReader::new(
                child
                    .stdout
                    .take()
                    .ok_or("Transcriber stdout unavailable")?,
            );
            *slot = Some(Worker {
                child,
                input,
                output,
            });
        }
        let result = (|| {
            let worker = slot.as_mut().unwrap();
            writeln!(worker.input, "{}", json!({"audio":audio})).map_err(|e| e.to_string())?;
            worker.input.flush().map_err(|e| e.to_string())?;
            let mut line = String::new();
            worker
                .output
                .read_line(&mut line)
                .map_err(|e| e.to_string())?;
            let value: Value = serde_json::from_str(&line)
                .map_err(|_| "Local transcriber stopped; retry the input".to_string())?;
            if let Some(error) = value["error"].as_str() {
                return Err(error.to_string());
            }
            value["text"]
                .as_str()
                .map(str::to_owned)
                .ok_or("No transcript".into())
        })();
        if result.is_err() {
            *slot = None;
        }
        result
    }
    pub(crate) fn close(&self) {
        if let Ok(mut worker) = self.worker.lock() {
            *worker = None;
        }
        if let Ok(mut worker) = self.preview_worker.lock() {
            *worker = None;
        }
    }
}
