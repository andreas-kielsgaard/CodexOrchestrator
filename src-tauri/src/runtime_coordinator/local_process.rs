use fs2::FileExt;
use orchid_coordinator::{client::Client, credentials::Credentials};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tokio::sync::Mutex;
pub(crate) struct LocalCoordinator {
    root: PathBuf,
    start: Mutex<()>,
}
impl LocalCoordinator {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            start: Mutex::new(()),
        }
    }
    pub async fn client(&self) -> Result<Client, String> {
        let _gate = self.start.lock().await;
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let startup = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("startup.lock"))
            .map_err(|e| e.to_string())?;
        let mut acquired = false;
        for _ in 0..100 {
            if startup.try_lock_exclusive().is_ok() {
                acquired = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        if !acquired {
            return Err(
                "Another Orchid instance is starting the local coordinator; retry shortly".into(),
            );
        }
        let auth = Credentials::new(&self.root.join("auth"))?;
        let reference = self.root.join("auth-reference");
        let id = if reference.exists() {
            fs::read_to_string(&reference)
                .map_err(|_| "Cannot read local coordinator authentication")?
        } else {
            let id = auth.save(&format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ))?;
            fs::write(&reference, &id)
                .map_err(|_| "Cannot retain local coordinator authentication")?;
            id
        };
        let token = auth.read(id.trim())?;
        let ready = self.root.join("ready.json");
        if let Some(c) = ready_client(&ready, &token) {
            if c.health().await.is_ok() {
                return Ok(c);
            }
        }
        let executable = std::env::var_os("ORCHID_COORDINATOR_EXECUTABLE")
            .map(PathBuf::from)
            .unwrap_or(
                std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .with_file_name(if cfg!(windows) {
                        "orchid-coordinator.exe"
                    } else {
                        "orchid-coordinator"
                    }),
            );
        if !executable.is_file() {
            return Err("The bundled coordinator is missing; rebuild Orchid with the normal application build command".into());
        }
        let mut command = Command::new(executable);
        command
            .arg("--data")
            .arg(&self.root)
            .args(["--bind", "127.0.0.1:0", "--token-file"])
            .arg(self.root.join("auth").join(id.trim()))
            .arg("--ready-file")
            .arg(&ready)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("Cannot start local coordinator: {e}"))?;
        for _ in 0..60 {
            if let Some(c) = ready_client(&ready, &token) {
                if c.health().await.is_ok() {
                    return Ok(c);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            let _ = child.try_wait();
        }
        Err("Local coordinator did not become available".into())
    }
}
fn ready_client(path: &Path, token: &str) -> Option<Client> {
    let v: serde_json::Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    Client::new(v["url"].as_str()?, token).ok()
}
