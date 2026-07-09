use super::*;

pub(crate) fn ensure_same_anchor(label: &str, expected: &str, actual: &str) -> Result<(), String> {
    if expected == actual {
        return Ok(());
    }

    Err(format!(
        "Mismatched {label}: {expected} does not match {actual}"
    ))
}

pub(crate) fn path_label(path: &str) -> Option<String> {
    PathBuf::from(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn bool_to_sqlite(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

pub(crate) fn normalize_git_branch_ref(branch_ref: &str) -> Option<String> {
    let branch_name = branch_ref
        .trim()
        .strip_prefix("refs/heads/")
        .unwrap_or(branch_ref.trim());

    if branch_name.is_empty() {
        None
    } else {
        Some(branch_name.to_string())
    }
}

pub(crate) fn same_filesystem_path(left: &str, right: &str) -> bool {
    normalize_path_for_compare(left) == normalize_path_for_compare(right)
}

pub(crate) fn normalize_path_for_compare(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}
