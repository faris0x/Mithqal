//! Interactive commands.

use crate::Session;

use crate::catalog;

use super::{license, settings};

pub(super) enum Outcome {
    Continue,
    Quit,
}

pub(super) fn help() -> &'static str {
    "commands:
  help                  show this help
  help FUNCTION         show function information
  list functions [TEXT] list functions, optionally filtered
  list capabilities [TEXT]
                        show the support matrix
  set                   show settings
  set timing on|off     toggle processing time
  set pretty on|off     toggle formatted output
  license               show the license notice
  quit                  exit the calculator

Type an expression and press Enter. Tab completes commands and function names.
Assignments use name = expression."
}

pub(super) fn handle(
    session: &mut Session,
    source: &str,
    settings: &mut settings::Settings,
) -> Option<Outcome> {
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
        "set" | "settings" => {
            settings::show(settings);
            return Some(Outcome::Continue);
        }
        _ => {}
    }
    if let Some(rest) = source.strip_prefix("set ") {
        settings::set(rest, settings);
        return Some(Outcome::Continue);
    }
    if let Some(name) = source.strip_prefix("help ") {
        show_function(name.trim());
        return Some(Outcome::Continue);
    }
    if let Some(query) = source.strip_prefix("list functions") {
        list_functions(query.trim());
        return Some(Outcome::Continue);
    }
    if let Some(query) = source.strip_prefix("list capabilities") {
        list_capabilities(query.trim());
        return Some(Outcome::Continue);
    }
    let _ = session;
    None
}

fn show_function(name: &str) {
    match catalog::find(name) {
        Some(function) => {
            let capability = function.capability;
            println!(
                "{}\n  {}\n  {}\n  support: {}\n  domain: {}\n  symbolic: {}\n  approx: {}\n  errors: {}\n  display: {}\n  example: {}",
                function.name,
                function.signature,
                function.summary,
                capability.support.label(),
                capability.domain,
                capability.symbolic,
                capability.approximate,
                capability.errors,
                capability.display.label(),
                function.example
            );
        }
        None => eprintln!("error: no function named {name}"),
    }
}

fn list_functions(query: &str) {
    if !query.is_empty() {
        for function in catalog::FUNCTIONS
            .iter()
            .filter(|function| catalog::matches(function, query))
        {
            println!(
                "  {:<34} [{}] {}",
                function.signature,
                function.capability.support.label(),
                function.summary
            );
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
    println!();
    println!("Use list capabilities for support details.");
}

fn list_capabilities(query: &str) {
    let mut shown = 0;
    for function in catalog::FUNCTIONS
        .iter()
        .filter(|function| query.is_empty() || catalog::matches(function, query))
    {
        let capability = function.capability;
        println!(
            "{} [{}]\n  {}\n  domain: {}\n  symbolic: {}\n  approx: {}\n  errors: {}\n  display: {}\n",
            function.name,
            capability.support.label(),
            function.signature,
            capability.domain,
            capability.symbolic,
            capability.approximate,
            capability.errors,
            capability.display.label()
        );
        shown += 1;
    }
    if shown == 0 {
        eprintln!("error: no functions match {query}");
    }
}
