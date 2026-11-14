//! Structural pattern matching and substitution.

use std::collections::HashMap;

use crate::expr::{Arena, Attributes, ExprId, Node, SymbolId};

#[derive(Clone, Debug)]
pub(crate) enum Binding {
    One(ExprId),
    Seq(Vec<ExprId>),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Bindings {
    map: HashMap<SymbolId, Binding>,
}

impl Bindings {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get(&self, symbol: SymbolId) -> Option<&Binding> {
        self.map.get(&symbol)
    }
}

pub(crate) fn match_pattern(
    arena: &Arena,
    pattern: ExprId,
    value: ExprId,
    bindings: &mut Bindings,
    conditions: &mut Vec<ExprId>,
) -> bool {
    match arena.node(pattern) {
        Node::Int(_) | Node::Rat(_) | Node::Real(_) | Node::Symbol(_) => pattern == value,
        Node::Call { head, args } => {
            let name = head_name(arena, *head);
            match name {
                Some("blank") => true,
                Some("blankSequence") | Some("blankNullSequence") => false,
                Some("condition") if args.len() == 2 => {
                    if match_pattern(arena, args[0], value, bindings, conditions) {
                        conditions.push(args[1]);
                        true
                    } else {
                        false
                    }
                }
                Some("pattern") if args.len() == 2 => {
                    let Some(symbol) = arena.symbol_id(args[0]) else {
                        return false;
                    };
                    if let Some(existing) = bindings.get(symbol) {
                        let matches = match existing {
                            Binding::One(bound) => *bound == value,
                            Binding::Seq(_) => false,
                        };
                        if !matches {
                            return false;
                        }
                    }
                    if !match_pattern(arena, args[1], value, bindings, conditions) {
                        return false;
                    }
                    bindings.map.insert(symbol, Binding::One(value));
                    true
                }
                _ => {
                    let Node::Call {
                        head: value_head,
                        args: value_args,
                    } = arena.node(value)
                    else {
                        return false;
                    };
                    if *value_head != *head {
                        return false;
                    }
                    match_arguments(arena, *head, args, value_args, bindings, conditions)
                }
            }
        }
    }
}

fn match_arguments(
    arena: &Arena,
    head: ExprId,
    patterns: &[ExprId],
    values: &[ExprId],
    bindings: &mut Bindings,
    conditions: &mut Vec<ExprId>,
) -> bool {
    let orderless = arena.head_attributes(head).has(Attributes::ORDERLESS);
    if !orderless {
        return match_sequential(arena, patterns, values, bindings, conditions);
    }
    if patterns
        .iter()
        .any(|pattern| sequence_info(arena, *pattern).is_some())
    {
        let mut patterns = patterns.to_vec();
        let mut values = values.to_vec();
        patterns.sort_by(|left, right| arena.canonical_cmp(*left, *right));
        values.sort_by(|left, right| arena.canonical_cmp(*left, *right));
        return match_sequential(arena, &patterns, &values, bindings, conditions);
    }
    let mut used = vec![false; values.len()];
    match_orderless(arena, patterns, values, &mut used, bindings, conditions)
}

fn match_orderless(
    arena: &Arena,
    patterns: &[ExprId],
    values: &[ExprId],
    used: &mut [bool],
    bindings: &mut Bindings,
    conditions: &mut Vec<ExprId>,
) -> bool {
    let Some((pattern, rest)) = patterns.split_first() else {
        return used.iter().all(|value| *value);
    };
    for (index, value) in values.iter().enumerate() {
        if used[index] {
            continue;
        }
        let mut next_bindings = bindings.clone();
        let mut next_conditions = conditions.clone();
        if match_pattern(
            arena,
            *pattern,
            *value,
            &mut next_bindings,
            &mut next_conditions,
        ) {
            used[index] = true;
            if match_orderless(
                arena,
                rest,
                values,
                used,
                &mut next_bindings,
                &mut next_conditions,
            ) {
                *bindings = next_bindings;
                *conditions = next_conditions;
                return true;
            }
            used[index] = false;
        }
    }
    false
}

fn match_sequential(
    arena: &Arena,
    patterns: &[ExprId],
    values: &[ExprId],
    bindings: &mut Bindings,
    conditions: &mut Vec<ExprId>,
) -> bool {
    let Some((pattern, rest)) = patterns.split_first() else {
        return values.is_empty();
    };
    if let Some((name, minimum)) = sequence_info(arena, *pattern) {
        let remaining = values.len();
        if remaining < minimum {
            return false;
        }
        for count in minimum..=remaining {
            let mut next_bindings = bindings.clone();
            let mut next_conditions = conditions.clone();
            if let Some(symbol) = name {
                next_bindings
                    .map
                    .insert(symbol, Binding::Seq(values[..count].to_vec()));
            }
            if match_sequential(
                arena,
                rest,
                &values[count..],
                &mut next_bindings,
                &mut next_conditions,
            ) {
                *bindings = next_bindings;
                *conditions = next_conditions;
                return true;
            }
        }
        return false;
    }
    let Some((value, rest_values)) = values.split_first() else {
        return false;
    };
    let mut next_bindings = bindings.clone();
    let mut next_conditions = conditions.clone();
    if match_pattern(
        arena,
        *pattern,
        *value,
        &mut next_bindings,
        &mut next_conditions,
    ) && match_sequential(
        arena,
        rest,
        rest_values,
        &mut next_bindings,
        &mut next_conditions,
    ) {
        *bindings = next_bindings;
        *conditions = next_conditions;
        true
    } else {
        false
    }
}

fn sequence_info(arena: &Arena, pattern: ExprId) -> Option<(Option<SymbolId>, usize)> {
    let Node::Call { head, args } = arena.node(pattern) else {
        return None;
    };
    match head_name(arena, *head) {
        Some("blankSequence") => Some((None, 1)),
        Some("blankNullSequence") => Some((None, 0)),
        Some("pattern") if args.len() == 2 => {
            let symbol = arena.symbol_id(args[0]);
            let inner = args[1];
            match sequence_info(arena, inner) {
                Some((_, minimum)) => Some((symbol, minimum)),
                None => None,
            }
        }
        _ => None,
    }
}

pub(crate) fn substitute(arena: &mut Arena, expression: ExprId, bindings: &Bindings) -> ExprId {
    match arena.node(expression) {
        Node::Symbol(symbol) => match bindings.get(*symbol) {
            Some(Binding::One(value)) => *value,
            _ => expression,
        },
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => expression,
        Node::Call { head, args } => {
            let head = *head;
            let args = args.to_vec();
            let head = substitute(arena, head, bindings);
            let mut replaced = Vec::with_capacity(args.len());
            for arg in args {
                if let Node::Symbol(symbol) = arena.node(arg) {
                    if let Some(Binding::Seq(values)) = bindings.get(*symbol) {
                        replaced.extend_from_slice(values);
                        continue;
                    }
                }
                replaced.push(substitute(arena, arg, bindings));
            }
            arena.intern_call(head, &replaced)
        }
    }
}

fn head_name(arena: &Arena, head: ExprId) -> Option<&str> {
    let symbol = arena.symbol_id(head)?;
    Some(arena.symbol_name(symbol))
}
