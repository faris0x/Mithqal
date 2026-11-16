//! Command line frontend.

mod args;
mod catalog;
mod commands;
mod completion;
mod license;
mod repl;

/// Run the command line frontend with the arguments after the program name.
pub fn run(arguments: Vec<String>) -> i32 {
    args::run(arguments)
}
