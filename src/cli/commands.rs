//! Interactive commands.

use crate::Session;

use super::{catalog, license};

pub(super) enum Outcome {
    Continue,
    Quit,
}

pub(super) fn help() -> &'static str {
    "commands:
  help                  show this help
  help FUNCTION         show function information
  list functions [TEXT] list functions, optionally filtered
  license               show the license notice
  quit                  exit the calculator

Type an expression and press Enter. Tab completes commands and function names.
Assignments use name = expression."
}

pub(super) fn handle(session: &mut Session, source: &str) -> Option<Outcome> {
    let source = source.trim();
    match source {
        "quit" | "exit" => return Some(Outcome::Quit),
        "help" | "?" => {
            println!("{}", help());
            return Some(Outcome::Continue);
        }
        "license" => {
            license::print_notice();
            return Some(Outcome::Continue);
        }
        _ => {}
    }
    if let Some(name) = source.strip_prefix("help ") {
        show_function(name.trim());
        return Some(Outcome::Continue);
    }
    if let Some(query) = source.strip_prefix("list functions") {
        list_functions(query.trim());
        return Some(Outcome::Continue);
    }
    let _ = session;
    None
}

fn show_function(name: &str) {
    match catalog::find(name) {
        Some(function) => println!(
            "{}\n  {}\n  {}\n  example: {}",
            function.name, function.signature, function.summary, function.example
        ),
        None => eprintln!("error: no function named {name}"),
    }
}

fn list_functions(query: &str) {
    if !query.is_empty() {
        for function in catalog::FUNCTIONS
            .iter()
            .filter(|function| catalog::matches(function, query))
        {
            println!("  {:<34} {}", function.signature, function.summary);
        }
        return;
    }
    for category in catalog::CATEGORIES {
        println!("{category}");
        let names = catalog::FUNCTIONS
            .iter()
            .filter(|function| function.category == *category)
            .map(|function| function.name)
            .collect::<Vec<_>>()
            .join(", ");
        println!("  {names}");
    }
}
