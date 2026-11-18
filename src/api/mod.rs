//! Public session and expression handle API.

use std::collections::HashMap;

use crate::config::Config;
use crate::diag::{Diagnostic, ErrorCode, Frame, Span};
use crate::eval::{SessionDefinitions, evaluate_with_definitions};
use crate::expr::{Arena, ExprId, Node, SymbolId};
use crate::hash::FnvBuildHasher;
use crate::num::{Int, Rat};
use crate::print;
use crate::runtime;
use crate::syntax::{self, Body, BodyStmt, Case, ExprSource, PatternSource, Stmt};

/// A view of an expression.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub enum ExprView<'a> {
    /// An exact integer.
    Integer(&'a Int),
    /// An exact rational.
    Rational(&'a Rat),
    /// An approximate real.
    Real(&'a crate::num::Real),
    /// A symbol.
    Symbol(&'a str),
    /// A call.
    Call {
        /// The head expression.
        head: ExprId,
        /// The argument expressions.
        args: &'a [ExprId],
    },
}

fn stale_handle() -> Diagnostic {
    Diagnostic::eval(ErrorCode::EvalNotLive, "expression handle is not live")
}

/// A symbolic math session.
pub struct Session {
    arena: Arena,
    definitions: SessionDefinitions,
    last_result: Option<ExprId>,
    parse_cache: HashMap<String, ExprId, FnvBuildHasher>,
    compiled_cache: HashMap<ExprId, runtime::Code, FnvBuildHasher>,
    eval_cache: HashMap<(ExprId, u64), ExprId, FnvBuildHasher>,
    compiled_results: HashMap<(ExprId, u64), ExprId, FnvBuildHasher>,
    output: Vec<String>,
    language_scopes: Vec<HashMap<SymbolId, ExprId, FnvBuildHasher>>,
    source_name: Option<String>,
    definition_source: Option<u32>,
    structures: HashMap<String, Vec<String>>,
    config: Config,
}

mod session;

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
