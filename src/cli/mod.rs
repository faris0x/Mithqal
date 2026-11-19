//! Command line frontend.

#[allow(unused_imports)]
use crate::prelude::*;

mod args;
mod commands;
mod completion;
mod format;
mod license;
mod repl;
mod settings;

/// Run the command line frontend with the arguments after the program name.
pub fn run(arguments: Vec<String>) -> i32 {
    args::run(arguments)
}
