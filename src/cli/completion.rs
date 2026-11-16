//! Tab completion for the interactive calculator.

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Result};

use super::catalog;

pub(super) struct CliHelper;

impl Completer for CliHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        position: usize,
        _context: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>)> {
        let start = line[..position]
            .rfind(|character: char| !(character.is_alphanumeric() || character == '_'))
            .map_or(0, |index| index + 1);
        let prefix = &line[start..position];
        if prefix.is_empty() {
            return Ok((start, Vec::new()));
        }
        let mut names = catalog::FUNCTIONS
            .iter()
            .map(|function| function.name)
            .collect::<Vec<_>>();
        names.extend(["help", "list", "license", "quit", "exit"]);
        names.sort_unstable();
        names.dedup();
        let candidates = names
            .into_iter()
            .filter(|name| name.starts_with(prefix))
            .map(|name| Pair {
                display: name.to_string(),
                replacement: name.to_string(),
            })
            .collect();
        Ok((start, candidates))
    }
}

impl Hinter for CliHelper {
    type Hint = String;
}

impl Highlighter for CliHelper {}

impl Validator for CliHelper {}

impl Helper for CliHelper {}
