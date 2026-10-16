use super::*;

pub(super) fn validate_relative(name: &str) -> Result<(), String> {
    if name.contains('\\')
        || name.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.contains(':')
                || part.ends_with(['.', ' '])
                || part.chars().any(|c| c.is_control())
                || [
                    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6",
                    "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7",
                    "lpt8", "lpt9",
                ]
                .contains(&part.split('.').next().unwrap_or("").to_lowercase().as_str())
        })
    {
        return Err("Skill resource path is not portable".into());
    }
    Ok(())
}
pub(super) fn read_package(root: &Path) -> Result<BTreeMap<String, SkillFile>, String> {
    fn visit(
        root: &Path,
        dir: &Path,
        output: &mut BTreeMap<String, SkillFile>,
        bytes: &mut u64,
    ) -> Result<(), String> {
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err(format!(
                    "Skill resource {} is a symbolic link",
                    path.display()
                ));
            }
            if kind.is_dir() {
                visit(root, &path, output, bytes)?;
                continue;
            }
            if !kind.is_file() {
                return Err("Skill resources must be regular files".into());
            }
            let name = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("Skill resource filename is not UTF-8")?
                .replace('\\', "/");
            validate_relative(&name)?;
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            *bytes += metadata.len();
            if *bytes > 32 * 1024 * 1024 {
                return Err("Skill package exceeds 32 MiB".into());
            }
            #[cfg(unix)]
            let executable = {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            };
            #[cfg(not(unix))]
            let executable = false;
            output.insert(
                name,
                SkillFile {
                    data_base64: base64::engine::general_purpose::STANDARD
                        .encode(std::fs::read(&path).map_err(|e| e.to_string())?),
                    executable,
                },
            );
        }
        Ok(())
    }
    let mut output = BTreeMap::new();
    visit(root, root, &mut output, &mut 0)?;
    Ok(output)
}
pub(super) fn materialize(root: &Path, skill: &SkillPackage) -> Result<PathBuf, String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let directory = root.join(&skill.id);
    if std::fs::symlink_metadata(&directory).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Skill materialization contains a symbolic link".into());
    }
    if directory.exists() {
        let owned = directory.canonicalize().map_err(|e| e.to_string())?;
        if !owned.starts_with(&root) {
            return Err("Skill copy is outside its owned directory".into());
        }
        for name in read_package(&owned)?
            .keys()
            .filter(|name| !skill.files.contains_key(*name))
        {
            let stale = owned.join(name).canonicalize().map_err(|e| e.to_string())?;
            if !stale.starts_with(&owned) {
                return Err("Stale skill resource is outside its owned directory".into());
            }
            std::fs::remove_file(stale).map_err(|e| e.to_string())?;
        }
    }
    // This directory is exclusively an Orchid copy, never the external origin.
    for (name, file) in &skill.files {
        let path = directory.join(name);
        let mut parent = root.clone();
        for component in path
            .strip_prefix(&root)
            .map_err(|e| e.to_string())?
            .components()
        {
            parent.push(component);
            if std::fs::symlink_metadata(&parent).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err("Skill materialization contains a symbolic link".into());
            }
        }
        std::fs::create_dir_all(path.parent().ok_or("Skill path has no parent")?)
            .map_err(|e| e.to_string())?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&file.data_base64)
            .map_err(|e| e.to_string())?;
        if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(if file.executable { 0o755 } else { 0o644 }),
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(directory.join("SKILL.md"))
}
