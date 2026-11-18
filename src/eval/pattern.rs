//! Structural pattern matching and substitution.

use std::collections::{HashMap, HashSet};

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
            sequence_info(arena, inner).map(|(_, minimum)| (symbol, minimum))
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
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => expression,
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
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => expression,
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
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => {}
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
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => {}
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
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => expression,
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
    let symbol = arena.symbol_or_intern(name);
    arena.symbol_expr(symbol)
}

fn head_name(arena: &Arena, head: ExprId) -> Option<&str> {
    let symbol = arena.symbol_id(head)?;
    Some(arena.symbol_name(symbol))
}
