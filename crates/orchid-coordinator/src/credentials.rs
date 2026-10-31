use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub struct Credentials {
    root: PathBuf,
}
impl Credentials {
    pub fn new(root: &Path) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|_| "Cannot create coordinator credential directory")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(root, fs::Permissions::from_mode(0o700))
                .map_err(|_| "Cannot protect credential directory")?;
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let account = format!(
                "{}\\{}",
                std::env::var("USERDOMAIN").map_err(|_| "Missing Windows account")?,
                std::env::var("USERNAME").map_err(|_| "Missing Windows account")?
            );
            let result = std::process::Command::new("icacls")
                .arg(root)
                .args([
                    "/inheritance:r",
                    "/grant:r",
                    &format!("{account}:(OI)(CI)F"),
                ])
                .creation_flags(0x08000000)
                .output()
                .map_err(|_| "Cannot protect credential directory")?;
            if !result.status.success() {
                return Err("Cannot protect credential directory".into());
            }
        }
        Ok(Self {
            root: root.to_owned(),
        })
    }
    fn path(&self, id: &str) -> Result<PathBuf, String> {
        Uuid::parse_str(id).map_err(|_| "Invalid credential reference")?;
        Ok(self.root.join(id))
    }
    pub fn save(&self, token: &str) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(self.path(&id)?)
            .map_err(|_| "Cannot save coordinator credential")?;
        file.write_all(token.as_bytes())
            .map_err(|_| "Cannot save coordinator credential")?;
        Ok(id)
    }
    pub fn read(&self, id: &str) -> Result<String, String> {
        fs::read_to_string(self.path(id)?).map_err(|_| "Coordinator credential is missing".into())
    }
}
