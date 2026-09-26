//! Orchid's `$name` skill mention syntax. It is the same for every provider: Orchid resolves a
//! mention to a pinned skill and the selected provider delivers that skill in its native form.

pub(crate) fn mention(name: &str) -> String {
    format!("${name}")
}

/// Whether `text` mentions `name` as a whole token.
pub(crate) fn mentions(text: &str, name: &str) -> bool {
    let marker = mention(name);
    text.match_indices(&marker).any(|(start, _)| {
        text[start + marker.len()..]
            .chars()
            .next()
            .is_none_or(|next| !next.is_alphanumeric() && next != '_' && next != '-')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_whole_names_only() {
        assert!(mentions("Use $review now", "review"));
        assert!(mentions("$review", "review"));
        assert!(!mentions("Use $reviewer now", "review"));
        assert!(!mentions("Use $review-deep now", "review"));
        assert!(!mentions("review without a marker", "review"));
    }
}
