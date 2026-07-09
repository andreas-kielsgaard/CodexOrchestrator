use super::*;

pub(crate) fn discover_git_repos(
    input: DiscoverTaskReposCommandInput,
) -> Result<Vec<DiscoveredTaskRepo>, String> {
    let root_path = input.root_path.trim();
    validate_non_empty("rootPath", root_path)?;

    let root = PathBuf::from(root_path);

    if !root.is_dir() {
        return Err(format!("Search root is not a directory: {root_path}"));
    }

    let max_depth = input.max_depth.unwrap_or(4).min(8);
    let mut repos = Vec::new();
    let mut seen_paths = HashSet::new();
    collect_git_repos(&root, 0, max_depth, &mut repos, &mut seen_paths);
    repos.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(repos)
}

pub(crate) fn collect_git_repos(
    directory: &Path,
    depth: usize,
    max_depth: usize,
    repos: &mut Vec<DiscoveredTaskRepo>,
    seen_paths: &mut HashSet<String>,
) {
    if is_git_repo_path(directory) {
        add_discovered_repo(directory, repos, seen_paths);
        return;
    }

    if depth >= max_depth {
        return;
    }

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() && should_scan_child_dir(&path) {
            collect_git_repos(&path, depth + 1, max_depth, repos, seen_paths);
        }
    }
}

pub(crate) fn is_git_repo_path(directory: &Path) -> bool {
    let git_marker = directory.join(".git");
    git_marker.is_dir() || git_marker.is_file()
}

pub(crate) fn should_scan_child_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    !matches!(
        name,
        ".git" | ".dev" | ".vite" | "coverage" | "dist" | "dist-ssr" | "node_modules" | "target"
    )
}

pub(crate) fn add_discovered_repo(
    directory: &Path,
    repos: &mut Vec<DiscoveredTaskRepo>,
    seen_paths: &mut HashSet<String>,
) {
    let display_path = directory.to_string_lossy().to_string();
    let key = normalize_path_for_compare(&display_path);

    if !seen_paths.insert(key) {
        return;
    }

    repos.push(DiscoveredTaskRepo {
        name: path_label(&display_path).unwrap_or_else(|| display_path.clone()),
        path: display_path,
    });
}
