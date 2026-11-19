//! Structural pattern matching and substitution.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::collections::{HashMap, HashSet};

use crate::expr::{Arena, AssociationEntry, Attributes, ExprId, Node, SymbolId};

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

#[derive(Clone, Copy)]
struct SequenceInfo {
    name: Option<SymbolId>,
    min: usize,
    max: Option<usize>,
    longest: bool,
    optional_inner: Option<ExprId>,
    default: Option<ExprId>,
}

pub(crate) fn match_pattern(
    arena: &mut Arena,
    pattern: ExprId,
    value: ExprId,
    bindings: &mut Bindings,
    conditions: &mut Vec<ExprId>,
) -> bool {
    match arena.node(pattern) {
        Node::Int(_)
        | Node::Rat(_)
        | Node::Real(_)
        | Node::String(_)
        | Node::Bytes(_)
        | Node::Symbol(_)
        | Node::Association(_) => pattern == value,
        Node::Call { head, args } => {
            let head = *head;
            let args = args.to_vec();
            let name = head_name(arena, head);
            match name {
                Some("blank") => blank_matches(arena, value, &args),
                Some("blankSequence") | Some("blankNullSequence") => false,
                Some("alternatives") => {
                    for alternative in args.iter() {
                        let mut next_bindings = bindings.clone();
                        let mut next_conditions = conditions.clone();
                        if match_pattern(
                            arena,
                            *alternative,
                            value,
                            &mut next_bindings,
                            &mut next_conditions,
                        ) {
                            *bindings = next_bindings;
                            *conditions = next_conditions;
                            return true;
                        }
                    }
                    false
                }
                Some("patternTest") if args.len() == 2 => {
                    if !match_pattern(arena, args[0], value, bindings, conditions) {
                        return false;
                    }
                    let test = arena.intern_call(args[1], &[value]);
                    conditions.push(test);
                    true
                }
                Some("except") if args.len() == 1 => {
                    let mut ignored_bindings = bindings.clone();
                    let mut ignored_conditions = conditions.clone();
                    !match_pattern(
                        arena,
                        args[0],
                        value,
                        &mut ignored_bindings,
                        &mut ignored_conditions,
                    )
                }
                Some("verbatim") if args.len() == 1 => args[0] == value,
                Some("optional" | "repeated" | "repeatedNull" | "longest" | "shortest")
                    if !args.is_empty() =>
                {
                    match_pattern(arena, args[0], value, bindings, conditions)
                }
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
                    let (value_head, value_args) = match arena.node(value) {
                        Node::Call { head, args } => (*head, args.to_vec()),
                        _ => {
                            if arena.head_attributes(head).has(Attributes::ONE_IDENTITY)
                                && args.len() == 1
                            {
                                return match_pattern(arena, args[0], value, bindings, conditions);
                            }
                            return false;
                        }
                    };
                    if value_head != head {
                        return false;
                    }
                    match_arguments(arena, head, &args, &value_args, bindings, conditions)
                }
            }
        }
    }
}

fn blank_matches(arena: &Arena, value: ExprId, args: &[ExprId]) -> bool {
    match args {
        [] => true,
        [head] => type_matches(arena, value, *head),
        _ => false,
    }
}

fn type_matches(arena: &Arena, value: ExprId, head: ExprId) -> bool {
    let Some(symbol) = arena.symbol_id(head) else {
        return false;
    };
    let name = arena.symbol_name(symbol);
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "integer" => matches!(arena.node(value), Node::Int(_)),
        "rational" => matches!(arena.node(value), Node::Rat(_)),
        "real" => matches!(arena.node(value), Node::Real(_)),
        "number" => matches!(
            arena.node(value),
            Node::Int(_) | Node::Rat(_) | Node::Real(_)
        ),
        "string" => matches!(arena.node(value), Node::String(_)),
        "bytearray" => matches!(arena.node(value), Node::Bytes(_)),
        "association" => matches!(arena.node(value), Node::Association(_)),
        "symbol" => matches!(arena.node(value), Node::Symbol(_)),
        "list" => matches!(
            arena.node(value),
            Node::Call { head, .. } if arena.symbol_id(*head).map(|id| arena.symbol_name(id)) == Some("list")
        ),
        "call" => matches!(arena.node(value), Node::Call { .. }),
        _ => match arena.node(value) {
            Node::Call {
                head: value_head, ..
            } => *value_head == head,
            _ => false,
        },
    }
}

fn match_arguments(
    arena: &mut Arena,
    head: ExprId,
    patterns: &[ExprId],
    values: &[ExprId],
    bindings: &mut Bindings,
    conditions: &mut Vec<ExprId>,
) -> bool {
    let orderless = arena.head_attributes(head).has(Attributes::ORDERLESS);
    if orderless {
        let mut used = vec![false; values.len()];
        return match_orderless(arena, patterns, values, &mut used, bindings, conditions);
    }
    match_sequential(arena, patterns, values, bindings, conditions)
}

fn match_orderless(
    arena: &mut Arena,
    patterns: &[ExprId],
    values: &[ExprId],
    used: &mut [bool],
    bindings: &mut Bindings,
    conditions: &mut Vec<ExprId>,
) -> bool {
    let Some((pattern, rest)) = patterns.split_first() else {
        return used.iter().all(|value| *value);
    };
    let unused: Vec<usize> = used
        .iter()
        .enumerate()
        .filter_map(|(index, used)| (!used).then_some(index))
        .collect();
    if let Some(info) = sequence_info(arena, *pattern) {
        let maximum = info.max.unwrap_or(unused.len()).min(unused.len());
        if unused.len() < info.min {
            return false;
        }
        let mut counts: Vec<usize> = (info.min..=maximum).collect();
        if info.longest {
            counts.reverse();
        }
        for count in counts {
            let mut selected = Vec::with_capacity(count);
            let found = combinations(&unused, count, 0, &mut selected, &mut |selected| {
                let mut next_bindings = bindings.clone();
                let mut next_conditions = conditions.clone();
                if !bind_sequence(
                    arena,
                    &info,
                    selected,
                    values,
                    &mut next_bindings,
                    &mut next_conditions,
                ) {
                    return false;
                }
                for index in selected {
                    used[*index] = true;
                }
                let matched = match_orderless(
                    arena,
                    rest,
                    values,
                    used,
                    &mut next_bindings,
                    &mut next_conditions,
                );
                for index in selected {
                    used[*index] = false;
                }
                if matched {
                    *bindings = next_bindings;
                    *conditions = next_conditions;
                }
                matched
            });
            if found {
                return true;
            }
        }
        return false;
    }
    for &index in &unused {
        let mut next_bindings = bindings.clone();
        let mut next_conditions = conditions.clone();
        if match_pattern(
            arena,
            *pattern,
            values[index],
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
    arena: &mut Arena,
    patterns: &[ExprId],
    values: &[ExprId],
    bindings: &mut Bindings,
    conditions: &mut Vec<ExprId>,
) -> bool {
    let Some((pattern, rest)) = patterns.split_first() else {
        return values.is_empty();
    };
    if let Some(info) = sequence_info(arena, *pattern) {
        let maximum = info.max.unwrap_or(values.len()).min(values.len());
        if values.len() < info.min {
            return false;
        }
        let mut counts: Vec<usize> = (info.min..=maximum).collect();
        if info.longest {
            counts.reverse();
        }
        for count in counts {
            let mut next_bindings = bindings.clone();
            let mut next_conditions = conditions.clone();
            if !bind_sequence(
                arena,
                &info,
                &(0..count).collect::<Vec<_>>(),
                values,
                &mut next_bindings,
                &mut next_conditions,
            ) {
                continue;
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

fn bind_sequence(
    arena: &mut Arena,
    info: &SequenceInfo,
    selected: &[usize],
    values: &[ExprId],
    bindings: &mut Bindings,
    conditions: &mut Vec<ExprId>,
) -> bool {
    if let Some(inner) = info.optional_inner {
        if selected.len() == 1 {
            if !match_pattern(arena, inner, values[selected[0]], bindings, conditions) {
                return false;
            }
        } else if let Some(symbol) = info.name
            && let Some(default) = info.default
        {
            bindings.map.insert(symbol, Binding::One(default));
        }
        return true;
    }
    if let Some(symbol) = info.name {
        if selected.is_empty() {
            if let Some(default) = info.default {
                bindings.map.insert(symbol, Binding::One(default));
            } else {
                bindings.map.insert(symbol, Binding::Seq(Vec::new()));
            }
        } else {
            let bound: Vec<ExprId> = selected.iter().map(|index| values[*index]).collect();
            bindings.map.insert(symbol, Binding::Seq(bound));
        }
    }
    true
}

fn combinations(
    indices: &[usize],
    count: usize,
    start: usize,
    current: &mut Vec<usize>,
    callback: &mut impl FnMut(&[usize]) -> bool,
) -> bool {
    if count == 0 {
        return callback(current);
    }
    for index in start..=indices.len().saturating_sub(count) {
        current.push(indices[index]);
        if combinations(indices, count - 1, index + 1, current, callback) {
            return true;
        }
        current.pop();
    }
    false
}

fn pattern_name(arena: &Arena, pattern: ExprId) -> Option<SymbolId> {
    let Node::Call { head, args } = arena.node(pattern) else {
        return None;
    };
    if head_name(arena, *head) == Some("pattern") && args.len() == 2 {
        arena.symbol_id(args[0])
    } else {
        None
    }
}

fn sequence_info(arena: &Arena, pattern: ExprId) -> Option<SequenceInfo> {
    let Node::Call { head, args } = arena.node(pattern) else {
        return None;
    };
    let info = |name, min, max, longest| SequenceInfo {
        name,
        min,
        max,
        longest,
        optional_inner: None,
        default: None,
    };
    match head_name(arena, *head) {
        Some("blankSequence") => Some(info(None, 1, None, false)),
        Some("blankNullSequence") => Some(info(None, 0, None, false)),
        Some("repeated") => Some(info(pattern_name(arena, args[0]), 1, None, false)),
        Some("repeatedNull") => Some(info(pattern_name(arena, args[0]), 0, None, false)),
        Some("longest") if args.len() == 1 => sequence_info(arena, args[0]).map(|mut info| {
            info.longest = true;
            info
        }),
        Some("shortest") if args.len() == 1 => sequence_info(arena, args[0]).map(|mut info| {
            info.longest = false;
            info
        }),
        Some("optional") if !args.is_empty() && args.len() <= 2 => Some(SequenceInfo {
            name: pattern_name(arena, args[0]),
            min: 0,
            max: Some(1),
            longest: false,
            optional_inner: Some(args[0]),
            default: args.get(1).copied(),
        }),
        Some("pattern") if args.len() == 2 => {
            let symbol = arena.symbol_id(args[0]);
            sequence_info(arena, args[1]).map(|mut info| {
                if info.name.is_none() {
                    info.name = symbol;
                }
                info
            })
        }
        _ => None,
    }
}

pub(crate) fn substitute(arena: &mut Arena, expression: ExprId, bindings: &Bindings) -> ExprId {
    if bindings.map.is_empty() {
        return expression;
    }
    let blocked = if contains_binder(arena, expression) {
        binding_free_variables(arena, bindings)
    } else {
        HashSet::new()
    };
    substitute_in(arena, expression, bindings, &blocked)
}

fn substitute_in(
    arena: &mut Arena,
    expression: ExprId,
    bindings: &Bindings,
    blocked: &HashSet<SymbolId>,
) -> ExprId {
    match arena.node(expression) {
        Node::Symbol(symbol) => match bindings.get(*symbol) {
            Some(Binding::One(value)) => *value,
            _ => expression,
        },
        Node::Int(_) | Node::Rat(_) | Node::Real(_) | Node::String(_) | Node::Bytes(_) => {
            expression
        }
        Node::Association(entries) => {
            let entries = entries.to_vec();
            let entries = entries
                .iter()
                .map(|entry| AssociationEntry {
                    key: substitute_in(arena, entry.key, bindings, blocked),
                    value: substitute_in(arena, entry.value, bindings, blocked),
                })
                .collect();
            arena.intern_association(entries).unwrap_or(expression)
        }
        Node::Call { head, args } => {
            let head = *head;
            let args = args.to_vec();
            match head_name(arena, head) {
                Some("function") if args.len() == 2 => {
                    substitute_binder(arena, head, &args, bindings, blocked, false)
                }
                Some("closure") if args.len() == 3 => {
                    substitute_binder(arena, head, &args, bindings, blocked, true)
                }
                Some("let") if args.len() == 2 => {
                    substitute_let(arena, head, &args, bindings, blocked)
                }
                Some("rule" | "ruleDelayed") if args.len() == 2 => {
                    substitute_rule(arena, head, &args, bindings, blocked)
                }
                Some("condition") if args.len() == 2 => {
                    substitute_condition(arena, head, &args, bindings, blocked)
                }
                Some("pattern") if args.len() == 2 => {
                    substitute_pattern(arena, head, &args, bindings, blocked)
                }
                Some("localDefs") if args.len() == 2 => {
                    substitute_local_defs(arena, head, &args, bindings, blocked)
                }
                Some("forall" | "exists" | "setBuilder") if args.len() == 3 => {
                    substitute_quantifier(arena, head, &args, bindings, blocked)
                }
                _ => {
                    let head = substitute_in(arena, head, bindings, blocked);
                    let mut replaced = Vec::with_capacity(args.len());
                    for arg in args {
                        if let Node::Symbol(symbol) = arena.node(arg)
                            && let Some(Binding::Seq(values)) = bindings.get(*symbol)
                        {
                            replaced.extend_from_slice(values);
                            continue;
                        }
                        replaced.push(substitute_in(arena, arg, bindings, blocked));
                    }
                    arena.intern_call(head, &replaced)
                }
            }
        }
    }
}

fn substitute_binder(
    arena: &mut Arena,
    head: ExprId,
    args: &[ExprId],
    bindings: &Bindings,
    blocked: &HashSet<SymbolId>,
    closure: bool,
) -> ExprId {
    let params = parameter_symbols(arena, args[0]);
    let mut inner = bindings.clone();
    let mut names = params.clone();
    let mut renames = Vec::new();
    for (index, param) in params.iter().enumerate() {
        if blocked.contains(param) {
            let fresh = arena.fresh_symbol(*param);
            renames.push((*param, fresh));
            names[index] = fresh;
        }
        inner.map.remove(param);
    }
    let mut body = args[1];
    for (from, to) in &renames {
        body = rename_free(arena, body, *from, *to);
    }
    let body = substitute_in(arena, body, &inner, blocked);
    let params = symbol_list(arena, &names);
    if closure {
        let environment = substitute_in(arena, args[2], bindings, blocked);
        arena.intern_call(head, &[params, body, environment])
    } else {
        arena.intern_call(head, &[params, body])
    }
}

fn substitute_rule(
    arena: &mut Arena,
    head: ExprId,
    args: &[ExprId],
    bindings: &Bindings,
    blocked: &HashSet<SymbolId>,
) -> ExprId {
    let (lhs, rhs) = substitute_definition_pair(arena, args[0], args[1], bindings, blocked);
    arena.intern_call(head, &[lhs, rhs])
}

fn substitute_definition_pair(
    arena: &mut Arena,
    lhs: ExprId,
    rhs: ExprId,
    bindings: &Bindings,
    blocked: &HashSet<SymbolId>,
) -> (ExprId, ExprId) {
    let names = pattern_names(arena, lhs);
    let mut lhs = lhs;
    let mut rhs = rhs;
    let mut inner = bindings.clone();
    for name in &names {
        if blocked.contains(name) {
            let fresh = arena.fresh_symbol(*name);
            lhs = rename_symbol(arena, lhs, *name, fresh);
            rhs = rename_symbol(arena, rhs, *name, fresh);
        }
        inner.map.remove(name);
    }
    (
        substitute_in(arena, lhs, &inner, blocked),
        substitute_in(arena, rhs, &inner, blocked),
    )
}

fn substitute_local_defs(
    arena: &mut Arena,
    head: ExprId,
    args: &[ExprId],
    bindings: &Bindings,
    blocked: &HashSet<SymbolId>,
) -> ExprId {
    let Some(entries) = local_definition_parts(arena, args[0]) else {
        let entries = substitute_in(arena, args[0], bindings, blocked);
        let body = substitute_in(arena, args[1], bindings, blocked);
        return arena.intern_call(head, &[entries, body]);
    };
    let mut names = Vec::new();
    for (lhs, _) in &entries {
        if let Some(symbol) = rule_target(arena, *lhs)
            && !names.contains(&symbol)
        {
            names.push(symbol);
        }
    }
    let mut inner = bindings.clone();
    let mut renames = Vec::new();
    for name in &names {
        if blocked.contains(name) {
            renames.push((*name, arena.fresh_symbol(*name)));
        }
        inner.map.remove(name);
    }
    let mut rebuilt = Vec::with_capacity(entries.len());
    for (lhs, rhs) in entries {
        let mut lhs = lhs;
        let mut rhs = rhs;
        for (from, to) in &renames {
            lhs = rename_symbol(arena, lhs, *from, *to);
            rhs = rename_symbol(arena, rhs, *from, *to);
        }
        let (lhs, rhs) = substitute_definition_pair(arena, lhs, rhs, &inner, blocked);
        let set_delayed = head_expr(arena, "setDelayed");
        rebuilt.push(arena.intern_call(set_delayed, &[lhs, rhs]));
    }
    let mut body = args[1];
    for (from, to) in &renames {
        body = rename_symbol(arena, body, *from, *to);
    }
    let body = substitute_in(arena, body, &inner, blocked);
    let list = head_expr(arena, "list");
    let entries = arena.intern_call(list, &rebuilt);
    arena.intern_call(head, &[entries, body])
}

fn substitute_condition(
    arena: &mut Arena,
    head: ExprId,
    args: &[ExprId],
    bindings: &Bindings,
    blocked: &HashSet<SymbolId>,
) -> ExprId {
    let names = pattern_names(arena, args[0]);
    let mut pattern = args[0];
    let mut condition = args[1];
    let mut inner = bindings.clone();
    for name in &names {
        if blocked.contains(name) {
            let fresh = arena.fresh_symbol(*name);
            pattern = rename_symbol(arena, pattern, *name, fresh);
            condition = rename_symbol(arena, condition, *name, fresh);
        }
        inner.map.remove(name);
    }
    let pattern = substitute_in(arena, pattern, &inner, blocked);
    let condition = substitute_in(arena, condition, &inner, blocked);
    arena.intern_call(head, &[pattern, condition])
}

fn substitute_quantifier(
    arena: &mut Arena,
    head: ExprId,
    args: &[ExprId],
    bindings: &Bindings,
    blocked: &HashSet<SymbolId>,
) -> ExprId {
    let domain = substitute_in(arena, args[1], bindings, blocked);
    let mut variable = args[0];
    let mut body = args[2];
    let mut inner = bindings.clone();
    if let Some(symbol) = arena.symbol_id(args[0]) {
        if blocked.contains(&symbol) {
            let fresh = arena.fresh_symbol(symbol);
            variable = arena.symbol_expr(fresh);
            body = rename_symbol(arena, body, symbol, fresh);
        }
        inner.map.remove(&symbol);
    }
    let body = substitute_in(arena, body, &inner, blocked);
    arena.intern_call(head, &[variable, domain, body])
}

fn substitute_pattern(
    arena: &mut Arena,
    head: ExprId,
    args: &[ExprId],
    bindings: &Bindings,
    blocked: &HashSet<SymbolId>,
) -> ExprId {
    let mut name = args[0];
    let mut body = args[1];
    let mut inner = bindings.clone();
    if let Some(symbol) = arena.symbol_id(args[0]) {
        if blocked.contains(&symbol) {
            let fresh = arena.fresh_symbol(symbol);
            name = arena.symbol_expr(fresh);
            body = rename_symbol(arena, body, symbol, fresh);
        }
        inner.map.remove(&symbol);
    }
    let body = substitute_in(arena, body, &inner, blocked);
    arena.intern_call(head, &[name, body])
}

fn substitute_let(
    arena: &mut Arena,
    head: ExprId,
    args: &[ExprId],
    bindings: &Bindings,
    blocked: &HashSet<SymbolId>,
) -> ExprId {
    let Some(entries) = let_entries(arena, args[0]) else {
        let entries = substitute_in(arena, args[0], bindings, blocked);
        let body = substitute_in(arena, args[1], bindings, blocked);
        return arena.intern_call(head, &[entries, body]);
    };
    let mut inner = bindings.clone();
    let mut renames: Vec<(SymbolId, SymbolId)> = Vec::new();
    let mut rebuilt = Vec::with_capacity(entries.len());
    for (entry, name, rhs) in entries {
        let mut rhs = rhs;
        for (from, to) in &renames {
            rhs = rename_free(arena, rhs, *from, *to);
        }
        let rhs = substitute_in(arena, rhs, &inner, blocked);
        let mut name_out = name;
        if blocked.contains(&name) {
            let fresh = arena.fresh_symbol(name);
            renames.push((name, fresh));
            name_out = fresh;
        }
        inner.map.remove(&name);
        let name_expr = arena.symbol_expr(name_out);
        rebuilt.push(replace_set(arena, entry, name_expr, rhs));
    }
    let mut body = args[1];
    for (from, to) in &renames {
        body = rename_free(arena, body, *from, *to);
    }
    let body = substitute_in(arena, body, &inner, blocked);
    let list = head_expr(arena, "list");
    let entries = arena.intern_call(list, &rebuilt);
    arena.intern_call(head, &[entries, body])
}

fn replace_set(arena: &mut Arena, entry: ExprId, name: ExprId, rhs: ExprId) -> ExprId {
    let head = match arena.node(entry) {
        Node::Call { head, args } if args.len() == 2 => {
            if head_name(arena, *head) == Some("set") {
                Some(*head)
            } else {
                None
            }
        }
        _ => None,
    };
    let head = head.unwrap_or_else(|| head_expr(arena, "set"));
    arena.intern_call(head, &[name, rhs])
}

fn rename_free(arena: &mut Arena, expression: ExprId, from: SymbolId, to: SymbolId) -> ExprId {
    match arena.node(expression) {
        Node::Symbol(symbol) => {
            if *symbol == from {
                arena.symbol_expr(to)
            } else {
                expression
            }
        }
        Node::Int(_) | Node::Rat(_) | Node::Real(_) | Node::String(_) | Node::Bytes(_) => {
            expression
        }
        Node::Association(entries) => {
            let entries = entries.to_vec();
            let entries = entries
                .iter()
                .map(|entry| AssociationEntry {
                    key: rename_free(arena, entry.key, from, to),
                    value: rename_free(arena, entry.value, from, to),
                })
                .collect();
            arena.intern_association(entries).unwrap_or(expression)
        }
        Node::Call { head, args } => {
            let head = *head;
            let args = args.to_vec();
            match head_name(arena, head) {
                Some("function") if args.len() == 2 => {
                    if parameter_symbols(arena, args[0]).contains(&from) {
                        return expression;
                    }
                    let body = rename_free(arena, args[1], from, to);
                    arena.intern_call(head, &[args[0], body])
                }
                Some("closure") if args.len() == 3 => {
                    let environment = rename_free(arena, args[2], from, to);
                    if parameter_symbols(arena, args[0]).contains(&from) {
                        arena.intern_call(head, &[args[0], args[1], environment])
                    } else {
                        let body = rename_free(arena, args[1], from, to);
                        arena.intern_call(head, &[args[0], body, environment])
                    }
                }
                Some("let") if args.len() == 2 => rename_let(arena, head, &args, from, to),
                Some("rule" | "ruleDelayed") if args.len() == 2 => {
                    if pattern_names(arena, args[0]).contains(&from) {
                        return expression;
                    }
                    let lhs = rename_free(arena, args[0], from, to);
                    let rhs = rename_free(arena, args[1], from, to);
                    arena.intern_call(head, &[lhs, rhs])
                }
                Some("condition") if args.len() == 2 => {
                    if pattern_names(arena, args[0]).contains(&from) {
                        return expression;
                    }
                    let pattern = rename_free(arena, args[0], from, to);
                    let condition = rename_free(arena, args[1], from, to);
                    arena.intern_call(head, &[pattern, condition])
                }
                Some("pattern") if args.len() == 2 => {
                    if arena.symbol_id(args[0]) == Some(from) {
                        return expression;
                    }
                    let body = rename_free(arena, args[1], from, to);
                    arena.intern_call(head, &[args[0], body])
                }
                Some("localDefs") if args.len() == 2 => {
                    rename_local_defs(arena, head, &args, from, to)
                }
                Some("forall" | "exists" | "setBuilder") if args.len() == 3 => {
                    let domain = rename_free(arena, args[1], from, to);
                    if arena.symbol_id(args[0]) == Some(from) {
                        arena.intern_call(head, &[args[0], domain, args[2]])
                    } else {
                        let body = rename_free(arena, args[2], from, to);
                        arena.intern_call(head, &[args[0], domain, body])
                    }
                }
                _ => {
                    let head = rename_free(arena, head, from, to);
                    let args: Vec<ExprId> = args
                        .into_iter()
                        .map(|arg| rename_free(arena, arg, from, to))
                        .collect();
                    arena.intern_call(head, &args)
                }
            }
        }
    }
}

fn rename_let(
    arena: &mut Arena,
    head: ExprId,
    args: &[ExprId],
    from: SymbolId,
    to: SymbolId,
) -> ExprId {
    let Some(entries) = let_entries(arena, args[0]) else {
        return arena.intern_call(head, args);
    };
    let mut shadowed = false;
    let mut rebuilt = Vec::with_capacity(entries.len());
    for (entry, name, rhs) in entries {
        let rhs = if shadowed {
            rhs
        } else {
            rename_free(arena, rhs, from, to)
        };
        if name == from {
            shadowed = true;
        }
        let name_expr = arena.symbol_expr(name);
        rebuilt.push(replace_set(arena, entry, name_expr, rhs));
    }
    let body = if shadowed {
        args[1]
    } else {
        rename_free(arena, args[1], from, to)
    };
    let list = head_expr(arena, "list");
    let entries = arena.intern_call(list, &rebuilt);
    arena.intern_call(head, &[entries, body])
}

fn binding_free_variables(arena: &Arena, bindings: &Bindings) -> HashSet<SymbolId> {
    let mut free = HashSet::new();
    let mut bound = HashSet::new();
    for binding in bindings.map.values() {
        match binding {
            Binding::One(value) => collect_free(arena, *value, &mut bound, &mut free),
            Binding::Seq(values) => {
                for value in values {
                    collect_free(arena, *value, &mut bound, &mut free);
                }
            }
        }
    }
    free
}

fn collect_free(
    arena: &Arena,
    expression: ExprId,
    bound: &mut HashSet<SymbolId>,
    free: &mut HashSet<SymbolId>,
) {
    match arena.node(expression) {
        Node::Symbol(symbol) => {
            if !bound.contains(symbol) {
                free.insert(*symbol);
            }
        }
        Node::Int(_) | Node::Rat(_) | Node::Real(_) | Node::String(_) | Node::Bytes(_) => {}
        Node::Association(entries) => {
            for entry in entries {
                collect_free(arena, entry.key, bound, free);
                collect_free(arena, entry.value, bound, free);
            }
        }
        Node::Call { head, args } => {
            let head = *head;
            match head_name(arena, head) {
                Some("function") if args.len() == 2 => {
                    let added = bind_all(bound, &parameter_symbols(arena, args[0]));
                    collect_free(arena, args[1], bound, free);
                    release_all(bound, &added);
                }
                Some("closure") if args.len() == 3 => {
                    let added = bind_all(bound, &parameter_symbols(arena, args[0]));
                    collect_free(arena, args[1], bound, free);
                    release_all(bound, &added);
                    collect_free(arena, args[2], bound, free);
                }
                Some("let") if args.len() == 2 => {
                    if let Some(entries) = let_entries(arena, args[0]) {
                        let mut added = Vec::new();
                        for (_, name, rhs) in entries {
                            collect_free(arena, rhs, bound, free);
                            if bound.insert(name) {
                                added.push(name);
                            }
                        }
                        collect_free(arena, args[1], bound, free);
                        release_all(bound, &added);
                    } else {
                        collect_free(arena, args[0], bound, free);
                        collect_free(arena, args[1], bound, free);
                    }
                }
                Some("rule" | "ruleDelayed") if args.len() == 2 => {
                    let names = pattern_names(arena, args[0]);
                    collect_free(arena, args[0], bound, free);
                    let added = bind_all(bound, &names);
                    collect_free(arena, args[1], bound, free);
                    release_all(bound, &added);
                }
                Some("condition") if args.len() == 2 => {
                    let names = pattern_names(arena, args[0]);
                    collect_free(arena, args[0], bound, free);
                    let added = bind_all(bound, &names);
                    collect_free(arena, args[1], bound, free);
                    release_all(bound, &added);
                }
                Some("pattern") if args.len() == 2 => {
                    collect_free(arena, args[1], bound, free);
                }
                Some("forall" | "exists" | "setBuilder") if args.len() == 3 => {
                    collect_free(arena, args[1], bound, free);
                    let added = arena
                        .symbol_id(args[0])
                        .filter(|symbol| bound.insert(*symbol));
                    collect_free(arena, args[2], bound, free);
                    if let Some(symbol) = added {
                        bound.remove(&symbol);
                    }
                }
                Some("localDefs") if args.len() == 2 => {
                    if let Some(entries) = local_definition_parts(arena, args[0]) {
                        let mut names = Vec::new();
                        for (lhs, _) in &entries {
                            if let Some(symbol) = rule_target(arena, *lhs)
                                && !names.contains(&symbol)
                            {
                                names.push(symbol);
                            }
                        }
                        let added = bind_all(bound, &names);
                        for (lhs, rhs) in entries {
                            collect_free(arena, lhs, bound, free);
                            let parameters = pattern_names(arena, lhs);
                            let parameters = bind_all(bound, &parameters);
                            collect_free(arena, rhs, bound, free);
                            release_all(bound, &parameters);
                        }
                        collect_free(arena, args[1], bound, free);
                        release_all(bound, &added);
                    } else {
                        collect_free(arena, args[0], bound, free);
                        collect_free(arena, args[1], bound, free);
                    }
                }
                _ => {
                    collect_free(arena, head, bound, free);
                    for arg in args {
                        collect_free(arena, *arg, bound, free);
                    }
                }
            }
        }
    }
}

fn bind_all(bound: &mut HashSet<SymbolId>, symbols: &[SymbolId]) -> Vec<SymbolId> {
    let mut added = Vec::new();
    for symbol in symbols {
        if bound.insert(*symbol) {
            added.push(*symbol);
        }
    }
    added
}

fn release_all(bound: &mut HashSet<SymbolId>, symbols: &[SymbolId]) {
    for symbol in symbols {
        bound.remove(symbol);
    }
}

fn rename_local_defs(
    arena: &mut Arena,
    head: ExprId,
    args: &[ExprId],
    from: SymbolId,
    to: SymbolId,
) -> ExprId {
    let Some(entries) = local_definition_parts(arena, args[0]) else {
        return arena.intern_call(head, args);
    };
    let mut names = Vec::new();
    for (lhs, _) in &entries {
        if let Some(symbol) = rule_target(arena, *lhs)
            && !names.contains(&symbol)
        {
            names.push(symbol);
        }
    }
    if names.contains(&from) {
        return arena.intern_call(head, args);
    }
    let mut rebuilt = Vec::with_capacity(entries.len());
    for (lhs, rhs) in entries {
        let set_delayed = head_expr(arena, "setDelayed");
        if pattern_names(arena, lhs).contains(&from) {
            rebuilt.push(arena.intern_call(set_delayed, &[lhs, rhs]));
        } else {
            let lhs = rename_free(arena, lhs, from, to);
            let rhs = rename_free(arena, rhs, from, to);
            rebuilt.push(arena.intern_call(set_delayed, &[lhs, rhs]));
        }
    }
    let body = rename_free(arena, args[1], from, to);
    let list = head_expr(arena, "list");
    let entries = arena.intern_call(list, &rebuilt);
    arena.intern_call(head, &[entries, body])
}

fn local_definition_parts(arena: &Arena, expression: ExprId) -> Option<Vec<(ExprId, ExprId)>> {
    let Node::Call { head, args } = arena.node(expression) else {
        return None;
    };
    if head_name(arena, *head) != Some("list") {
        return None;
    }
    let mut entries = Vec::with_capacity(args.len());
    for entry in args.iter() {
        let Node::Call {
            head: entry_head,
            args: entry_args,
        } = arena.node(*entry)
        else {
            return None;
        };
        if head_name(arena, *entry_head) != Some("setDelayed") || entry_args.len() != 2 {
            return None;
        }
        entries.push((entry_args[0], entry_args[1]));
    }
    Some(entries)
}

fn rule_target(arena: &Arena, lhs: ExprId) -> Option<SymbolId> {
    let Node::Call { head, args } = arena.node(lhs) else {
        return None;
    };
    let target = if head_name(arena, *head) == Some("condition") {
        match args.first().map(|argument| arena.node(*argument)) {
            Some(Node::Call { head: inner, .. }) => *inner,
            _ => *head,
        }
    } else {
        *head
    };
    arena.symbol_id(target)
}

pub(crate) fn all_symbols(arena: &Arena, expression: ExprId, symbols: &mut HashSet<SymbolId>) {
    match arena.node(expression) {
        Node::Symbol(symbol) => {
            symbols.insert(*symbol);
        }
        Node::Int(_) | Node::Rat(_) | Node::Real(_) | Node::String(_) | Node::Bytes(_) => {}
        Node::Association(entries) => {
            for entry in entries {
                all_symbols(arena, entry.key, symbols);
                all_symbols(arena, entry.value, symbols);
            }
        }
        Node::Call { head, args } => {
            all_symbols(arena, *head, symbols);
            for argument in args {
                all_symbols(arena, *argument, symbols);
            }
        }
    }
}

fn pattern_names(arena: &Arena, expression: ExprId) -> Vec<SymbolId> {
    let mut names = Vec::new();
    collect_pattern_names(arena, expression, &mut names);
    names
}

fn collect_pattern_names(arena: &Arena, expression: ExprId, names: &mut Vec<SymbolId>) {
    let Node::Call { head, args } = arena.node(expression) else {
        return;
    };
    match head_name(arena, *head) {
        Some("pattern") if args.len() == 2 => {
            if let Some(symbol) = arena.symbol_id(args[0])
                && !names.contains(&symbol)
            {
                names.push(symbol);
            }
            collect_pattern_names(arena, args[1], names);
        }
        Some("condition") if args.len() == 2 => {
            collect_pattern_names(arena, args[0], names);
            collect_pattern_names(arena, args[1], names);
        }
        Some("rule" | "ruleDelayed") => {}
        _ => {
            for argument in args {
                collect_pattern_names(arena, *argument, names);
            }
        }
    }
}

fn rename_symbol(arena: &mut Arena, expression: ExprId, from: SymbolId, to: SymbolId) -> ExprId {
    match arena.node(expression) {
        Node::Symbol(symbol) => {
            if *symbol == from {
                arena.symbol_expr(to)
            } else {
                expression
            }
        }
        Node::Int(_) | Node::Rat(_) | Node::Real(_) | Node::String(_) | Node::Bytes(_) => {
            expression
        }
        Node::Association(entries) => {
            let entries = entries.to_vec();
            let entries = entries
                .iter()
                .map(|entry| AssociationEntry {
                    key: rename_symbol(arena, entry.key, from, to),
                    value: rename_symbol(arena, entry.value, from, to),
                })
                .collect();
            arena.intern_association(entries).unwrap_or(expression)
        }
        Node::Call { head, args } => {
            let head = *head;
            let args = args.to_vec();
            let head = rename_symbol(arena, head, from, to);
            let args: Vec<ExprId> = args
                .into_iter()
                .map(|argument| rename_symbol(arena, argument, from, to))
                .collect();
            arena.intern_call(head, &args)
        }
    }
}

fn contains_binder(arena: &Arena, expression: ExprId) -> bool {
    match arena.node(expression) {
        Node::Call { head, args } => {
            let head = *head;
            if let Some(
                "function" | "closure" | "let" | "rule" | "ruleDelayed" | "condition" | "pattern"
                | "localDefs" | "forall" | "exists" | "setBuilder",
            ) = head_name(arena, head)
            {
                return true;
            }
            contains_binder(arena, head) || args.iter().any(|arg| contains_binder(arena, *arg))
        }
        Node::Association(entries) => entries
            .iter()
            .any(|entry| contains_binder(arena, entry.key) || contains_binder(arena, entry.value)),
        _ => false,
    }
}

fn parameter_symbols(arena: &Arena, expression: ExprId) -> Vec<SymbolId> {
    match arena.node(expression) {
        Node::Call { head, args } if head_name(arena, *head) == Some("list") => args
            .iter()
            .filter_map(|arg| arena.symbol_id(*arg))
            .collect(),
        _ => Vec::new(),
    }
}

fn let_entries(arena: &Arena, expression: ExprId) -> Option<Vec<(ExprId, SymbolId, ExprId)>> {
    let Node::Call { head, args } = arena.node(expression) else {
        return None;
    };
    if head_name(arena, *head) != Some("list") {
        return None;
    }
    let mut entries = Vec::with_capacity(args.len());
    for entry in args.iter() {
        let Node::Call {
            head: entry_head,
            args: entry_args,
        } = arena.node(*entry)
        else {
            return None;
        };
        if head_name(arena, *entry_head) != Some("set") || entry_args.len() != 2 {
            return None;
        }
        let name = arena.symbol_id(entry_args[0])?;
        entries.push((*entry, name, entry_args[1]));
    }
    Some(entries)
}

fn symbol_list(arena: &mut Arena, symbols: &[SymbolId]) -> ExprId {
    let values: Vec<ExprId> = symbols
        .iter()
        .map(|symbol| arena.symbol_expr(*symbol))
        .collect();
    let head = head_expr(arena, "list");
    arena.intern_call(head, &values)
}

pub(crate) fn rename_expression(
    arena: &mut Arena,
    expression: ExprId,
    from: SymbolId,
    to: SymbolId,
) -> ExprId {
    let value = arena.symbol_expr(to);
    let mut bindings = Bindings::new();
    bindings.map.insert(from, Binding::One(value));
    substitute(arena, expression, &bindings)
}

pub(crate) fn rename_definition(
    arena: &mut Arena,
    lhs: ExprId,
    rhs: ExprId,
    from: SymbolId,
    to: SymbolId,
) -> (ExprId, ExprId) {
    let value = arena.symbol_expr(to);
    let mut bindings = Bindings::new();
    bindings.map.insert(from, Binding::One(value));
    substitute_definition_pair(arena, lhs, rhs, &bindings, &HashSet::new())
}

fn head_expr(arena: &mut Arena, name: &str) -> ExprId {
    let attrs = crate::builtins::attributes(name).unwrap_or_default();
    let symbol = arena.intern_symbol(name, attrs);
    arena.symbol_expr(symbol)
}

fn head_name(arena: &Arena, head: ExprId) -> Option<&str> {
    let symbol = arena.symbol_id(head)?;
    Some(arena.symbol_name(symbol))
}
