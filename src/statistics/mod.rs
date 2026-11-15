//! Exact descriptive statistics over rational lists.

use crate::expr::{Arena, ExprId, Node};
use crate::num::{Int, Rat};

fn list_values(arena: &Arena, expression: ExprId) -> Option<&[ExprId]> {
    match arena.node(expression) {
        Node::Call { head, args } => {
            let symbol = arena.symbol_id(*head)?;
            if arena.symbol_name(symbol) == "list" {
                Some(args.as_ref())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn as_rat(arena: &Arena, expression: ExprId) -> Option<Rat> {
    match arena.node(expression) {
        Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
        Node::Rat(value) => Some(value.clone()),
        _ => None,
    }
}

fn numbers(arena: &Arena, expression: ExprId) -> Option<Vec<Rat>> {
    let values = list_values(arena, expression)?;
    if values.is_empty() {
        return None;
    }
    values.iter().map(|value| as_rat(arena, *value)).collect()
}

fn all_integers(arena: &Arena, values: &[ExprId]) -> bool {
    values
        .iter()
        .all(|value| matches!(arena.node(*value), Node::Int(_)))
}

fn integer_sum(arena: &Arena, values: &[ExprId]) -> Option<Int> {
    if let Some(total) = integer_sum_fast(arena, values) {
        return Some(total);
    }
    let mut total = Int::zero();
    for value in values {
        let Node::Int(value) = arena.node(*value) else {
            return None;
        };
        total = total.add(value);
    }
    Some(total)
}

fn integer_sum_fast(arena: &Arena, values: &[ExprId]) -> Option<Int> {
    let mut total = 0i128;
    for value in values {
        let Node::Int(value) = arena.node(*value) else {
            return None;
        };
        total = total.checked_add(value.to_i128()?)?;
    }
    i64::try_from(total).ok().map(Int::from_i64)
}

fn integer_variance_fast(arena: &Arena, values: &[ExprId], sample: bool) -> Option<Rat> {
    let mut sum = 0i128;
    let mut squares = 0i128;
    for value in values {
        let Node::Int(value) = arena.node(*value) else {
            return None;
        };
        let value = value.to_i128()?;
        sum = sum.checked_add(value)?;
        squares = squares.checked_add(value.checked_mul(value)?)?;
    }
    let count = values.len() as i128;
    let numerator = count
        .checked_mul(squares)?
        .checked_sub(sum.checked_mul(sum)?)?;
    let denominator = if sample {
        count.checked_mul(count.checked_sub(1)?)?
    } else {
        count.checked_mul(count)?
    };
    Rat::from_i64(
        i64::try_from(numerator).ok()?,
        i64::try_from(denominator).ok()?,
    )
    .ok()
}

fn rational_expression(arena: &mut Arena, value: &Rat) -> ExprId {
    if value.den().is_one() {
        arena.intern_int(value.num())
    } else {
        arena.intern_rat(value.clone())
    }
}

fn sorted(values: &[Rat]) -> Vec<Rat> {
    let mut values = values.to_vec();
    values.sort_by(Rat::cmp);
    values
}

fn square_root(value: &Rat) -> Option<Rat> {
    let numerator = value.num().to_i64()?;
    let denominator = value.den().to_i64()?;
    let numerator = integer_sqrt(numerator)?;
    let denominator = integer_sqrt(denominator)?;
    Rat::from_i64(numerator, denominator).ok()
}

fn integer_sqrt(value: i64) -> Option<i64> {
    if value < 0 {
        return None;
    }
    let mut low = 0i64;
    let mut high = value;
    while low <= high {
        let middle = low + (high - low) / 2;
        let square = (middle as i128) * (middle as i128);
        if square == value as i128 {
            return Some(middle);
        }
        if square < value as i128 {
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    None
}

fn deviation_expression(arena: &mut Arena, variance: &Rat) -> ExprId {
    if let Some(root) = square_root(variance) {
        return rational_expression(arena, &root);
    }
    let value = rational_expression(arena, variance);
    let head = arena.builtin_head("sqrt");
    arena.intern_call(head, &[value])
}

pub(crate) fn mean_stat(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let values = list_values(arena, expression)?;
    if values.is_empty() {
        return None;
    }
    if let Some(total) = integer_sum_fast(arena, values) {
        let count = Int::from_i64(values.len() as i64);
        let value = Rat::from_ints(total, count).ok()?;
        return Some(rational_expression(arena, &value));
    }
    if all_integers(arena, values) {
        let total = integer_sum(arena, values)?;
        let count = Int::from_i64(values.len() as i64);
        let value = Rat::from_ints(total, count).ok()?;
        return Some(rational_expression(arena, &value));
    }
    let mut total = Rat::zero();
    for value in values {
        total = total.add(&as_rat(arena, *value)?);
    }
    let count = Rat::from_i64(values.len() as i64, 1).ok()?;
    Some(rational_expression(arena, &total.div(&count).ok()?))
}

pub(crate) fn median(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let values = sorted(&numbers(arena, expression)?);
    let middle = values.len() / 2;
    let value = if values.len() % 2 == 1 {
        values[middle].clone()
    } else {
        values[middle - 1]
            .add(&values[middle])
            .div(&Rat::from_i64(2, 1).ok()?)
            .ok()?
    };
    Some(rational_expression(arena, &value))
}

pub(crate) fn minimum(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let values = numbers(arena, expression)?;
    let value = values.into_iter().reduce(|left, right| {
        if left.cmp(&right) == core::cmp::Ordering::Less {
            left
        } else {
            right
        }
    })?;
    Some(rational_expression(arena, &value))
}

pub(crate) fn maximum(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let values = numbers(arena, expression)?;
    let value = values.into_iter().reduce(|left, right| {
        if left.cmp(&right) == core::cmp::Ordering::Greater {
            left
        } else {
            right
        }
    })?;
    Some(rational_expression(arena, &value))
}

pub(crate) fn range(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let values = numbers(arena, expression)?;
    let minimum = values.iter().cloned().reduce(|left, right| {
        if left.cmp(&right) == core::cmp::Ordering::Less {
            left
        } else {
            right
        }
    })?;
    let maximum = values.iter().cloned().reduce(|left, right| {
        if left.cmp(&right) == core::cmp::Ordering::Greater {
            left
        } else {
            right
        }
    })?;
    Some(rational_expression(arena, &maximum.sub(&minimum)))
}

pub(crate) fn total(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let values = list_values(arena, expression)?;
    if values.is_empty() {
        return None;
    }
    if let Some(total) = integer_sum_fast(arena, values) {
        return Some(arena.intern_int(total));
    }
    if all_integers(arena, values) {
        return Some(arena.intern_int(integer_sum(arena, values)?));
    }
    let mut total = Rat::zero();
    for value in values {
        total = total.add(&as_rat(arena, *value)?);
    }
    Some(rational_expression(arena, &total))
}

pub(crate) fn count(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let values = list_values(arena, expression)?;
    if values.is_empty() {
        return None;
    }
    for value in values {
        as_rat(arena, *value)?;
    }
    Some(arena.intern_int(Int::from_i64(values.len() as i64)))
}

pub(crate) fn variance_stat(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    Some(rational_expression(
        arena,
        &direct_variance(arena, expression, false)?,
    ))
}

pub(crate) fn sample_variance(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    Some(rational_expression(
        arena,
        &direct_variance(arena, expression, true)?,
    ))
}

fn direct_variance(arena: &Arena, expression: ExprId, sample: bool) -> Option<Rat> {
    let values = list_values(arena, expression)?;
    let count = values.len();
    if count == 0 || (sample && count < 2) {
        return None;
    }
    if let Some(variance) = integer_variance_fast(arena, values, sample) {
        return Some(variance);
    }
    if all_integers(arena, values) {
        let mut sum = Int::zero();
        let mut squares = Int::zero();
        for value in values {
            let Node::Int(value) = arena.node(*value) else {
                return None;
            };
            sum = sum.add(value);
            squares = squares.add(&value.mul(value));
        }
        let count = Int::from_i64(count as i64);
        let numerator = count.mul(&squares).sub(&sum.mul(&sum));
        let denominator = if sample {
            count.mul(&Int::from_i64(count.to_i64()? - 1))
        } else {
            count.mul(&count)
        };
        return Rat::from_ints(numerator, denominator).ok();
    }
    let mut sum = Rat::zero();
    let mut squares = Rat::zero();
    for value in values {
        let value = as_rat(arena, *value)?;
        sum = sum.add(&value);
        squares = squares.add(&value.mul(&value));
    }
    let count_rat = Rat::from_i64(count as i64, 1).ok()?;
    let numerator = count_rat.mul(&squares).sub(&sum.mul(&sum));
    let denominator = if sample {
        count_rat.mul(&Rat::from_i64(count as i64 - 1, 1).ok()?)
    } else {
        count_rat.mul(&count_rat)
    };
    numerator.div(&denominator).ok()
}

pub(crate) fn standard_deviation(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let variance = direct_variance(arena, expression, false)?;
    Some(deviation_expression(arena, &variance))
}

pub(crate) fn sample_standard_deviation(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let variance = direct_variance(arena, expression, true)?;
    Some(deviation_expression(arena, &variance))
}

pub(crate) fn frequency(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let values = sorted(&numbers(arena, expression)?);
    let mut pairs = Vec::new();
    let mut index = 0usize;
    while index < values.len() {
        let value = values[index].clone();
        let mut count = 1usize;
        while index + count < values.len() && values[index + count] == value {
            count += 1;
        }
        let value = rational_expression(arena, &value);
        let count_expression = arena.intern_int(Int::from_i64(count as i64));
        let list = arena.builtin_head("list");
        pairs.push(arena.intern_call(list, &[value, count_expression]));
        index += count;
    }
    let list = arena.builtin_head("list");
    Some(arena.intern_call(list, &pairs))
}

pub(crate) fn mode(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let values = sorted(&numbers(arena, expression)?);
    let mut best = 0usize;
    let mut modes = Vec::new();
    let mut index = 0usize;
    while index < values.len() {
        let value = values[index].clone();
        let mut count = 1usize;
        while index + count < values.len() && values[index + count] == value {
            count += 1;
        }
        if count > best {
            best = count;
            modes.clear();
            modes.push(value);
        } else if count == best {
            modes.push(value);
        }
        index += count;
    }
    let list = arena.builtin_head("list");
    let entries = modes
        .iter()
        .map(|value| rational_expression(arena, value))
        .collect::<Vec<_>>();
    Some(arena.intern_call(list, &entries))
}
