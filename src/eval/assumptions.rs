//! Local assumption formulas and a compact inference engine.
//!
//! Assumptions are keyed by interned symbol identity. The engine normalizes
//! formulas to disjunctive normal form and reasons about signs, exact
//! comparisons, equality, domains, and parity within each branch.

#[allow(unused_imports)]
use crate::prelude::*;

use super::Sign;
use crate::expr::{Arena, ExprId, Node, SymbolId};
use crate::num::{Int, Rat};

const MAX_BRANCHES: usize = 256;

#[derive(Clone, Debug)]
pub(crate) struct Assumption {
    pub(crate) formula: Formula,
    pub(crate) source: ExprId,
}

#[derive(Clone, Debug)]
pub(crate) enum Formula {
    Atom(Atom),
    And(Vec<Formula>),
    Or(Vec<Formula>),
    Not(Box<Formula>),
}

#[derive(Clone, Debug)]
pub(crate) enum Atom {
    Compare {
        left: Term,
        op: CompareOp,
        right: Term,
    },
    Domain {
        symbol: SymbolId,
        domain: Domain,
        positive: bool,
    },
    Parity {
        symbol: SymbolId,
        parity: Parity,
        positive: bool,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum Term {
    Symbol(SymbolId),
    Constant(Rat),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CompareOp {
    Less,
    LessEqual,
    Equal,
    NotEqual,
    GreaterEqual,
    Greater,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Domain {
    Integer,
    Rational,
    Real,
    Complex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Parity {
    Even,
    Odd,
}

const DOMAIN_INTEGER: u8 = 1;
const DOMAIN_RATIONAL: u8 = 2;
const DOMAIN_REAL: u8 = 4;
const DOMAIN_COMPLEX: u8 = 8;
const PARITY_EVEN: u8 = 1;
const PARITY_ODD: u8 = 2;

pub(crate) fn parse(arena: &Arena, expression: ExprId) -> Option<Formula> {
    let Node::Call { head, args } = arena.node(expression) else {
        return None;
    };
    let name = arena
        .symbol_id(*head)
        .map(|symbol| arena.symbol_name(symbol))?;
    match name {
        "and" => Some(Formula::And(
            args.iter()
                .map(|arg| parse(arena, *arg))
                .collect::<Option<Vec<_>>>()?,
        )),
        "or" => Some(Formula::Or(
            args.iter()
                .map(|arg| parse(arena, *arg))
                .collect::<Option<Vec<_>>>()?,
        )),
        "not" if args.len() == 1 => Some(Formula::Not(Box::new(parse(arena, args[0])?))),
        "equal" | "notEqual" | "less" | "lessEqual" | "greater" | "greaterEqual" => {
            if args.len() != 2 {
                return None;
            }
            let op = match name {
                "equal" => CompareOp::Equal,
                "notEqual" => CompareOp::NotEqual,
                "less" => CompareOp::Less,
                "lessEqual" => CompareOp::LessEqual,
                "greater" => CompareOp::Greater,
                "greaterEqual" => CompareOp::GreaterEqual,
                _ => unreachable!(),
            };
            Some(Formula::Atom(Atom::Compare {
                left: parse_term(arena, args[0])?,
                op,
                right: parse_term(arena, args[1])?,
            }))
        }
        "integer" | "isInteger" => domain_atom(arena, args, Domain::Integer),
        "rational" | "isRational" => domain_atom(arena, args, Domain::Rational),
        "real" | "isReal" => domain_atom(arena, args, Domain::Real),
        "complex" | "isComplex" => domain_atom(arena, args, Domain::Complex),
        "even" | "isEven" => parity_atom(arena, args, Parity::Even),
        "odd" | "isOdd" => parity_atom(arena, args, Parity::Odd),
        "element" if args.len() == 2 => {
            let symbol = arena.symbol_id(args[0])?;
            let domain_name = arena
                .symbol_id(args[1])
                .map(|symbol| arena.symbol_name(symbol))?;
            let domain = match domain_name {
                "integers" | "integer" => Domain::Integer,
                "rationals" | "rational" => Domain::Rational,
                "reals" | "real" => Domain::Real,
                "complexes" | "complex" => Domain::Complex,
                _ => return None,
            };
            Some(Formula::Atom(Atom::Domain {
                symbol,
                domain,
                positive: true,
            }))
        }
        "nonzero" if args.len() == 1 => {
            let symbol = arena.symbol_id(args[0])?;
            Some(Formula::Atom(Atom::Compare {
                left: Term::Symbol(symbol),
                op: CompareOp::NotEqual,
                right: Term::Constant(Rat::zero()),
            }))
        }
        "positive" | "negative" | "nonnegative" | "nonpositive" if args.len() == 1 => {
            let symbol = arena.symbol_id(args[0])?;
            let op = match name {
                "positive" => CompareOp::Greater,
                "negative" => CompareOp::Less,
                "nonnegative" => CompareOp::GreaterEqual,
                "nonpositive" => CompareOp::LessEqual,
                _ => unreachable!(),
            };
            Some(Formula::Atom(Atom::Compare {
                left: Term::Symbol(symbol),
                op,
                right: Term::Constant(Rat::zero()),
            }))
        }
        _ => None,
    }
}

fn domain_atom(arena: &Arena, args: &[ExprId], domain: Domain) -> Option<Formula> {
    if args.len() != 1 {
        return None;
    }
    let symbol = arena.symbol_id(args[0])?;
    Some(Formula::Atom(Atom::Domain {
        symbol,
        domain,
        positive: true,
    }))
}

fn parity_atom(arena: &Arena, args: &[ExprId], parity: Parity) -> Option<Formula> {
    if args.len() != 1 {
        return None;
    }
    let symbol = arena.symbol_id(args[0])?;
    Some(Formula::Atom(Atom::Parity {
        symbol,
        parity,
        positive: true,
    }))
}

fn parse_term(arena: &Arena, expression: ExprId) -> Option<Term> {
    match arena.node(expression) {
        Node::Symbol(symbol) => Some(Term::Symbol(*symbol)),
        Node::Int(value) => Some(Term::Constant(
            Rat::from_ints(value.clone(), Int::one()).ok()?,
        )),
        Node::Rat(value) => Some(Term::Constant(value.clone())),
        _ => None,
    }
}

pub(crate) fn is_consistent(frames: &[Assumption]) -> bool {
    let Some(branches) = branches(frames) else {
        return false;
    };
    branches
        .iter()
        .any(|atoms| Branch::from_atoms(atoms).is_consistent())
}

pub(crate) fn sign(frames: &[Assumption], symbol: SymbolId) -> Option<Sign> {
    let branches = branches(frames)?;
    let mut result = None;
    for atoms in branches {
        let mut branch = Branch::from_atoms(&atoms);
        if !branch.is_consistent() {
            continue;
        }
        let value = branch.sign(symbol)?;
        result = Some(match result {
            None => value,
            Some(previous) => meet_sign(previous, value)?,
        });
    }
    result
}

pub(crate) fn sources(frames: &[Assumption], symbol: Option<SymbolId>) -> Vec<ExprId> {
    frames
        .iter()
        .filter(|assumption| symbol.is_none_or(|symbol| mentions(&assumption.formula, symbol)))
        .map(|assumption| assumption.source)
        .collect()
}

fn mentions(formula: &Formula, symbol: SymbolId) -> bool {
    match formula {
        Formula::Atom(atom) => match atom {
            Atom::Compare { left, right, .. } => {
                term_mentions(left, symbol) || term_mentions(right, symbol)
            }
            Atom::Domain {
                symbol: candidate, ..
            }
            | Atom::Parity {
                symbol: candidate, ..
            } => *candidate == symbol,
        },
        Formula::And(items) | Formula::Or(items) => {
            items.iter().any(|formula| mentions(formula, symbol))
        }
        Formula::Not(inner) => mentions(inner, symbol),
    }
}

fn term_mentions(term: &Term, symbol: SymbolId) -> bool {
    matches!(term, Term::Symbol(candidate) if *candidate == symbol)
}

fn branches(frames: &[Assumption]) -> Option<Vec<Vec<Atom>>> {
    let mut result = vec![Vec::new()];
    for frame in frames {
        let next = dnf(&frame.formula)?;
        result = combine(result, next)?;
    }
    Some(result)
}

fn dnf(formula: &Formula) -> Option<Vec<Vec<Atom>>> {
    dnf_nnf(&to_nnf(formula, false))
}

fn to_nnf(formula: &Formula, negated: bool) -> Formula {
    match formula {
        Formula::Atom(atom) => {
            if negated {
                Formula::Atom(negate_atom(atom))
            } else {
                Formula::Atom(atom.clone())
            }
        }
        Formula::And(items) => {
            let items = items
                .iter()
                .map(|item| to_nnf(item, negated))
                .collect::<Vec<_>>();
            if negated {
                Formula::Or(items)
            } else {
                Formula::And(items)
            }
        }
        Formula::Or(items) => {
            let items = items
                .iter()
                .map(|item| to_nnf(item, negated))
                .collect::<Vec<_>>();
            if negated {
                Formula::And(items)
            } else {
                Formula::Or(items)
            }
        }
        Formula::Not(inner) => to_nnf(inner, !negated),
    }
}

fn negate_atom(atom: &Atom) -> Atom {
    match atom {
        Atom::Compare { left, op, right } => Atom::Compare {
            left: left.clone(),
            op: match op {
                CompareOp::Less => CompareOp::GreaterEqual,
                CompareOp::LessEqual => CompareOp::Greater,
                CompareOp::Equal => CompareOp::NotEqual,
                CompareOp::NotEqual => CompareOp::Equal,
                CompareOp::GreaterEqual => CompareOp::Less,
                CompareOp::Greater => CompareOp::LessEqual,
            },
            right: right.clone(),
        },
        Atom::Domain {
            symbol,
            domain,
            positive,
        } => Atom::Domain {
            symbol: *symbol,
            domain: *domain,
            positive: !positive,
        },
        Atom::Parity {
            symbol,
            parity,
            positive,
        } => Atom::Parity {
            symbol: *symbol,
            parity: *parity,
            positive: !positive,
        },
    }
}

fn dnf_nnf(formula: &Formula) -> Option<Vec<Vec<Atom>>> {
    match formula {
        Formula::Atom(atom) => Some(vec![vec![atom.clone()]]),
        Formula::And(items) => {
            let mut result = vec![Vec::new()];
            for item in items {
                result = combine(result, dnf_nnf(item)?)?;
            }
            Some(result)
        }
        Formula::Or(items) => {
            let mut result = Vec::new();
            for item in items {
                result.extend(dnf_nnf(item)?);
                if result.len() > MAX_BRANCHES {
                    return None;
                }
            }
            Some(result)
        }
        Formula::Not(_) => None,
    }
}

fn combine(left: Vec<Vec<Atom>>, right: Vec<Vec<Atom>>) -> Option<Vec<Vec<Atom>>> {
    let mut result = Vec::new();
    for first in &left {
        for second in &right {
            if result.len() >= MAX_BRANCHES {
                return None;
            }
            let mut branch = first.clone();
            branch.extend(second.iter().cloned());
            result.push(branch);
        }
    }
    Some(result)
}

fn meet_sign(left: Sign, right: Sign) -> Option<Sign> {
    if left == right {
        return Some(left);
    }
    match (left, right) {
        (Sign::Positive, Sign::NonNegative) | (Sign::NonNegative, Sign::Positive) => {
            Some(Sign::NonNegative)
        }
        (Sign::Negative, Sign::NonPositive) | (Sign::NonPositive, Sign::Negative) => {
            Some(Sign::NonPositive)
        }
        (Sign::Zero, Sign::NonNegative) | (Sign::NonNegative, Sign::Zero) => {
            Some(Sign::NonNegative)
        }
        (Sign::Zero, Sign::NonPositive) | (Sign::NonPositive, Sign::Zero) => {
            Some(Sign::NonPositive)
        }
        _ => None,
    }
}

struct Branch {
    terms: Vec<Term>,
    parent: Vec<usize>,
    order: Vec<(usize, usize, bool)>,
    unequal: Vec<(usize, usize)>,
    domains: Vec<u8>,
    excluded: Vec<u8>,
    parity: Vec<u8>,
    parity_excluded: Vec<u8>,
    finalized: bool,
}

impl Branch {
    fn from_atoms(atoms: &[Atom]) -> Self {
        let mut branch = Branch {
            terms: Vec::new(),
            parent: Vec::new(),
            order: Vec::new(),
            unequal: Vec::new(),
            domains: Vec::new(),
            excluded: Vec::new(),
            parity: Vec::new(),
            parity_excluded: Vec::new(),
            finalized: false,
        };
        for atom in atoms {
            branch.add(atom);
        }
        branch.finish();
        branch
    }

    fn add(&mut self, atom: &Atom) {
        match atom {
            Atom::Compare { left, op, right } => {
                let left = self.intern(left.clone());
                let right = self.intern(right.clone());
                match op {
                    CompareOp::Equal => self.union(left, right),
                    CompareOp::NotEqual => self.unequal.push((left, right)),
                    CompareOp::Greater => self.order.push((left, right, true)),
                    CompareOp::GreaterEqual => self.order.push((left, right, false)),
                    CompareOp::Less => self.order.push((right, left, true)),
                    CompareOp::LessEqual => self.order.push((right, left, false)),
                }
            }
            Atom::Domain {
                symbol,
                domain,
                positive,
            } => {
                let index = self.intern(Term::Symbol(*symbol));
                if *positive {
                    self.domains[index] |= domain_bit(*domain);
                } else {
                    self.excluded[index] |= domain_bit(*domain);
                }
            }
            Atom::Parity {
                symbol,
                parity,
                positive,
            } => {
                let index = self.intern(Term::Symbol(*symbol));
                let bit = match parity {
                    Parity::Even => PARITY_EVEN,
                    Parity::Odd => PARITY_ODD,
                };
                if *positive {
                    self.parity[index] |= bit;
                    self.domains[index] |= DOMAIN_INTEGER;
                } else {
                    self.parity_excluded[index] |= bit;
                }
            }
        }
    }

    fn finish(&mut self) {
        if self.finalized {
            return;
        }
        self.finalized = true;
        let zero = self.intern(Term::Constant(Rat::zero()));
        let constants: Vec<(usize, Rat)> = self
            .terms
            .iter()
            .enumerate()
            .filter_map(|(index, term)| match term {
                Term::Constant(value) => Some((index, value.clone())),
                Term::Symbol(_) => None,
            })
            .collect();
        for (index, value) in constants {
            match value.cmp(&Rat::zero()) {
                core::cmp::Ordering::Greater => self.order.push((index, zero, true)),
                core::cmp::Ordering::Less => self.order.push((zero, index, true)),
                core::cmp::Ordering::Equal => self.union(index, zero),
            }
        }
    }

    fn intern(&mut self, term: Term) -> usize {
        if let Some(index) = self
            .terms
            .iter()
            .position(|candidate| same_term(candidate, &term))
        {
            return index;
        }
        let index = self.terms.len();
        self.terms.push(term);
        self.parent.push(index);
        self.domains.push(0);
        self.excluded.push(0);
        self.parity.push(0);
        self.parity_excluded.push(0);
        index
    }

    fn root(&self, mut index: usize) -> usize {
        while self.parent[index] != index {
            index = self.parent[index];
        }
        index
    }

    fn union(&mut self, left: usize, right: usize) {
        let left = self.root(left);
        let right = self.root(right);
        if left != right {
            self.parent[right] = left;
        }
    }

    fn is_consistent(&mut self) -> bool {
        self.finish();
        let count = self.terms.len();
        let roots: Vec<usize> = (0..count).map(|index| self.root(index)).collect();

        let mut constants: Vec<(usize, Rat)> = Vec::new();
        for (index, root) in roots.iter().enumerate() {
            if let Term::Constant(value) = &self.terms[index] {
                if let Some((_, previous)) =
                    constants.iter().find(|(candidate, _)| candidate == root)
                {
                    if value.cmp(previous) != core::cmp::Ordering::Equal {
                        return false;
                    }
                } else {
                    constants.push((*root, value.clone()));
                }
            }
        }

        for (left, right) in &self.unequal {
            if roots[*left] == roots[*right] {
                return false;
            }
        }

        let mut domains = vec![0u8; count];
        let mut excluded = vec![0u8; count];
        let mut parity = vec![0u8; count];
        let mut parity_excluded = vec![0u8; count];
        for (index, root) in roots.iter().enumerate() {
            domains[*root] |= self.domains[index];
            excluded[*root] |= self.excluded[index];
            parity[*root] |= self.parity[index];
            parity_excluded[*root] |= self.parity_excluded[index];
        }
        for root in 0..count {
            if domains[root] & excluded[root] != 0 {
                return false;
            }
            if parity[root] == (PARITY_EVEN | PARITY_ODD) {
                return false;
            }
            if parity[root] & parity_excluded[root] != 0 {
                return false;
            }
        }

        let mut adjacency = vec![Vec::new(); count];
        for (left, right, strict) in &self.order {
            let left = roots[*left];
            let right = roots[*right];
            if left == right {
                if *strict {
                    return false;
                }
                continue;
            }
            adjacency[left].push((right, *strict));
        }
        for (left, right, strict) in &self.order {
            let left = roots[*left];
            let right = roots[*right];
            if left == right {
                continue;
            }
            if let Some(path_strict) = path(&adjacency, right, left)
                && (*strict || path_strict)
            {
                return false;
            }
        }
        true
    }

    fn sign(&self, symbol: SymbolId) -> Option<Sign> {
        let index = self
            .terms
            .iter()
            .position(|term| matches!(term, Term::Symbol(candidate) if *candidate == symbol))?;
        let root = self.root(index);
        if let Some((_, value)) = self
            .terms
            .iter()
            .enumerate()
            .find_map(|(index, term)| match term {
                Term::Constant(value) if self.root(index) == root => Some((index, value)),
                _ => None,
            })
        {
            return Some(match value.cmp(&Rat::zero()) {
                core::cmp::Ordering::Greater => Sign::Positive,
                core::cmp::Ordering::Less => Sign::Negative,
                core::cmp::Ordering::Equal => Sign::Zero,
            });
        }
        let zero = self
            .terms
            .iter()
            .position(|term| matches!(term, Term::Constant(value) if value.is_zero()))?;
        let zero = self.root(zero);
        if root == zero {
            return Some(Sign::Zero);
        }
        let to_zero = path(&self.adjacency(), root, zero);
        let from_zero = path(&self.adjacency(), zero, root);
        match (to_zero, from_zero) {
            (Some(true), _) => Some(Sign::Positive),
            (_, Some(true)) => Some(Sign::Negative),
            (Some(false), Some(false)) => Some(Sign::Zero),
            (Some(false), None) => Some(Sign::NonNegative),
            (None, Some(false)) => Some(Sign::NonPositive),
            _ => None,
        }
    }

    fn adjacency(&self) -> Vec<Vec<(usize, bool)>> {
        let count = self.terms.len();
        let mut adjacency = vec![Vec::new(); count];
        for (left, right, strict) in &self.order {
            let left = self.root(*left);
            let right = self.root(*right);
            if left != right {
                adjacency[left].push((right, *strict));
            }
        }
        adjacency
    }
}

fn same_term(left: &Term, right: &Term) -> bool {
    match (left, right) {
        (Term::Symbol(left), Term::Symbol(right)) => left == right,
        (Term::Constant(left), Term::Constant(right)) => {
            left.cmp(right) == core::cmp::Ordering::Equal
        }
        _ => false,
    }
}

fn domain_bit(domain: Domain) -> u8 {
    match domain {
        Domain::Integer => DOMAIN_INTEGER,
        Domain::Rational => DOMAIN_RATIONAL,
        Domain::Real => DOMAIN_REAL,
        Domain::Complex => DOMAIN_COMPLEX,
    }
}

fn path(adjacency: &[Vec<(usize, bool)>], start: usize, target: usize) -> Option<bool> {
    let count = adjacency.len();
    let mut seen = vec![false; count * 2];
    let mut queue = Vec::new();
    queue.push((start, false));
    seen[start * 2] = true;
    let mut result = None;
    let mut position = 0;
    while position < queue.len() {
        let (node, strict) = queue[position];
        position += 1;
        if node == target {
            result = Some(result.unwrap_or(false) || strict);
        }
        for (next, edge_strict) in &adjacency[node] {
            let next_strict = strict || *edge_strict;
            let slot = next * 2 + usize::from(next_strict);
            if !seen[slot] {
                seen[slot] = true;
                queue.push((*next, next_strict));
            }
        }
    }
    result
}
