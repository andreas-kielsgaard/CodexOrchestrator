//! Source guards for the agent-provider boundary.
use std::path::Path;

const RAW_PAYLOAD_READS: [&str; 4] = [
    "raw_payload[",
    "raw_payload.get(",
    "raw_payload.as_",
    "raw_payload.pointer(",
];

/// Provider raw payloads are diagnostic evidence. Outside provider modules, product code reads
/// provider facts from normalized events and Orchid's own control vocabulary through
/// `RuntimeControlRecord`; it never decides on raw payload fields. Tests may inspect evidence.
#[test]
fn product_code_never_reads_raw_payload_fields_outside_providers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    visit(&root, &mut |path| {
        let relative = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if relative.starts_with("runtime/providers/") || is_test_source(&relative) {
            return;
        }
        let source = std::fs::read_to_string(path).unwrap();
        for (index, line) in production_lines(&source) {
            if RAW_PAYLOAD_READS.iter().any(|read| line.contains(read)) {
                offenders.push(format!("{relative}:{}", index + 1));
            }
        }
    });
    assert!(
        offenders.is_empty(),
        "Decide on normalized events or RuntimeControlRecord, not raw payload fields: {offenders:?}"
    );
}

fn is_test_source(relative: &str) -> bool {
    let name = relative.rsplit('/').next().unwrap_or(relative);
    name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name == "live_smoke.rs"
        || relative.split('/').any(|part| part == "tests")
}

/// Lines before the file's inline `#[cfg(test)] mod … {` block.
fn production_lines(source: &str) -> impl Iterator<Item = (usize, &str)> {
    let lines = source.lines().collect::<Vec<_>>();
    let end = lines
        .windows(2)
        .position(|pair| {
            pair[0].trim() == "#[cfg(test)]"
                && pair[1].trim_start().starts_with("mod ")
                && pair[1].trim_end().ends_with('{')
        })
        .unwrap_or(lines.len());
    lines.into_iter().take(end).enumerate()
}

fn visit(directory: &Path, found: &mut dyn FnMut(&Path)) {
    for entry in std::fs::read_dir(directory).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found(&path);
        }
    }
}
