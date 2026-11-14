use core::cmp::Ordering;

use mithqal::expr::{Arena, Attributes, Node};
use mithqal::num::{Int, Rat};

fn symbol(arena: &mut Arena, name: &str) -> u32 {
    let id = arena.symbol_or_intern(name);
    arena.symbol_expr(id)
}

fn arithmetic(arena: &mut Arena, name: &str) -> u32 {
    let id = arena.intern_symbol(name, Attributes::arithmetic());
    arena.symbol_expr(id)
}

#[test]
fn empty_arena() {
    let arena = Arena::new();
    assert!(arena.is_empty());
    assert_eq!(arena.len(), 0);
}

#[test]
fn interning_numbers_is_idempotent() {
    let mut arena = Arena::new();
    let a = arena.intern_int(Int::from_i64(42));
    let b = arena.intern_int(Int::from_i64(42));
    assert_eq!(a, b);
    assert_eq!(arena.len(), 1);
}

#[test]
fn zero_denominator_is_never_interned() {
    let mut arena = Arena::new();
    let value = Rat::from_i64(7, 1).unwrap();
    let id = arena.intern_rat(value);
    assert!(matches!(arena.node(id), Node::Int(_)));
}

#[test]
fn empty_calls_intern() {
    let mut arena = Arena::new();
    let f = symbol(&mut arena, "f");
    let first = arena.intern_call(f, &[]);
    let second = arena.intern_call(f, &[]);
    assert_eq!(first, second);
    match arena.node(first) {
        Node::Call { args, .. } => assert!(args.is_empty()),
        _ => panic!("expected a call"),
    }
}

#[test]
fn deep_flattening() {
    let mut arena = Arena::new();
    let plus = arithmetic(&mut arena, "plus");
    let a = symbol(&mut arena, "a");
    let b = symbol(&mut arena, "b");
    let c = symbol(&mut arena, "c");
    let d = symbol(&mut arena, "d");
    let ab = arena.intern_call(plus, &[a, b]);
    let abc = arena.intern_call(plus, &[ab, c]);
    let all = arena.intern_call(plus, &[abc, d]);
    match arena.node(all) {
        Node::Call { args, .. } => assert_eq!(args.len(), 4),
        _ => panic!("expected a call"),
    }
}

#[test]
fn orderless_sorting_is_stable_and_total() {
    let mut arena = Arena::new();
    let plus = arithmetic(&mut arena, "plus");
    let c = symbol(&mut arena, "c");
    let a = symbol(&mut arena, "a");
    let b = symbol(&mut arena, "b");
    let first = arena.intern_call(plus, &[c, a, b]);
    let second = arena.intern_call(plus, &[a, b, c]);
    assert_eq!(first, second);
    match arena.node(first) {
        Node::Call { args, .. } => {
            let names: Vec<&str> = args
                .iter()
                .map(|id| {
                    let symbol = arena.symbol_id(*id).unwrap();
                    arena.symbol_name(symbol)
                })
                .collect();
            assert_eq!(names, vec!["a", "b", "c"]);
        }
        _ => panic!("expected a call"),
    }
}

#[test]
fn raw_calls_skip_flattening() {
    let mut arena = Arena::new();
    let plus = arithmetic(&mut arena, "plus");
    let a = symbol(&mut arena, "a");
    let b = symbol(&mut arena, "b");
    let ab = arena.intern_call(plus, &[a, b]);
    let raw = arena.intern_call_raw(plus, &[ab, a]);
    match arena.node(raw) {
        Node::Call { args, .. } => assert_eq!(args.len(), 2),
        _ => panic!("expected a call"),
    }
}

#[test]
fn attributes_are_not_clobbered_by_plain_lookup() {
    let mut arena = Arena::new();
    let id = arena.intern_symbol(
        "protectedName",
        Attributes::new().with(Attributes::PROTECTED),
    );
    let same = arena.symbol_or_intern("protectedName");
    assert_eq!(id, same);
    assert!(arena.symbol_attributes(id).has(Attributes::PROTECTED));
}

#[test]
fn attributes_can_be_replaced_explicitly() {
    let mut arena = Arena::new();
    let id = arena.intern_symbol("x", Attributes::new());
    arena.set_symbol_attributes(id, Attributes::arithmetic());
    assert!(arena.symbol_attributes(id).has(Attributes::FLAT));
    assert!(arena.symbol_attributes(id).has(Attributes::ORDERLESS));
}

#[test]
fn head_attributes_only_apply_to_symbols() {
    let mut arena = Arena::new();
    let f = symbol(&mut arena, "f");
    let call = arena.intern_call(f, &[]);
    assert_eq!(arena.head_attributes(call), Attributes::new());
    assert_eq!(arena.head_attributes(f), Attributes::new());
}

#[test]
fn non_symbol_head_has_no_symbol_id() {
    let mut arena = Arena::new();
    let f = symbol(&mut arena, "f");
    let call = arena.intern_call(f, &[]);
    assert_eq!(arena.symbol_id(call), None);
}

#[test]
fn canonical_order_is_reflexive() {
    let mut arena = Arena::new();
    let x = symbol(&mut arena, "x");
    let one = arena.intern_int(Int::one());
    let call = arena.intern_call(x, &[one]);
    for id in [x, one, call] {
        assert_eq!(arena.canonical_cmp(id, id), Ordering::Equal);
    }
}

#[test]
fn canonical_order_orders_calls_by_head_then_args() {
    let mut arena = Arena::new();
    let f = symbol(&mut arena, "f");
    let g = symbol(&mut arena, "g");
    let x = symbol(&mut arena, "x");
    let y = symbol(&mut arena, "y");
    let fx = arena.intern_call(f, &[x]);
    let fy = arena.intern_call(f, &[y]);
    let gx = arena.intern_call(g, &[x]);
    assert_eq!(arena.canonical_cmp(fx, fy), Ordering::Less);
    assert_eq!(arena.canonical_cmp(fy, gx), Ordering::Less);
}

#[test]
fn canonical_order_orders_shorter_argument_lists_first() {
    let mut arena = Arena::new();
    let f = symbol(&mut arena, "f");
    let x = symbol(&mut arena, "x");
    let one = arena.intern_call(f, &[x]);
    let two = arena.intern_call(f, &[x, x]);
    assert_eq!(arena.canonical_cmp(one, two), Ordering::Less);
}

#[test]
fn symbols_sort_by_name() {
    let mut arena = Arena::new();
    let a = symbol(&mut arena, "a");
    let b = symbol(&mut arena, "b");
    assert_eq!(arena.canonical_cmp(a, b), Ordering::Less);
    assert_eq!(arena.canonical_cmp(b, a), Ordering::Greater);
}

#[test]
fn integer_nodes_sort_before_rational_nodes() {
    let mut arena = Arena::new();
    let integer = arena.intern_int(Int::from_i64(100));
    let rational = arena.intern_rat(Rat::from_i64(1, 100).unwrap());
    assert_eq!(arena.canonical_cmp(integer, rational), Ordering::Less);
}

#[test]
fn many_duplicate_calls_interning() {
    let mut arena = Arena::new();
    let f = symbol(&mut arena, "f");
    let x = symbol(&mut arena, "x");
    let first = arena.intern_call(f, &[x]);
    for _ in 0..1000 {
        assert_eq!(arena.intern_call(f, &[x]), first);
    }
    assert_eq!(arena.len(), 3);
}
