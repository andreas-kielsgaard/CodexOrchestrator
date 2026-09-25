use serde::Serialize;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RegisteredNativeCodexHome {
    pub(super) profile_id: String,
    pub(super) home_path: PathBuf,
    pub(super) selected: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiscoveredNativeCodexHome {
    home_path: String,
    source: NativeCodexHomeDiscoverySource,
    registered_profile_id: Option<String>,
    selected: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NativeCodexHomeDiscoverySource {
    Environment,
    Default,
    Sibling,
    Registered,
}

pub(super) fn discover_native_codex_homes(
    registered: &[RegisteredNativeCodexHome],
) -> Result<Vec<DiscoveredNativeCodexHome>, String> {
    let user_home = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from);
    discover_from(
        registered,
        env::var_os("CODEX_HOME").map(PathBuf::from),
        user_home,
    )
}

fn discover_from(
    registered: &[RegisteredNativeCodexHome],
    environment_home: Option<PathBuf>,
    user_home: Option<PathBuf>,
) -> Result<Vec<DiscoveredNativeCodexHome>, String> {
    let mut candidates = Vec::new();
    if let Some(path) = environment_home {
        candidates.push((path, NativeCodexHomeDiscoverySource::Environment));
    }
    if let Some(root) = user_home {
        candidates.push((root.join(".codex"), NativeCodexHomeDiscoverySource::Default));
        if root.is_dir() {
            for entry in fs::read_dir(&root)
                .map_err(|error| format!("Unable to inspect conventional Codex homes: {error}"))?
            {
                let entry = entry.map_err(|error| {
                    format!("Unable to inspect a conventional Codex home: {error}")
                })?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with(".codex-") {
                    candidates.push((entry.path(), NativeCodexHomeDiscoverySource::Sibling));
                }
            }
        }
    }
    candidates.extend(registered.iter().map(|home| {
        (
            home.home_path.clone(),
            NativeCodexHomeDiscoverySource::Registered,
        )
    }));

    let mut registered_by_path = HashMap::new();
    for home in registered {
        if let Ok(path) = fs::canonicalize(&home.home_path) {
            registered_by_path.insert(path_key(&path), home);
        }
    }

    let mut discovered = HashMap::<String, DiscoveredNativeCodexHome>::new();
    for (candidate, source) in candidates {
        if !candidate.is_dir() {
            continue;
        }
        let canonical = fs::canonicalize(&candidate).map_err(|error| {
            format!(
                "Unable to resolve discovered Codex home {}: {error}",
                candidate.display()
            )
        })?;
        let key = path_key(&canonical);
        let registration = registered_by_path.get(&key);
        discovered
            .entry(key)
            .or_insert_with(|| DiscoveredNativeCodexHome {
                home_path: canonical.to_string_lossy().into_owned(),
                source,
                registered_profile_id: registration.map(|home| home.profile_id.clone()),
                selected: registration.is_some_and(|home| home.selected),
            });
    }

    let mut homes = discovered.into_values().collect::<Vec<_>>();
    homes.sort_by(|left, right| {
        left.home_path
            .to_lowercase()
            .cmp(&right.home_path.to_lowercase())
            .then_with(|| left.home_path.cmp(&right.home_path))
    });
    Ok(homes)
}

fn path_key(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_bounded_existing_homes_and_deduplicates_registered_paths() {
        let root = tempfile::tempdir().unwrap();
        let default = root.path().join(".codex");
        let sibling = root.path().join(".codex-review");
        fs::create_dir(&default).unwrap();
        fs::create_dir(&sibling).unwrap();
        let homes = discover_from(
            &[RegisteredNativeCodexHome {
                profile_id: "profile-default".into(),
                home_path: default.clone(),
                selected: true,
            }],
            Some(default),
            Some(root.path().to_path_buf()),
        )
        .unwrap();

        assert_eq!(homes.len(), 2);
        let selected = homes.iter().find(|home| home.selected).unwrap();
        assert_eq!(
            selected.registered_profile_id.as_deref(),
            Some("profile-default")
        );
        assert!(homes
            .iter()
            .any(|home| home.home_path.ends_with(".codex-review")));
    }

    #[test]
    fn excludes_missing_registered_and_conventional_paths() {
        let root = tempfile::tempdir().unwrap();
        let homes = discover_from(
            &[RegisteredNativeCodexHome {
                profile_id: "missing".into(),
                home_path: root.path().join("missing"),
                selected: false,
            }],
            Some(root.path().join("missing-environment")),
            Some(root.path().to_path_buf()),
        )
        .unwrap();
        assert!(homes.is_empty());
    }
}
