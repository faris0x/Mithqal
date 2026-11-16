use mithqal::Session;
use mithqal::catalog::{self, ExampleKind, Support};

#[test]
fn documented_examples_match_their_declared_kind() {
    let mut failures = Vec::new();
    for function in catalog::FUNCTIONS {
        let mut session = Session::new();
        let parsed = match session.parse_statement(function.example) {
            Ok(value) => value,
            Err(error) => {
                failures.push(format!("{}: parse error: {error}", function.name));
                continue;
            }
        };
        match session.eval_source(function.example) {
            Ok(value) => {
                if value == parsed && function.example_kind != ExampleKind::Symbolic {
                    failures.push(format!(
                        "{}: example remained unchanged: {}",
                        function.name, function.example
                    ));
                }
            }
            Err(error) => failures.push(format!("{}: evaluation error: {error}", function.name)),
        }
    }
    assert!(
        failures.is_empty(),
        "catalog example failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn kernel_heads_have_catalog_entries() {
    let source = include_str!("../src/eval/heads.rs");
    let internal = ["plus", "times", "negate", "power"];
    let mut missing = Vec::new();
    for line in source.lines() {
        let Some((left, _)) = line.split_once("=> Self::") else {
            continue;
        };
        let Some(start) = left.find('"') else {
            continue;
        };
        let Some(end) = left[start + 1..].find('"') else {
            continue;
        };
        let name = &left[start + 1..start + 1 + end];
        if catalog::find(name).is_none() && !internal.contains(&name) {
            missing.push(name.to_string());
        }
    }
    missing.sort();
    missing.dedup();
    assert!(
        missing.is_empty(),
        "kernel heads without catalog entries: {missing:?}"
    );
}

#[test]
fn every_non_language_function_has_a_test_reference() {
    let mut sources = String::new();
    for entry in std::fs::read_dir("tests").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push_str(&std::fs::read_to_string(&path).unwrap());
        }
    }
    let mut missing = Vec::new();
    for function in catalog::FUNCTIONS {
        if function.capability.support == Support::Language {
            continue;
        }
        let call = format!("{}(", function.name);
        if !sources.contains(&call) {
            missing.push(function.name);
        }
    }
    assert!(
        missing.is_empty(),
        "catalog functions without test references: {missing:?}"
    );
}
