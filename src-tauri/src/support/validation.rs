pub(crate) fn validate_non_empty(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} is required."));
    }

    Ok(())
}

pub(crate) fn validate_optional_non_empty(
    label: &str,
    value: Option<String>,
) -> Result<Option<String>, String> {
    match value {
        Some(value) => {
            validate_non_empty(label, &value)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

pub(crate) fn validate_value<'a>(
    label: &str,
    value: &'a str,
    allowed_values: &[&str],
) -> Result<&'a str, String> {
    if allowed_values.contains(&value) {
        return Ok(value);
    }

    Err(format!("Invalid {label}: {value}"))
}
