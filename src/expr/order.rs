//! Canonical order over expressions.

#[allow(unused_imports)]
use crate::prelude::*;

use core::cmp::Ordering;

use super::arena::Arena;
use super::node::{ExprId, Node};

fn rank(node: &Node) -> u8 {
    match node {
        Node::Int(_) => 0,
        Node::Rat(_) => 1,
        Node::Real(_) => 2,
        Node::String(_) => 3,
        Node::Bytes(_) => 4,
        Node::Symbol(_) => 5,
        Node::Call { .. } => 6,
        Node::Association(_) => 7,
    }
}

/// Compare two expressions in canonical order.
pub(crate) fn canonical_cmp(arena: &Arena, left: ExprId, right: ExprId) -> Ordering {
    if left == right {
        return Ordering::Equal;
    }
    let left_node = arena.node(left);
    let right_node = arena.node(right);
    let left_rank = rank(left_node);
    let right_rank = rank(right_node);
    if left_rank != right_rank {
        return left_rank.cmp(&right_rank);
    }
    match (left_node, right_node) {
        (Node::Int(a), Node::Int(b)) => a.cmp(b),
        (Node::Rat(a), Node::Rat(b)) => a.cmp(b),
        (Node::Real(a), Node::Real(b)) => a.cmp(b),
        (Node::String(a), Node::String(b)) => a.cmp(b),
        (Node::Bytes(a), Node::Bytes(b)) => a.cmp(b),
        (Node::Symbol(a), Node::Symbol(b)) => arena
            .symbol_context(*a)
            .cmp(arena.symbol_context(*b))
            .then_with(|| arena.symbol_name(*a).cmp(arena.symbol_name(*b))),
        (
            Node::Call {
                head: left_head,
                args: left_args,
            },
            Node::Call {
                head: right_head,
                args: right_args,
            },
        ) => {
            let head_order = canonical_cmp(arena, *left_head, *right_head);
            if head_order != Ordering::Equal {
                return head_order;
            }
            match left_args.len().cmp(&right_args.len()) {
                Ordering::Equal => {
                    for (left_arg, right_arg) in left_args.iter().zip(right_args.iter()) {
                        let order = canonical_cmp(arena, *left_arg, *right_arg);
                        if order != Ordering::Equal {
                            return order;
                        }
                    }
                    Ordering::Equal
                }
                other => other,
            }
        }
        (Node::Association(left), Node::Association(right)) => match left.len().cmp(&right.len()) {
            Ordering::Equal => {
                for (left_entry, right_entry) in left.iter().zip(right.iter()) {
                    let order = canonical_cmp(arena, left_entry.key, right_entry.key);
                    if order != Ordering::Equal {
                        return order;
                    }
                    let order = canonical_cmp(arena, left_entry.value, right_entry.value);
                    if order != Ordering::Equal {
                        return order;
                    }
                }
                Ordering::Equal
            }
            other => other,
        },
        _ => Ordering::Equal,
    }
}
