use cas::expr::{Arena, Attributes, Node};
use cas::num::{Int, Rat};

fn symbol(arena: &mut Arena, name: &str) -> u32 {
    let id = arena.intern_symbol(name, Attributes::new());
    arena.symbol_expr(id)
}

fn arithmetic_head(arena: &mut Arena, name: &str) -> u32 {
    let id = arena.intern_symbol(name, Attributes::arithmetic());
    arena.symbol_expr(id)
}

#[test]
fn commutative_orderless_args_are_one_node() {
    let mut arena = Arena::new();
    let plus = arithmetic_head(&mut arena, "plus");
    let x = symbol(&mut arena, "x");
    let y = symbol(&mut arena, "y");
    let left = arena.intern_call(plus, &[x, y]);
    let right = arena.intern_call(plus, &[y, x]);
    assert_eq!(left, right);
}

#[test]
fn associative_flat_args_are_one_node() {
    let mut arena = Arena::new();
    let plus = arithmetic_head(&mut arena, "plus");
    let x = symbol(&mut arena, "x");
    let y = symbol(&mut arena, "y");
    let z = symbol(&mut arena, "z");
    let xy = arena.intern_call(plus, &[x, y]);
    let left = arena.intern_call(plus, &[xy, z]);
    let yz = arena.intern_call(plus, &[y, z]);
    let right = arena.intern_call(plus, &[x, yz]);
    assert_eq!(left, right);
    match arena.node(left) {
        Node::Call { args, .. } => assert_eq!(args.len(), 3),
        _ => panic!("expected a call"),
    }
}

#[test]
fn plain_head_keeps_argument_order() {
    let mut arena = Arena::new();
    let f = arena.intern_symbol("f", Attributes::new());
    let f = arena.symbol_expr(f);
    let x = symbol(&mut arena, "x");
    let y = symbol(&mut arena, "y");
    let left = arena.intern_call(f, &[x, y]);
    let right = arena.intern_call(f, &[y, x]);
    assert_ne!(left, right);
}

#[test]
fn rationals_are_canonical_and_integers_collapse() {
    let mut arena = Arena::new();
    let half = arena.intern_rat(Rat::from_i64(1, 2).unwrap());
    let two_quarters = arena.intern_rat(Rat::from_i64(2, 4).unwrap());
    assert_eq!(half, two_quarters);

    let two = arena.intern_int(Int::from_i64(2));
    let two_over_one = arena.intern_rat(Rat::from_i64(2, 1).unwrap());
    assert_eq!(two, two_over_one);
}

#[test]
fn symbols_are_unique_by_name() {
    let mut arena = Arena::new();
    let first = arena.intern_symbol("x", Attributes::new());
    let second = arena.intern_symbol("x", Attributes::new());
    assert_eq!(first, second);
    assert_eq!(arena.symbol_name(first), "x");
}

#[test]
fn canonical_order_places_numbers_before_symbols() {
    let mut arena = Arena::new();
    let number = arena.intern_int(Int::from_i64(1));
    let symbol_node = symbol(&mut arena, "x");
    assert!(arena.canonical_cmp(number, symbol_node).is_lt());
}
