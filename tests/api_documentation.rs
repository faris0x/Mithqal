use std::collections::BTreeSet;

fn methods(source: &str) -> BTreeSet<&str> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line.strip_prefix("pub fn ")?;
            let name = rest.split(['(', '<', ' ']).next()?;
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

#[test]
fn every_public_session_method_is_documented() {
    let source = methods(include_str!("../src/api.rs"));
    let documentation = methods(include_str!("../docs/api.md"));
    let undocumented = source
        .difference(&documentation)
        .copied()
        .collect::<Vec<_>>();
    let stale = documentation
        .difference(&source)
        .copied()
        .collect::<Vec<_>>();
    assert!(
        undocumented.is_empty(),
        "undocumented session methods: {undocumented:?}"
    );
    assert!(
        stale.is_empty(),
        "documented methods missing from source: {stale:?}"
    );
}
