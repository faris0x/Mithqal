//! License notice for Mithqal and its dependencies.

#[allow(unused_imports)]
use crate::prelude::*;

pub(super) const NOTICE: &str = "\
Mithqal is licensed under the GNU GPL version 3.0 License only.
Copyright (c) 2026 Faris Alfarhan

Numeric library:
  Mithqalite  GNU LGPL version 3.0 only

Rust dependencies:
  rustyline  MIT
  Transitive Rust crates are listed in license/LICENSES.md.

See license/LICENSES.md for the complete list of dependencies, their licenses,
and repository links. The license texts are stored in the license directory.";

pub(super) fn print_notice() {
    println!("{NOTICE}");
}
