//! Canonical order over expressions.

use core::cmp::Ordering;

use super::arena::Arena;
use super::node::{ExprId, Node};

fn rank(node: &Node) -> u8 {
    match node {
        Node::Int(_) => 0,
        Node::Rat(_) => 1,
        Node::Symbol(_) => 2,
        Node::Call { .. } => 3,
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
        (Node::Symbol(a), Node::Symbol(b)) => arena.symbol_name(*a).cmp(arena.symbol_name(*b)),
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
        _ => Ordering::Equal,
    }
}
