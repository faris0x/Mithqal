//! Own values, down values, up values, and definition scopes.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::collections::HashMap;

use crate::diag::{Diagnostic, ErrorCode, Span};
use crate::expr::{Arena, Attributes, ExprId, Node, SymbolId};

/// An own value attached to a symbol.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OwnValue {
    pub(crate) value: ExprId,
}

/// The source location of a definition.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DefinitionSite {
    pub(crate) span: Span,
    pub(crate) source: u32,
}

/// A down value rule.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rule {
    pub(crate) lhs: ExprId,
    pub(crate) rhs: ExprId,
    pub(crate) delayed: bool,
    pub(crate) site: Option<DefinitionSite>,
}

/// Session definitions.
#[derive(Default)]
pub(crate) struct Definitions {
    own: HashMap<SymbolId, OwnValue>,
    down: HashMap<SymbolId, Vec<Rule>>,
    up: HashMap<SymbolId, Vec<Rule>>,
    local: Vec<HashMap<SymbolId, Vec<Rule>>>,
    sources: Vec<String>,
    version: u64,
}

impl Definitions {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn own_value(&self, symbol: SymbolId) -> Option<OwnValue> {
        self.own.get(&symbol).copied()
    }

    /// The rules for a symbol. The innermost local frame shadows outer frames
    /// and the global definitions entirely.
    pub(crate) fn up_rules(&self, symbol: SymbolId) -> Vec<Rule> {
        self.up.get(&symbol).cloned().unwrap_or_default()
    }

    pub(crate) fn rules(&self, symbol: SymbolId) -> Vec<Rule> {
        for frame in self.local.iter().rev() {
            if let Some(rules) = frame.get(&symbol) {
                return rules.clone();
            }
        }
        self.down.get(&symbol).cloned().unwrap_or_default()
    }

    pub(crate) fn version(&self) -> u64 {
        self.version
    }

    pub(crate) fn bump_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    /// Intern a source name and return its identifier.
    pub(crate) fn intern_source(&mut self, name: &str) -> u32 {
        if let Some(index) = self.sources.iter().position(|source| source == name) {
            return index as u32;
        }
        self.sources.push(name.to_string());
        (self.sources.len() - 1) as u32
    }

    /// The name of an interned source.
    pub(crate) fn source_name(&self, id: u32) -> &str {
        &self.sources[id as usize]
    }

    /// Attach a source location to the most recent rule for a symbol.
    pub(crate) fn annotate_last(
        &mut self,
        arena: &Arena,
        lhs: ExprId,
        local: bool,
        site: DefinitionSite,
    ) {
        let Some(symbol) = target_symbol(arena, lhs) else {
            return;
        };
        let rules = if local {
            self.local
                .last_mut()
                .and_then(|frame| frame.get_mut(&symbol))
        } else {
            self.down.get_mut(&symbol)
        };
        if let Some(rule) = rules.and_then(|rules| rules.last_mut()) {
            rule.site = Some(site);
        }
    }

    /// Enter a lexical definition scope.
    pub(crate) fn push_scope(&mut self) {
        self.local.push(HashMap::new());
    }

    /// Leave the innermost lexical definition scope.
    pub(crate) fn pop_scope(&mut self) {
        if let Some(frame) = self.local.pop()
            && !frame.is_empty()
        {
            self.version = self.version.wrapping_add(1);
        }
    }

    /// True when the rules for a symbol accept the argument count.
    ///
    /// A function with any sequence parameter is variadic and is not checked.
    /// A symbol with no rules is not checked.
    pub(crate) fn accepts_arity(&self, arena: &Arena, symbol: SymbolId, arity: usize) -> bool {
        let rules = self.rules(symbol);
        if rules.is_empty() {
            return true;
        }
        let mut fixed = Vec::new();
        for rule in &rules {
            match fixed_arity(arena, rule.lhs) {
                Some(count) => {
                    if !fixed.contains(&count) {
                        fixed.push(count);
                    }
                }
                None => return true,
            }
        }
        fixed.contains(&arity)
    }

    /// Remove and return the global rules for a symbol.
    pub(crate) fn take_rules(&mut self, symbol: SymbolId) -> Vec<Rule> {
        self.down.remove(&symbol).unwrap_or_default()
    }

    /// Install global rules for a symbol.
    pub(crate) fn install_rules(&mut self, symbol: SymbolId, rules: Vec<Rule>) {
        if rules.is_empty() {
            return;
        }
        self.down.entry(symbol).or_default().extend(rules);
        self.version = self.version.wrapping_add(1);
    }

    /// Remove and return the own value for a symbol.
    pub(crate) fn take_own(&mut self, symbol: SymbolId) -> Option<OwnValue> {
        self.own.remove(&symbol)
    }

    /// Install an own value for a symbol.
    pub(crate) fn install_own(&mut self, symbol: SymbolId, value: OwnValue) {
        self.own.insert(symbol, value);
        self.version = self.version.wrapping_add(1);
    }

    pub(crate) fn unset_symbol(&mut self, symbol: SymbolId) {
        self.own.remove(&symbol);
        self.version = self.version.wrapping_add(1);
    }

    pub(crate) fn unset_rules(&mut self, symbol: SymbolId) {
        self.down.remove(&symbol);
        self.up.remove(&symbol);
        for frame in &mut self.local {
            frame.remove(&symbol);
        }
        self.version = self.version.wrapping_add(1);
    }

    pub(crate) fn clear_symbol(&mut self, symbol: SymbolId) {
        self.own.remove(&symbol);
        self.down.remove(&symbol);
        self.up.remove(&symbol);
        for frame in &mut self.local {
            frame.remove(&symbol);
        }
        self.version = self.version.wrapping_add(1);
    }

    pub(crate) fn roots(&self, roots: &mut Vec<ExprId>) {
        for own in self.own.values() {
            roots.push(own.value);
        }
        for rules in self.down.values() {
            for rule in rules {
                roots.push(rule.lhs);
                roots.push(rule.rhs);
            }
        }
        for rules in self.up.values() {
            for rule in rules {
                roots.push(rule.lhs);
                roots.push(rule.rhs);
            }
        }
        for frame in &self.local {
            for rules in frame.values() {
                for rule in rules {
                    roots.push(rule.lhs);
                    roots.push(rule.rhs);
                }
            }
        }
    }

    pub(crate) fn define(
        &mut self,
        arena: &Arena,
        lhs: ExprId,
        rhs: ExprId,
        delayed: bool,
    ) -> Result<(), Diagnostic> {
        match arena.node(lhs) {
            Node::Symbol(symbol) => {
                self.check_protected(arena, *symbol)?;
                self.own.insert(*symbol, OwnValue { value: rhs });
                self.version = self.version.wrapping_add(1);
                Ok(())
            }
            Node::Call { .. } => {
                let symbol = target_symbol(arena, lhs).ok_or_else(|| {
                    Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "a definition head must be a symbol",
                    )
                })?;
                self.check_protected(arena, symbol)?;
                let rule = Rule {
                    lhs,
                    rhs,
                    delayed,
                    site: None,
                };
                self.down.entry(symbol).or_default().push(rule);
                self.version = self.version.wrapping_add(1);
                Ok(())
            }
            _ => Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                "a definition target must be a symbol or a call",
            )),
        }
    }

    pub(crate) fn define_up(
        &mut self,
        arena: &Arena,
        tag: SymbolId,
        lhs: ExprId,
        rhs: ExprId,
        delayed: bool,
    ) -> Result<(), Diagnostic> {
        self.check_protected(arena, tag)?;
        self.up.entry(tag).or_default().push(Rule {
            lhs,
            rhs,
            delayed,
            site: None,
        });
        self.version = self.version.wrapping_add(1);
        Ok(())
    }

    /// Install a delayed rule in the innermost lexical scope.
    pub(crate) fn define_local(
        &mut self,
        arena: &Arena,
        lhs: ExprId,
        rhs: ExprId,
    ) -> Result<(), Diagnostic> {
        let symbol = target_symbol(arena, lhs).ok_or_else(|| {
            Diagnostic::eval(
                ErrorCode::EvalDefinition,
                "a local definition must define a function",
            )
        })?;
        self.check_protected(arena, symbol)?;
        let Some(frame) = self.local.last_mut() else {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                "a local definition requires an enclosing scope",
            ));
        };
        frame.entry(symbol).or_default().push(Rule {
            lhs,
            rhs,
            delayed: true,
            site: None,
        });
        self.version = self.version.wrapping_add(1);
        Ok(())
    }

    fn check_protected(&self, arena: &Arena, symbol: SymbolId) -> Result<(), Diagnostic> {
        let name = arena.symbol_name(symbol);
        if arena.symbol_attributes(symbol).has(Attributes::PROTECTED)
            || crate::eval::is_kernel_head(name)
        {
            Err(Diagnostic::eval(
                ErrorCode::EvalProtected,
                format!("symbol {name} is protected"),
            ))
        } else {
            Ok(())
        }
    }
}

fn target_symbol(arena: &Arena, lhs: ExprId) -> Option<SymbolId> {
    let Node::Call { head, args } = arena.node(lhs) else {
        return None;
    };
    let target = if arena
        .symbol_id(*head)
        .map(|symbol| arena.symbol_name(symbol))
        == Some("condition")
    {
        match args.first().map(|argument| arena.node(*argument)) {
            Some(Node::Call { head: inner, .. }) => *inner,
            _ => *head,
        }
    } else {
        *head
    };
    arena.symbol_id(target)
}

fn fixed_arity(arena: &Arena, lhs: ExprId) -> Option<usize> {
    let lhs = match arena.node(lhs) {
        Node::Call { head, args }
            if args.len() == 2
                && arena
                    .symbol_id(*head)
                    .map(|symbol| arena.symbol_name(symbol))
                    == Some("condition") =>
        {
            args[0]
        }
        _ => lhs,
    };
    let Node::Call { args, .. } = arena.node(lhs) else {
        return None;
    };
    if args
        .iter()
        .any(|argument| is_sequence_pattern(arena, *argument))
    {
        return None;
    }
    Some(args.len())
}

fn is_sequence_pattern(arena: &Arena, pattern: ExprId) -> bool {
    let Node::Call { head, args } = arena.node(pattern) else {
        return false;
    };
    match arena
        .symbol_id(*head)
        .map(|symbol| arena.symbol_name(symbol))
    {
        Some("blankSequence")
        | Some("blankNullSequence")
        | Some("repeated")
        | Some("repeatedNull")
        | Some("optional")
        | Some("longest")
        | Some("shortest") => true,
        Some("pattern") if args.len() == 2 => is_sequence_pattern(arena, args[1]),
        _ => false,
    }
}
