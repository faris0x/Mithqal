//! Argument handling for the command line frontend.

use crate::{ExprView, Session};

use super::{format, license, repl};

fn usage() -> &'static str {
    "usage: mithqal [options]

Options:
  -e, --eval <source>   evaluate a source string and print the result
      --license         print the license notice
  -h, --help            print this help
  -V, --version         print the version

Without options, Mithqal starts the interactive calculator."
}

pub(super) fn run(arguments: Vec<String>) -> i32 {
    if arguments
        .iter()
        .any(|argument| argument == "-h" || argument == "--help")
    {
        println!("{}", usage());
        return 0;
    }
    if arguments
        .iter()
        .any(|argument| argument == "-V" || argument == "--version")
    {
        println!("mithqal {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    if arguments.iter().any(|argument| argument == "--license") {
        license::print_notice();
        return 0;
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "-e" || argument == "--eval")
    {
        let Some(source) = arguments.get(index + 1) else {
            eprintln!("mithqal: --eval requires a source string");
            return 2;
        };
        return evaluate(source);
    }
    if let Some(argument) = arguments.first() {
        eprintln!("mithqal: unknown option {argument}");
        eprintln!("{}", usage());
        return 2;
    }
    repl::run()
}

fn evaluate(source: &str) -> i32 {
    let mut session = Session::new();
    match session.eval_source(source) {
        Ok(value) => {
            for line in session.take_output() {
                println!("{line}");
            }
            if !matches!(session.view(value), Ok(ExprView::Symbol("null"))) {
                println!("{}", format::value(&session, source, value));
            }
            0
        }
        Err(error) => {
            eprintln!("error: {error}");
            1
        }
    }
}
