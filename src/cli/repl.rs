//! Interactive read, evaluate, print loop.

#[allow(unused_imports)]
use crate::prelude::*;

use std::path::PathBuf;
use std::time::Instant;

use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Config, Editor};

use crate::{ExprView, Session};

use super::commands;
use super::completion::CliHelper;
use super::format;
use super::settings::Settings;

fn history_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return Some(PathBuf::from(path).join("mithqal").join("history"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share/mithqal/history"))
}

fn rewrite_assignment(source: &str) -> Option<String> {
    let mut depth = 0i32;
    let mut assignment = None;
    let bytes = source.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'=' if depth == 0 => {
                let previous = index.checked_sub(1).and_then(|index| bytes.get(index));
                let next = bytes.get(index + 1);
                if matches!(previous, Some(b'=' | b'<' | b'>' | b'!')) || matches!(next, Some(b'='))
                {
                    continue;
                }
                assignment = Some(index);
                break;
            }
            _ => {}
        }
    }
    let index = assignment?;
    let name = source[..index].trim();
    let value = source[index + 1..].trim();
    let mut characters = name.chars();
    let first = characters.next()?;
    if !(first.is_alphabetic() || first == '_')
        || !characters.all(|character| character.is_alphanumeric() || character == '_')
        || value.is_empty()
    {
        return None;
    }
    Some(format!("set({name},{value})"))
}

pub(super) fn run() -> i32 {
    let config = Config::builder().auto_add_history(true).build();
    let mut editor = match Editor::<CliHelper, DefaultHistory>::with_config(config) {
        Ok(editor) => editor,
        Err(error) => {
            eprintln!("mithqal: cannot start the line editor: {error}");
            return 1;
        }
    };
    editor.set_helper(Some(CliHelper));
    let history = history_path();
    if let Some(path) = &history {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = editor.load_history(path);
    }
    println!(
        "Mithqal {}. Type help for commands.",
        env!("CARGO_PKG_VERSION")
    );
    let mut session = Session::new();
    let mut settings = Settings::default();
    let mut status = 0;
    loop {
        match editor.readline("> ") {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Some(outcome) = commands::handle(&mut session, line, &mut settings) {
                    if matches!(outcome, commands::Outcome::Quit) {
                        break;
                    }
                    continue;
                }
                let source = rewrite_assignment(line).unwrap_or_else(|| line.to_string());
                let started = Instant::now();
                match session.eval_source(&source) {
                    Ok(value) => {
                        for line in session.take_output() {
                            println!("{line}");
                        }
                        if matches!(session.view(value), Ok(ExprView::Symbol("null"))) {
                            continue;
                        }
                        let text = if settings.pretty {
                            format::value(&session, &source, value)
                        } else {
                            session
                                .format_infix(value)
                                .unwrap_or_else(|_| "?".to_string())
                        };
                        let elapsed = started.elapsed().as_secs_f64() * 1_000_000.0;
                        if settings.timing {
                            println!("{text} ({elapsed:.2} us)");
                        } else {
                            println!("{text}");
                        }
                    }
                    Err(error) => {
                        for line in session.take_output() {
                            println!("{line}");
                        }
                        eprintln!("error: {error}");
                    }
                }
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(error) => {
                eprintln!("mithqal: {error}");
                status = 1;
                break;
            }
        }
    }
    if let Some(path) = &history {
        let _ = editor.save_history(path);
    }
    status
}

#[cfg(test)]
mod tests {
    use super::rewrite_assignment;

    #[test]
    fn simple_assignments_are_rewritten() {
        assert_eq!(rewrite_assignment("x = 5").as_deref(), Some("set(x,5)"));
        assert_eq!(
            rewrite_assignment(" value = x + 1 ").as_deref(),
            Some("set(value,x + 1)")
        );
    }

    #[test]
    fn comparisons_are_not_rewritten() {
        assert_eq!(rewrite_assignment("x == 5"), None);
        assert_eq!(rewrite_assignment("x <= 5"), None);
        assert_eq!(rewrite_assignment("1 = 1"), None);
    }
}
