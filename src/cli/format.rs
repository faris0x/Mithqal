//! Human-readable CLI output formatting.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::{ExprId, ExprView, Session};

pub(super) fn value(session: &Session, source: &str, value: ExprId) -> String {
    if source.trim_start().starts_with("factorInteger(")
        && let Some(text) = prime_product(session, value)
    {
        return text;
    }
    session
        .format_infix(value)
        .unwrap_or_else(|_| "?".to_string())
}

fn prime_product(session: &Session, value: ExprId) -> Option<String> {
    let factors = list_items(session, value)?;
    if factors.is_empty() {
        return Some("1".to_string());
    }
    let mut negative = false;
    let mut parts = Vec::new();
    for factor in factors {
        let pair = list_items(session, factor)?;
        if pair.len() != 2 {
            return None;
        }
        let base = integer(session, pair[0])?;
        let exponent = integer(session, pair[1])?;
        if exponent == 0 {
            continue;
        }
        if base == -1 && exponent == 1 {
            negative = true;
            continue;
        }
        if exponent == 1 {
            parts.push(base.to_string());
        } else if base < 0 {
            parts.push(format!("({base})^{exponent}"));
        } else {
            parts.push(format!("{base}^{exponent}"));
        }
    }
    if parts.is_empty() {
        return Some(if negative {
            "-1".to_string()
        } else {
            "1".to_string()
        });
    }
    let body = parts.join("*");
    Some(if negative { format!("-{body}") } else { body })
}

fn list_items(session: &Session, value: ExprId) -> Option<Vec<ExprId>> {
    let ExprView::Call { head, args } = session.view(value).ok()? else {
        return None;
    };
    if !matches!(session.view(head).ok()?, ExprView::Symbol("list")) {
        return None;
    }
    Some(args.to_vec())
}

fn integer(session: &Session, value: ExprId) -> Option<i64> {
    let ExprView::Integer(value) = session.view(value).ok()? else {
        return None;
    };
    value.to_i64()
}
