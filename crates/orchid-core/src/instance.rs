//! One execution or synchronization process owns an instance directory.
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
pub struct InstanceLease {
    _file: File,
}
impl InstanceLease {
    pub fn acquire(root: &Path, role: &str) -> Result<Self, String> {
        std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("instance.lock"))
            .map_err(|e| e.to_string())?;
        file.try_lock()
            .map_err(|_| format!("Another Orchid process owns {}", root.display()))?;
        let mut existing = String::new();
        file.read_to_string(&mut existing)
            .map_err(|e| e.to_string())?;
        if existing.is_empty() {
            file.write_all(role.as_bytes()).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        } else if existing != role {
            return Err(format!(
                "This is an Orchid {existing} directory. Give {role} a separate data directory."
            ));
        }
        Ok(Self { _file: file })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn separate_roles_and_processes_cannot_share_an_instance() {
        let root = tempfile::tempdir().unwrap();
        let owner = InstanceLease::acquire(root.path(), "host").unwrap();
        assert!(InstanceLease::acquire(root.path(), "host").is_err());
        drop(owner);
        assert!(InstanceLease::acquire(root.path(), "server").is_err());
        assert!(InstanceLease::acquire(root.path(), "host").is_ok());
    }
}
