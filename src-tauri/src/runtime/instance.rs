use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(crate) const APP_DATA_DIR_ENV: &str = "CODEX_ORCHESTRATOR_APP_DATA_DIR";

pub(crate) fn app_data_dir(default: PathBuf) -> Result<PathBuf, String> {
    resolve_app_data_dir(default, std::env::var_os(APP_DATA_DIR_ENV))
}

fn resolve_app_data_dir(
    default: PathBuf,
    override_value: Option<OsString>,
) -> Result<PathBuf, String> {
    let Some(value) = override_value else {
        return Ok(default);
    };
    if value.is_empty() {
        return Err(format!("{APP_DATA_DIR_ENV} must not be empty"));
    }
    let path = Path::new(&value);
    if !path.is_absolute() {
        return Err(format!("{APP_DATA_DIR_ENV} must be an absolute path"));
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_default_without_an_override() {
        let default = PathBuf::from("default");
        assert_eq!(
            resolve_app_data_dir(default.clone(), None).unwrap(),
            default
        );
    }

    #[test]
    fn accepts_only_an_absolute_override() {
        let absolute = std::env::temp_dir().join("isolated-app-data");
        assert_eq!(
            resolve_app_data_dir(PathBuf::from("default"), Some(absolute.clone().into())).unwrap(),
            absolute
        );
        assert!(
            resolve_app_data_dir(PathBuf::from("default"), Some(OsString::from("relative")))
                .is_err()
        );
    }
}
