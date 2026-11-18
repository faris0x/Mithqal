use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use mithqal::{ErrorCode, Session};

const PROFILES: &[&str] = &["core", "exact", "numeric", "modules", "embedded"];

#[derive(Debug)]
struct Case {
    id: String,
    profile: String,
    kind: String,
    expected: String,
    source: String,
    rationale: String,
    file: String,
    line: usize,
}

fn corpus_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/conformance");
    for entry in fs::read_dir(directory).expect("conformance directory") {
        let path = entry.expect("conformance entry").path();
        if path.extension().is_some_and(|extension| extension == "tsv") {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn unescape(text: &str) -> String {
    let mut result = String::new();
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('\\') => result.push('\\'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    result
}

fn parse_cases(file: &PathBuf) -> Vec<Case> {
    let name = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("conformance")
        .to_string();
    let text = fs::read_to_string(file).expect("conformance file");
    let mut cases = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            6,
            "{name}:{}: expected six tab separated fields",
            index + 1
        );
        cases.push(Case {
            id: fields[0].to_string(),
            profile: fields[1].to_string(),
            kind: fields[2].to_string(),
            expected: unescape(fields[3]),
            source: unescape(fields[4]),
            rationale: fields[5].to_string(),
            file: name.clone(),
            line: index + 1,
        });
    }
    cases
}

fn code(error: &mithqal::Diagnostic) -> String {
    ErrorCode::as_str(error.code).to_string()
}

fn run_case(case: &Case) -> Result<(), String> {
    let mut session = Session::new();
    match case.kind.as_str() {
        "value" | "unevaluated" => {
            let value = session
                .eval_named_source(&case.id, &case.source)
                .map_err(|error| format!("unexpected diagnostic: {error}"))?;
            let actual = session
                .format(value)
                .map_err(|error| format!("format failed: {error}"))?;
            if actual != case.expected {
                return Err(format!(
                    "expected {expected:?}, got {actual:?}",
                    expected = case.expected
                ));
            }
        }
        "output" => {
            session
                .eval_named_source(&case.id, &case.source)
                .map_err(|error| format!("unexpected diagnostic: {error}"))?;
            let actual = session.take_output().join("\n");
            if actual != case.expected {
                return Err(format!(
                    "expected {expected:?}, got {actual:?}",
                    expected = case.expected
                ));
            }
        }
        "parse" | "diagnostic" => {
            let error = session
                .eval_named_source(&case.id, &case.source)
                .err()
                .ok_or_else(|| "expected a diagnostic, evaluation succeeded".to_string())?;
            let actual = code(&error);
            if actual != case.expected {
                return Err(format!(
                    "expected {expected:?}, got {actual:?}",
                    expected = case.expected
                ));
            }
        }
        "trace_line" => {
            let expected = case
                .expected
                .parse::<usize>()
                .map_err(|_| format!("invalid trace line {:?}", case.expected))?;
            let error = session
                .eval_named_source(&case.id, &case.source)
                .err()
                .ok_or_else(|| "expected a diagnostic, evaluation succeeded".to_string())?;
            let found = error.span.as_deref().map(|span| span.line) == Some(expected)
                || error
                    .trace
                    .iter()
                    .any(|frame| frame.span.is_some_and(|span| span.line == expected));
            if !found {
                return Err(format!(
                    "no diagnostic location at line {expected}: {error}"
                ));
            }
        }
        "roundtrip" => {
            let value = session
                .parse_statement(&case.source)
                .map_err(|error| format!("parse failed: {error}"))?;
            let first = session
                .format(value)
                .map_err(|error| format!("format failed: {error}"))?;
            let reparsed = session
                .parse_statement(&first)
                .map_err(|error| format!("reparse failed: {error}"))?;
            let second = session
                .format(reparsed)
                .map_err(|error| format!("format failed: {error}"))?;
            if first != second {
                return Err(format!("round trip changed {first:?} to {second:?}"));
            }
            if !case.expected.is_empty() && first != case.expected {
                return Err(format!(
                    "expected {expected:?}, got {first:?}",
                    expected = case.expected
                ));
            }
        }
        other => return Err(format!("unknown case kind {other:?}")),
    }
    Ok(())
}

#[test]
fn conformance_corpus() {
    let filter = std::env::var("MITHQAL_CONFORMANCE_PROFILE").ok();
    let files = corpus_files();
    for profile in PROFILES {
        let expected = format!("spec/conformance/{profile}.tsv");
        assert!(
            files.iter().any(|file| file.ends_with(&expected)),
            "missing corpus file for profile {profile}"
        );
    }
    let mut seen = HashSet::new();
    let mut failures = Vec::new();
    let mut count = 0usize;
    for file in files {
        for case in parse_cases(&file) {
            assert!(
                PROFILES.contains(&case.profile.as_str()),
                "{}:{}: unknown profile {}",
                case.file,
                case.line,
                case.profile
            );
            assert!(
                seen.insert(case.id.clone()),
                "duplicate conformance case id {}",
                case.id
            );
            assert!(
                !case.rationale.trim().is_empty(),
                "{}:{}: missing rationale",
                case.file,
                case.line
            );
            if filter
                .as_deref()
                .is_some_and(|profile| profile != case.profile)
            {
                continue;
            }
            count += 1;
            if let Err(message) = run_case(&case) {
                failures.push(format!(
                    "{} ({}:{}): {message}",
                    case.id, case.file, case.line
                ));
            }
        }
    }
    assert!(count > 0, "the conformance corpus is empty");
    assert!(
        failures.is_empty(),
        "conformance failures:\n{}",
        failures.join("\n")
    );
}
